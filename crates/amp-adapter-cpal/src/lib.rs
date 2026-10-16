//! Pure-Rust hardware audio sink adapter using CPAL and lock-free SPSC ring buffer.
//! Isolates CPAL stream lifecycle to a dedicated thread and guarantees real-time safety.

use amp_ports::driven::AudioSinkPort;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, Device, SampleRate, Stream, StreamConfig};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::mpsc::{self, Sender};
use std::sync::atomic::{AtomicU32, AtomicU16, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const DEFAULT_RING_BUFFER_FRAMES: usize = 65536;

enum SinkControlMsg {
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

/// Thread-safe (`Send + Sync`) CPAL Audio Sink handle implementing `AudioSinkPort`.
pub struct CpalAudioSink {
    control_tx: Option<Sender<SinkControlMsg>>,
    sample_rate: Arc<AtomicU32>,
    channels: Arc<AtomicU16>,
}

impl CpalAudioSink {
    /// Spawns the audio actor thread, opens the default CPAL device, and returns the sink handle and sample Producer.
    pub fn new() -> Result<(Self, Producer<f32>), String> {
        let (producer, consumer) = RingBuffer::<f32>::new(DEFAULT_RING_BUFFER_FRAMES);

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "No default audio output device found".to_string())?;

        let supported_config = device
            .default_output_config()
            .map_err(|e| format!("Failed to read device output configuration: {e}"))?;

        let initial_rate = supported_config.sample_rate().0;
        let initial_channels = supported_config.channels();

        let sample_rate = Arc::new(AtomicU32::new(initial_rate));
        let channels = Arc::new(AtomicU16::new(initial_channels));

        let (cmd_tx, cmd_rx) = mpsc::channel::<SinkControlMsg>();

        let sr_clone = Arc::clone(&sample_rate);
        let ch_clone = Arc::clone(&channels);

        // Spawn dedicated audio thread that owns the non-Send/Sync CPAL Stream
        thread::Builder::new()
            .name("amp-cpal-actor".to_string())
            .spawn(move || {
                run_cpal_actor(device, consumer, cmd_rx, sr_clone, ch_clone);
            })
            .map_err(|e| format!("Failed to spawn CPAL actor thread: {e}"))?;

        let sink = Self {
            control_tx: Some(cmd_tx),
            sample_rate,
            channels,
        };

        Ok((sink, producer))
    }

    /// Mock constructor for headless CI and unit testing.
    pub fn new_mock(sample_rate: u32, channels: u16) -> (Self, Producer<f32>, Consumer<f32>) {
        let (producer, consumer) = RingBuffer::<f32>::new(DEFAULT_RING_BUFFER_FRAMES);
        let sr = Arc::new(AtomicU32::new(sample_rate));
        let ch = Arc::new(AtomicU16::new(channels));

        let (cmd_tx, cmd_rx) = mpsc::channel::<SinkControlMsg>();
        thread::spawn(move || {
            while let Ok(msg) = cmd_rx.recv() {
                match msg {
                    SinkControlMsg::Configure { reply, .. } => { let _ = reply.send(Ok(())); }
                    SinkControlMsg::Start { reply } => { let _ = reply.send(Ok(())); }
                    SinkControlMsg::Pause { reply } => { let _ = reply.send(Ok(())); }
                    SinkControlMsg::Stop { reply } => { let _ = reply.send(Ok(())); }
                    SinkControlMsg::Terminate => break,
                }
            }
        });

        let sink = Self {
            control_tx: Some(cmd_tx),
            sample_rate: sr,
            channels: ch,
        };

        (sink, producer, consumer)
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate.load(Ordering::Relaxed)
    }

    pub fn channels(&self) -> u16 {
        self.channels.load(Ordering::Relaxed)
    }
}

impl Drop for CpalAudioSink {
    fn drop(&mut self) {
        if let Some(tx) = self.control_tx.take() {
            let _ = tx.send(SinkControlMsg::Terminate);
        }
    }
}

impl AudioSinkPort for CpalAudioSink {
    fn configure(&mut self, sample_rate: u32, channels: u16) -> Result<(), String> {
        self.sample_rate.store(sample_rate, Ordering::Relaxed);
        self.channels.store(channels, Ordering::Relaxed);

        if let Some(tx) = &self.control_tx {
            let (reply_tx, reply_rx) = mpsc::channel();
            tx.send(SinkControlMsg::Configure {
                sample_rate,
                channels,
                reply: reply_tx,
            })
            .map_err(|e| e.to_string())?;

            reply_rx
                .recv_timeout(Duration::from_millis(500))
                .map_err(|e| format!("Configure timeout: {e}"))?
        } else {
            Ok(())
        }
    }

    fn start(&mut self) -> Result<(), String> {
        if let Some(tx) = &self.control_tx {
            let (reply_tx, reply_rx) = mpsc::channel();
            tx.send(SinkControlMsg::Start { reply: reply_tx })
                .map_err(|e| e.to_string())?;

            reply_rx
                .recv_timeout(Duration::from_millis(1000))
                .map_err(|e| format!("Start timeout: {e}"))?
        } else {
            Ok(())
        }
    }

    fn pause(&mut self) -> Result<(), String> {
        if let Some(tx) = &self.control_tx {
            let (reply_tx, reply_rx) = mpsc::channel();
            tx.send(SinkControlMsg::Pause { reply: reply_tx })
                .map_err(|e| e.to_string())?;

            reply_rx
                .recv_timeout(Duration::from_millis(500))
                .map_err(|e| format!("Pause timeout: {e}"))?
        } else {
            Ok(())
        }
    }

    fn stop(&mut self) -> Result<(), String> {
        if let Some(tx) = &self.control_tx {
            let (reply_tx, reply_rx) = mpsc::channel();
            tx.send(SinkControlMsg::Stop { reply: reply_tx })
                .map_err(|e| e.to_string())?;

            reply_rx
                .recv_timeout(Duration::from_millis(500))
                .map_err(|e| format!("Stop timeout: {e}"))?
        } else {
            Ok(())
        }
    }

    fn push_interleaved_samples(&mut self, _samples: &[f32]) -> usize {
        0
    }

    fn get_buffer_latency(&self) -> Duration {
        let ch = self.channels.load(Ordering::Relaxed).max(1) as f64;
        let sr = self.sample_rate.load(Ordering::Relaxed).max(1) as f64;
        let frames = (DEFAULT_RING_BUFFER_FRAMES as f64) / ch;
        Duration::from_secs_f64(frames / sr)
    }
}

fn run_cpal_actor(
    device: Device,
    consumer: Consumer<f32>,
    cmd_rx: mpsc::Receiver<SinkControlMsg>,
    current_sr: Arc<AtomicU32>,
    current_ch: Arc<AtomicU16>,
) {
    let mut consumer_opt = Some(consumer);
    let mut active_stream: Option<Stream> = None;

    while let Ok(msg) = cmd_rx.recv() {
        match msg {
            SinkControlMsg::Configure { sample_rate, channels, reply } => {
                current_sr.store(sample_rate, Ordering::Relaxed);
                current_ch.store(channels, Ordering::Relaxed);
                active_stream = None;
                let _ = reply.send(Ok(()));
            }
            SinkControlMsg::Start { reply } => {
                if active_stream.is_some() {
                    let _ = reply.send(Ok(()));
                    continue;
                }

                let mut consumer = match consumer_opt.take() {
                    Some(c) => c,
                    None => {
                        let _ = reply.send(Err("Ring buffer consumer already bound".to_string()));
                        continue;
                    }
                };

                let sr = current_sr.load(Ordering::Relaxed);
                let ch = current_ch.load(Ordering::Relaxed);
                let config = StreamConfig {
                    channels: ch,
                    sample_rate: SampleRate(sr),
                    buffer_size: BufferSize::Default,
                };

                let err_fn = |err| eprintln!("Audio stream callback error: {err}");

                let stream_res = device.build_output_stream(
                    &config,
                    move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        for sample in output.iter_mut() {
                            *sample = consumer.pop().unwrap_or(0.0);
                        }
                    },
                    err_fn,
                    None,
                );

                match stream_res {
                    Ok(stream) => {
                        if let Err(e) = stream.play() {
                            let _ = reply.send(Err(format!("Stream play failure: {e}")));
                        } else {
                            active_stream = Some(stream);
                            let _ = reply.send(Ok(()));
                        }
                    }
                    Err(e) => {
                        let _ = reply.send(Err(format!("Stream build failure: {e}")));
                    }
                }
            }
            SinkControlMsg::Pause { reply } => {
                if let Some(stream) = &active_stream {
                    let _ = stream.pause();
                }
                let _ = reply.send(Ok(()));
            }
            SinkControlMsg::Stop { reply } => {
                active_stream = None;
                let _ = reply.send(Ok(()));
            }
            SinkControlMsg::Terminate => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpal_audio_sink_mock() {
        let (mut sink, mut producer, mut consumer) = CpalAudioSink::new_mock(48000, 2);
        assert_eq!(sink.sample_rate(), 48000);
        assert_eq!(sink.channels(), 2);

        sink.start().expect("mock start failed");
        sink.pause().expect("mock pause failed");
        sink.stop().expect("mock stop failed");

        // Verify latency calculation
        let latency = sink.get_buffer_latency();
        assert!(latency.as_millis() > 0);

        // Verify lock-free sample flow
        producer.push(0.5).unwrap();
        producer.push(-0.5).unwrap();
        assert_eq!(consumer.pop().unwrap(), 0.5);
        assert_eq!(consumer.pop().unwrap(), -0.5);
    }
}
