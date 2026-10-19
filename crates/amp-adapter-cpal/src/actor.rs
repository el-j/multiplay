//! Dedicated background actor thread managing CPAL stream lifecycle.

#![forbid(unsafe_code)]

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{Device, StreamConfig};
use rtrb::Consumer;
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

pub(crate) enum SinkControlMsg {
    Configure {
        sample_rate: u32,
        channels: u16,
        reply: mpsc::Sender<Result<(), String>>,
    },
    Start {
        reply: mpsc::Sender<Result<(), String>>,
    },
    Pause {
        reply: mpsc::Sender<Result<(), String>>,
    },
    Stop {
        reply: mpsc::Sender<Result<(), String>>,
    },
    Terminate,
}

/// Fills a device buffer of `device_ch` interleaved channels from a queue holding
/// `source_ch`-channel frames. Stereo sources play on the first two channels (other channels
/// stay silent) and are averaged on a mono device; a source matching the device passes through.
/// Missing samples become silence. Real-time safe.
pub(crate) fn fill_output(
    output: &mut [f32],
    device_ch: usize,
    source_ch: usize,
    mut pop: impl FnMut() -> Option<f32>,
) {
    let (device_ch, source_ch) = (device_ch.max(1), source_ch.max(1));
    if device_ch == source_ch {
        for s in output.iter_mut() {
            *s = pop().unwrap_or(0.0);
        }
        return;
    }
    for frame in output.chunks_mut(device_ch) {
        frame.fill(0.0);
        let mut sum = 0.0;
        for c in 0..source_ch {
            let v = pop().unwrap_or(0.0);
            sum += v;
            if let Some(slot) = frame.get_mut(c) {
                *slot = v;
            }
        }
        if device_ch == 1 {
            frame[0] = sum / source_ch as f32;
        }
    }
}

pub(crate) fn run_cpal_actor(
    device: Device,
    config: StreamConfig,
    source_channels: u16,
    mut consumer: Consumer<f32>,
    cmd_rx: mpsc::Receiver<SinkControlMsg>,
    current_sr: Arc<AtomicU32>,
    current_ch: Arc<AtomicU16>,
) {
    let err_fn = |err| eprintln!("Audio stream callback error: {err}");
    let device_channels = usize::from(config.channels);
    let source_channels = usize::from(source_channels);

    let stream_res = device.build_output_stream(
        &config,
        move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
            fill_output(output, device_channels, source_channels, || {
                consumer.pop().ok()
            });
        },
        err_fn,
        None,
    );

    let stream = match stream_res {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize audio output stream: {e}");
            while let Ok(msg) = cmd_rx.recv() {
                match msg {
                    SinkControlMsg::Configure { reply, .. } => {
                        let _ = reply.send(Err(e.to_string()));
                    }
                    SinkControlMsg::Start { reply } => {
                        let _ = reply.send(Err(e.to_string()));
                    }
                    SinkControlMsg::Pause { reply } => {
                        let _ = reply.send(Err(e.to_string()));
                    }
                    SinkControlMsg::Stop { reply } => {
                        let _ = reply.send(Err(e.to_string()));
                    }
                    SinkControlMsg::Terminate => break,
                }
            }
            return;
        }
    };

    // The stream is built in paused state. It is controlled via play() and pause()
    while let Ok(msg) = cmd_rx.recv() {
        match msg {
            SinkControlMsg::Configure {
                sample_rate,
                channels,
                reply,
            } => {
                current_sr.store(sample_rate, Ordering::Relaxed);
                current_ch.store(channels, Ordering::Relaxed);
                let _ = reply.send(Ok(()));
            }
            SinkControlMsg::Start { reply } => {
                let res = stream
                    .play()
                    .map_err(|e| format!("Stream play failed: {e}"));
                let _ = reply.send(res);
            }
            SinkControlMsg::Pause { reply } => {
                let res = stream
                    .pause()
                    .map_err(|e| format!("Stream pause failed: {e}"));
                let _ = reply.send(res);
            }
            SinkControlMsg::Stop { reply } => {
                let res = stream
                    .pause()
                    .map_err(|e| format!("Stream stop failed: {e}"));
                let _ = reply.send(res);
            }
            SinkControlMsg::Terminate => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fill_output;

    fn run(device: usize, source: usize, input: &[f32], frames: usize) -> Vec<f32> {
        let mut it = input.iter().copied();
        let mut out = vec![9.0; frames * device];
        fill_output(&mut out, device, source, || it.next());
        out
    }

    #[test]
    fn stereo_plays_on_the_first_two_channels_of_a_4_channel_device() {
        let out = run(4, 2, &[0.1, 0.2, 0.3, 0.4], 2);
        assert_eq!(out, vec![0.1, 0.2, 0.0, 0.0, 0.3, 0.4, 0.0, 0.0]);
    }

    #[test]
    fn stereo_is_averaged_on_a_mono_device() {
        let out = run(1, 2, &[0.2, 0.4, -0.2, 0.0], 2);
        assert!((out[0] - 0.3).abs() < 1e-6 && (out[1] + 0.1).abs() < 1e-6);
    }

    #[test]
    fn matching_layouts_pass_through_and_underruns_are_silent() {
        assert_eq!(run(2, 2, &[0.5, -0.5], 2), vec![0.5, -0.5, 0.0, 0.0]);
        assert_eq!(
            run(4, 4, &[1.0, 2.0, 3.0, 4.0], 1),
            vec![1.0, 2.0, 3.0, 4.0]
        );
    }
}
