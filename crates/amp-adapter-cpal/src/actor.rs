//! Dedicated background actor thread managing CPAL stream lifecycle and hot-plug/error recovery (`TASK-133`).

#![forbid(unsafe_code)]

use crate::callback::fill_output_typed_with_clock;
#[cfg(test)]
pub(crate) use crate::callback::fill_output_with_clock;
pub use crate::callback::{
    f32_to_i16_dithered, f32_to_i32_dithered, f32_to_u16_dithered, TpdfDither,
};
use crate::device::{
    default_output_device_name, negotiate_for_device, resolve_output_device_with_rate,
    NegotiatedSampleFormat,
};
use amp_ports::{AudioSinkError, OutputClock};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{BufferSize, Device, Stream, StreamConfig};
use rtrb::Consumer;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub(crate) enum SinkControlMsg {
    Configure {
        sample_rate: u32,
        channels: u16,
        reply: mpsc::Sender<Result<(), AudioSinkError>>,
    },
    Start {
        reply: mpsc::Sender<Result<(), AudioSinkError>>,
    },
    Pause {
        reply: mpsc::Sender<Result<(), AudioSinkError>>,
    },
    Stop {
        reply: mpsc::Sender<Result<(), AudioSinkError>>,
    },
    SimulateStreamError {
        fallback_device_name: Option<String>,
        reply: mpsc::Sender<Result<(), AudioSinkError>>,
    },
    Terminate,
}

fn build_typed_stream(
    device: &Device,
    config: &StreamConfig,
    sample_format: NegotiatedSampleFormat,
    source_channels: usize,
    consumer: Arc<Mutex<Consumer<f32>>>,
    clock: Arc<OutputClock>,
    stream_error: Arc<AtomicBool>,
) -> Result<Stream, cpal::Error> {
    let device_channels = usize::from(config.channels);
    let err_flag = Arc::clone(&stream_error);
    let err_fn = move |err| {
        eprintln!("Audio stream callback error: {err}");
        err_flag.store(true, Ordering::Release);
    };

    match sample_format {
        NegotiatedSampleFormat::F32 => {
            let mut dither = TpdfDither::default();
            device.build_output_stream(
                *config,
                move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let Ok(mut cons) = consumer.try_lock() else {
                        output.fill(0.0);
                        return;
                    };
                    fill_output_typed_with_clock(
                        output,
                        device_channels,
                        source_channels,
                        &mut cons,
                        &clock,
                        &mut dither,
                        0.0,
                        |s, _| s.clamp(-1.0, 1.0),
                    );
                },
                err_fn,
                None,
            )
        }
        NegotiatedSampleFormat::I16 => {
            let mut dither = TpdfDither::default();
            device.build_output_stream(
                *config,
                move |output: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let Ok(mut cons) = consumer.try_lock() else {
                        output.fill(0);
                        return;
                    };
                    fill_output_typed_with_clock(
                        output,
                        device_channels,
                        source_channels,
                        &mut cons,
                        &clock,
                        &mut dither,
                        0i16,
                        f32_to_i16_dithered,
                    );
                },
                err_fn,
                None,
            )
        }
        NegotiatedSampleFormat::U16 => {
            let mut dither = TpdfDither::default();
            device.build_output_stream(
                *config,
                move |output: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    let Ok(mut cons) = consumer.try_lock() else {
                        output.fill(32_768);
                        return;
                    };
                    fill_output_typed_with_clock(
                        output,
                        device_channels,
                        source_channels,
                        &mut cons,
                        &clock,
                        &mut dither,
                        32_768u16,
                        f32_to_u16_dithered,
                    );
                },
                err_fn,
                None,
            )
        }
        NegotiatedSampleFormat::I32 => {
            let mut dither = TpdfDither::default();
            device.build_output_stream(
                *config,
                move |output: &mut [i32], _: &cpal::OutputCallbackInfo| {
                    let Ok(mut cons) = consumer.try_lock() else {
                        output.fill(0);
                        return;
                    };
                    fill_output_typed_with_clock(
                        output,
                        device_channels,
                        source_channels,
                        &mut cons,
                        &clock,
                        &mut dither,
                        0i32,
                        f32_to_i32_dithered,
                    );
                },
                err_fn,
                None,
            )
        }
    }
}

pub(crate) struct CpalActorParams {
    pub device: Device,
    pub config: StreamConfig,
    pub sample_format: NegotiatedSampleFormat,
    pub follow_default: bool,
    pub source_channels: u16,
    pub consumer: Consumer<f32>,
    pub cmd_rx: mpsc::Receiver<SinkControlMsg>,
    pub current_sr: Arc<AtomicU32>,
    pub current_ch: Arc<AtomicU16>,
    pub device_buffer_frames: Arc<AtomicU32>,
    pub active_device_name: Arc<Mutex<String>>,
    pub recovery_count: Arc<AtomicU32>,
    pub clock: Arc<OutputClock>,
}

pub(crate) fn run_cpal_actor(params: CpalActorParams) {
    let CpalActorParams {
        mut device,
        mut config,
        mut sample_format,
        follow_default,
        mut source_channels,
        consumer,
        cmd_rx,
        current_sr,
        current_ch,
        device_buffer_frames,
        active_device_name,
        recovery_count,
        clock,
    } = params;

    let shared_consumer = Arc::new(Mutex::new(consumer));
    let stream_error = Arc::new(AtomicBool::new(false));
    let mut is_playing = false;
    let mut last_default_poll = Instant::now();

    let mut active_stream: Option<Stream> = build_typed_stream(
        &device,
        &config,
        sample_format,
        usize::from(source_channels),
        Arc::clone(&shared_consumer),
        Arc::clone(&clock),
        Arc::clone(&stream_error),
    )
    .ok();

    loop {
        match cmd_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(msg) => match msg {
                SinkControlMsg::Configure {
                    sample_rate,
                    channels,
                    reply,
                } => {
                    active_stream = None;
                    source_channels = channels.max(1);
                    let negotiated = negotiate_for_device(&device, Some(sample_rate));
                    config = StreamConfig {
                        channels: negotiated.channels,
                        sample_rate: negotiated.sample_rate,
                        buffer_size: BufferSize::Default,
                    };
                    sample_format = negotiated.sample_format;
                    current_sr.store(config.sample_rate, Ordering::Relaxed);
                    current_ch.store(config.channels, Ordering::Relaxed);
                    device_buffer_frames.store(negotiated.buffer_frames, Ordering::Relaxed);
                    clock.set_format(config.sample_rate, source_channels);
                    clock.flush_and_reset(clock.current_position());

                    match build_typed_stream(
                        &device,
                        &config,
                        sample_format,
                        usize::from(source_channels),
                        Arc::clone(&shared_consumer),
                        Arc::clone(&clock),
                        Arc::clone(&stream_error),
                    ) {
                        Ok(s) => {
                            if is_playing {
                                let _ = s.play();
                            }
                            active_stream = Some(s);
                            let _ = reply.send(Ok(()));
                        }
                        Err(e) => {
                            let _ = reply.send(Err(AudioSinkError::StreamError(e.to_string())));
                        }
                    }
                }
                SinkControlMsg::Start { reply } => {
                    clock.resume();
                    is_playing = true;
                    let res = if let Some(ref s) = active_stream {
                        s.play().map_err(|e| {
                            AudioSinkError::StreamError(format!("Stream play failed: {e}"))
                        })
                    } else {
                        Err(AudioSinkError::StreamError(
                            "Audio stream not initialized".into(),
                        ))
                    };
                    let _ = reply.send(res);
                }
                SinkControlMsg::Pause { reply } => {
                    is_playing = false;
                    std::thread::sleep(Duration::from_millis(6));
                    let res = if let Some(ref s) = active_stream {
                        s.pause().map_err(|e| {
                            AudioSinkError::StreamError(format!("Stream pause failed: {e}"))
                        })
                    } else {
                        Ok(())
                    };
                    let _ = reply.send(res);
                }
                SinkControlMsg::Stop { reply } => {
                    is_playing = false;
                    let res = if let Some(ref s) = active_stream {
                        s.pause().map_err(|e| {
                            AudioSinkError::StreamError(format!("Stream stop failed: {e}"))
                        })
                    } else {
                        Ok(())
                    };
                    let _ = reply.send(res);
                }
                SinkControlMsg::SimulateStreamError {
                    fallback_device_name,
                    reply,
                } => {
                    stream_error.store(false, Ordering::Release);
                    clock.flush_and_reset(clock.current_position());
                    recovery_count.fetch_add(1, Ordering::SeqCst);
                    if let Some(name) = fallback_device_name {
                        if let Ok(mut g) = active_device_name.lock() {
                            *g = name;
                        }
                    }
                    let _ = reply.send(Ok(()));
                }
                SinkControlMsg::Terminate => break,
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let had_error = stream_error.swap(false, Ordering::AcqRel);
                let default_changed = follow_default
                    && last_default_poll.elapsed() >= Duration::from_millis(500)
                    && {
                        last_default_poll = Instant::now();
                        default_output_device_name().is_some_and(|def_name| {
                            active_device_name
                                .lock()
                                .map(|cur| *cur != def_name)
                                .unwrap_or(false)
                        })
                    };

                if had_error || default_changed {
                    active_stream = None;
                    clock.flush_and_reset(clock.current_position());
                    let target = if follow_default || had_error {
                        None
                    } else {
                        active_device_name.lock().ok().map(|s| s.clone())
                    };
                    if let Ok((new_dev, new_cfg, new_fmt, new_buf, new_name)) =
                        resolve_output_device_with_rate(
                            target.as_deref(),
                            Some(current_sr.load(Ordering::Relaxed)),
                        )
                    {
                        device = new_dev;
                        config = new_cfg;
                        sample_format = new_fmt;
                        current_sr.store(config.sample_rate, Ordering::Relaxed);
                        current_ch.store(config.channels, Ordering::Relaxed);
                        device_buffer_frames.store(new_buf, Ordering::Relaxed);
                        clock.set_format(config.sample_rate, source_channels);
                        if let Ok(mut g) = active_device_name.lock() {
                            *g = new_name;
                        }
                        if let Ok(s) = build_typed_stream(
                            &device,
                            &config,
                            sample_format,
                            usize::from(source_channels),
                            Arc::clone(&shared_consumer),
                            Arc::clone(&clock),
                            Arc::clone(&stream_error),
                        ) {
                            if is_playing {
                                let _ = s.play();
                            }
                            active_stream = Some(s);
                            recovery_count.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}
