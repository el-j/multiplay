//! Pure-Rust hardware audio sink adapter using CPAL and lock-free SPSC ring buffer.
//! Isolates CPAL stream lifecycle to a dedicated thread and guarantees real-time safety.

use amp_ports::driven::AudioSinkPort;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, Device, SampleRate, StreamConfig};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const DEFAULT_RING_BUFFER_FRAMES: usize = 65536;

/// Description of an audio hardware output device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDeviceDescription {
    pub name: String,
    pub is_default: bool,
}

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
    device_name: String,
}

impl CpalAudioSink {
    /// Lists all detected hardware audio output devices on the host.
    pub fn list_output_devices() -> Vec<AudioDeviceDescription> {
        let host = cpal::default_host();
        let default_name = host.default_output_device().and_then(|d| d.name().ok());
        let mut result = Vec::new();

        if let Ok(devices) = host.devices() {
            for d in devices {
                if let Ok(name) = d.name() {
                    let lower = name.to_lowercase();
                    if lower.contains("microphone") || lower.contains("input") {
                        continue;
                    }
                    let is_default = default_name.as_ref() == Some(&name);
                    if !result.iter().any(|item: &AudioDeviceDescription| item.name == name) {
                        result.push(AudioDeviceDescription { name, is_default });
                    }
                }
            }
        }
        result
    }

    /// Spawns the audio actor thread using the default output device.
    pub fn new() -> Result<(Self, Producer<f32>), String> {
        Self::new_with_device(None)
    }

    /// Spawns the audio actor thread for a specific device, or system default if None.
    pub fn new_with_device(target_device_name: Option<&str>) -> Result<(Self, Producer<f32>), String> {
        let (device, config, resolved_name) = resolve_output_device(target_device_name)?;

        let initial_rate = config.sample_rate.0;
        let initial_channels = config.channels;

        let sample_rate = Arc::new(AtomicU32::new(initial_rate));
        let channels = Arc::new(AtomicU16::new(initial_channels));

        let (producer, consumer) = RingBuffer::<f32>::new(DEFAULT_RING_BUFFER_FRAMES);
        let (cmd_tx, cmd_rx) = mpsc::channel::<SinkControlMsg>();

        let sr_clone = Arc::clone(&sample_rate);
        let ch_clone = Arc::clone(&channels);

        // Spawn dedicated audio thread that owns the non-Send/Sync CPAL Stream
        thread::Builder::new()
            .name("amp-cpal-actor".to_string())
            .spawn(move || {
                run_cpal_actor(device, config, consumer, cmd_rx, sr_clone, ch_clone);
            })
            .map_err(|e| format!("Failed to spawn CPAL actor thread: {e}"))?;

        let sink = Self {
            control_tx: Some(cmd_tx),
            sample_rate,
            channels,
            device_name: resolved_name,
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
            device_name: "Mock Device".to_string(),
        };

        (sink, producer, consumer)
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
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

fn resolve_output_device(target_name: Option<&str>) -> Result<(Device, StreamConfig, String), String> {
    let host = cpal::default_host();

    let (device, resolved_name) = if let Some(target) = target_name {
        let all_devs = host.devices().map_err(|e| format!("Device enumeration failed: {e}"))?;
        let mut found = None;
        for d in all_devs {
            if let Ok(name) = d.name() {
                if name == target {
                    found = Some((d, name));
                    break;
                }
            }
        }
        found.ok_or_else(|| format!("Target audio device '{target}' not found"))?
    } else if let Some(def) = host.default_output_device() {
        let name = def.name().unwrap_or_else(|_| "Default Audio Device".to_string());
        (def, name)
    } else {
        let mut out_devs = host.output_devices().map_err(|e| format!("Output device enumeration failed: {e}"))?;
        let d = out_devs.next().ok_or_else(|| "No hardware audio output devices found".to_string())?;
        let name = d.name().unwrap_or_else(|_| "Unknown Audio Device".to_string());
        (d, name)
    };

    // Determine working StreamConfig, handling macOS CoreAudio Apple Silicon driver quirks
    let config = if let Ok(supported) = device.default_output_config() {
        StreamConfig {
            channels: supported.channels(),
            sample_rate: supported.sample_rate(),
            buffer_size: BufferSize::Default,
        }
    } else {
        // Fallback standard 48kHz stereo configuration for built-in Mac speakers
        StreamConfig {
            channels: 2,
            sample_rate: SampleRate(48000),
            buffer_size: BufferSize::Default,
        }
    };

    Ok((device, config, resolved_name))
}

fn run_cpal_actor(
    device: Device,
    config: StreamConfig,
    mut consumer: Consumer<f32>,
    cmd_rx: mpsc::Receiver<SinkControlMsg>,
    current_sr: Arc<AtomicU32>,
    current_ch: Arc<AtomicU16>,
) {
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

    let stream = match stream_res {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize audio output stream: {e}");
            while let Ok(msg) = cmd_rx.recv() {
                match msg {
                    SinkControlMsg::Configure { reply, .. } => { let _ = reply.send(Err(e.to_string())); }
                    SinkControlMsg::Start { reply } => { let _ = reply.send(Err(e.to_string())); }
                    SinkControlMsg::Pause { reply } => { let _ = reply.send(Err(e.to_string())); }
                    SinkControlMsg::Stop { reply } => { let _ = reply.send(Err(e.to_string())); }
                    SinkControlMsg::Terminate => break,
                }
            }
            return;
        }
    };

    // The stream is built in paused state. It is controlled via play() and pause()
    while let Ok(msg) = cmd_rx.recv() {
        match msg {
            SinkControlMsg::Configure { sample_rate, channels, reply } => {
                current_sr.store(sample_rate, Ordering::Relaxed);
                current_ch.store(channels, Ordering::Relaxed);
                let _ = reply.send(Ok(()));
            }
            SinkControlMsg::Start { reply } => {
                let res = stream.play().map_err(|e| format!("Stream play failed: {e}"));
                let _ = reply.send(res);
            }
            SinkControlMsg::Pause { reply } => {
                let res = stream.pause().map_err(|e| format!("Stream pause failed: {e}"));
                let _ = reply.send(res);
            }
            SinkControlMsg::Stop { reply } => {
                let res = stream.pause().map_err(|e| format!("Stream stop failed: {e}"));
                let _ = reply.send(res);
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

        let latency = sink.get_buffer_latency();
        assert!(latency.as_millis() > 0);

        producer.push(0.5).unwrap();
        producer.push(-0.5).unwrap();
        assert_eq!(consumer.pop().unwrap(), 0.5);
        assert_eq!(consumer.pop().unwrap(), -0.5);
    }

    #[test]
    fn test_device_enumeration_and_fallback() {
        let devices = CpalAudioSink::list_output_devices();
        assert!(!devices.is_empty(), "Should detect at least one output device");
        println!("Detected devices: {:?}", devices);
    }

    #[test]
    fn test_repeated_start_pause_stop_lifecycle() {
        let (mut sink, mut producer) = CpalAudioSink::new().expect("Failed to initialize default sink");
        println!("Initialized sink on device: {}", sink.device_name());

        // Fill test samples
        for _ in 0..1024 {
            let _ = producer.push(0.0);
        }

        // Test repeated Start / Pause / Stop without consumer eviction
        sink.start().expect("First start failed");
        sink.pause().expect("Pause failed");
        sink.start().expect("Second start failed");
        sink.stop().expect("Stop failed");
        sink.start().expect("Third start after stop failed");
        sink.stop().expect("Final stop failed");
    }

    #[test]
    fn test_play_macbook_pro_speakers_explicit() {
        let target = "MacBook Pro Speakers";
        if let Ok((mut sink, mut producer)) = CpalAudioSink::new_with_device(Some(target)) {
            assert_eq!(sink.device_name(), target);
            sink.start().expect("Failed to start MacBook Pro Speakers");
            for _ in 0..2048 {
                let _ = producer.push(0.0);
            }
            std::thread::sleep(Duration::from_millis(50));
            sink.stop().expect("Failed to stop MacBook Pro Speakers");
        }
    }
}
