//! Pure-Rust hardware audio sink adapter using CPAL and lock-free SPSC ring buffer.
//! Isolates CPAL stream lifecycle to a dedicated thread and guarantees real-time safety.

#![forbid(unsafe_code)]

pub mod actor;
pub mod device;

use actor::{run_cpal_actor, SinkControlMsg};
pub use device::{list_output_devices, resolve_output_device, AudioDeviceDescription};

use amp_ports::driven::AudioSinkPort;
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const DEFAULT_RING_BUFFER_FRAMES: usize = 65536;

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
        device::list_output_devices()
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
    }

    #[test]
    fn test_repeated_start_pause_stop_lifecycle() {
        let (mut sink, mut producer) = CpalAudioSink::new().expect("Failed to initialize default sink");

        for _ in 0..1024 {
            let _ = producer.push(0.0);
        }

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
