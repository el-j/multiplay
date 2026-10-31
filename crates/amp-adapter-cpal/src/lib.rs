//! Pure-Rust hardware audio sink adapter using CPAL and lock-free SPSC ring buffer.
//! Isolates CPAL stream lifecycle to a dedicated thread and guarantees real-time safety.

#![forbid(unsafe_code)]

pub mod actor;
pub mod callback;
pub mod device;
pub mod null_sink;

use actor::{run_cpal_actor, CpalActorParams, SinkControlMsg};
pub use device::{
    default_output_device_name, list_output_devices, negotiate_stream_config,
    resolve_output_device, resolve_output_device_with_rate, AudioDeviceDescription,
    NegotiatedSampleFormat, NegotiatedStreamConfig, SupportedConfigRangeSpec,
    DEFAULT_DEVICE_BUFFER_FRAMES,
};
use null_sink::spawn_realtime_drain;
pub use null_sink::CapturedOutput;

use amp_ports::driven::AudioSinkPort;
use amp_ports::{AudioSinkError, OutputClock};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Default hardware ring buffer duration in milliseconds (~150 ms, TASK-127).
pub const DEFAULT_RING_BUFFER_MS: u32 = 150;
/// Default ring buffer capacity in interleaved samples (150 ms at 48 kHz stereo = 14,400 samples).
pub const DEFAULT_RING_BUFFER_FRAMES: usize = 14_400;

/// Computes the ring buffer capacity in interleaved samples for `buffer_ms` at `sample_rate` and `channels`.
pub fn ring_capacity_samples(sample_rate: u32, channels: u16, buffer_ms: u32) -> usize {
    let ch = usize::from(channels.max(1));
    let frames = ((sample_rate as usize) * (buffer_ms.max(20) as usize)) / 1000;
    (frames * ch).max(2048)
}

/// Thread-safe (`Send + Sync`) CPAL Audio Sink handle implementing `AudioSinkPort`.
pub struct CpalAudioSink {
    control_tx: Option<Sender<SinkControlMsg>>,
    sample_rate: Arc<AtomicU32>,
    channels: Arc<AtomicU16>,
    device_buffer_frames: Arc<AtomicU32>,
    ring_capacity: usize,
    device_name: Arc<Mutex<String>>,
    recovery_count: Arc<AtomicU32>,
    clock: Arc<OutputClock>,
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
    /// The returned queue takes interleaved stereo, whatever the device's channel count.
    pub fn new_with_device(
        target_device_name: Option<&str>,
    ) -> Result<(Self, Producer<f32>), String> {
        Self::open(target_device_name, false, DEFAULT_RING_BUFFER_MS)
    }

    /// Spawns the audio actor thread with an explicit ring buffer duration in milliseconds.
    pub fn new_with_buffer_ms(
        target_device_name: Option<&str>,
        buffer_ms: u32,
    ) -> Result<(Self, Producer<f32>), String> {
        Self::open(target_device_name, false, buffer_ms)
    }

    /// Like [`Self::new_with_device`], but the queue takes frames with all of the device's
    /// channels (see [`Self::channels`]), e.g. four for a DJ interface with a headphone pair.
    pub fn new_with_device_native(
        target_device_name: Option<&str>,
    ) -> Result<(Self, Producer<f32>), String> {
        Self::open(target_device_name, true, DEFAULT_RING_BUFFER_MS)
    }

    fn open(
        target_device_name: Option<&str>,
        native: bool,
        buffer_ms: u32,
    ) -> Result<(Self, Producer<f32>), String> {
        let follow_default = target_device_name.is_none();
        let (device, config, sample_format, buf_frames, resolved_name) =
            resolve_output_device_with_rate(target_device_name, None)?;
        let source_channels = if native { config.channels } else { 2 };

        let initial_rate = config.sample_rate;
        let initial_channels = config.channels;

        let sample_rate = Arc::new(AtomicU32::new(initial_rate));
        let channels = Arc::new(AtomicU16::new(initial_channels));
        let device_buffer_frames = Arc::new(AtomicU32::new(buf_frames));
        let device_name = Arc::new(Mutex::new(resolved_name));
        let recovery_count = Arc::new(AtomicU32::new(0));
        let clock = Arc::new(OutputClock::new(initial_rate, source_channels));

        let ring_capacity = ring_capacity_samples(initial_rate, source_channels, buffer_ms);
        let (producer, consumer) = RingBuffer::<f32>::new(ring_capacity);
        let (cmd_tx, cmd_rx) = mpsc::channel::<SinkControlMsg>();

        let params = CpalActorParams {
            device,
            config,
            sample_format,
            follow_default,
            source_channels,
            consumer,
            cmd_rx,
            current_sr: Arc::clone(&sample_rate),
            current_ch: Arc::clone(&channels),
            device_buffer_frames: Arc::clone(&device_buffer_frames),
            active_device_name: Arc::clone(&device_name),
            recovery_count: Arc::clone(&recovery_count),
            clock: Arc::clone(&clock),
        };

        thread::Builder::new()
            .name("amp-cpal-actor".to_string())
            .spawn(move || {
                run_cpal_actor(params);
            })
            .map_err(|e| format!("Failed to spawn CPAL actor thread: {e}"))?;

        let sink = Self {
            control_tx: Some(cmd_tx),
            sample_rate,
            channels,
            device_buffer_frames,
            ring_capacity,
            device_name,
            recovery_count,
            clock,
        };

        Ok((sink, producer))
    }

    /// Mock constructor for headless CI and unit testing.
    pub fn new_mock(sample_rate: u32, channels: u16) -> (Self, Producer<f32>, Consumer<f32>) {
        let ring_capacity = ring_capacity_samples(sample_rate, channels, DEFAULT_RING_BUFFER_MS);
        let (producer, consumer) = RingBuffer::<f32>::new(ring_capacity);
        let sr = Arc::new(AtomicU32::new(sample_rate));
        let ch = Arc::new(AtomicU16::new(channels));
        let device_buffer_frames = Arc::new(AtomicU32::new(DEFAULT_DEVICE_BUFFER_FRAMES));
        let device_name = Arc::new(Mutex::new("Mock Device".to_string()));
        let recovery_count = Arc::new(AtomicU32::new(0));
        let clock = Arc::new(OutputClock::new(sample_rate, channels));

        let actor_clock = Arc::clone(&clock);
        let actor_sr = Arc::clone(&sr);
        let actor_ch = Arc::clone(&ch);
        let actor_name = Arc::clone(&device_name);
        let actor_recoveries = Arc::clone(&recovery_count);

        let (cmd_tx, cmd_rx) = mpsc::channel::<SinkControlMsg>();
        thread::spawn(move || {
            while let Ok(msg) = cmd_rx.recv() {
                match msg {
                    SinkControlMsg::Configure {
                        sample_rate,
                        channels,
                        reply,
                    } => {
                        actor_sr.store(sample_rate, Ordering::Relaxed);
                        actor_ch.store(channels, Ordering::Relaxed);
                        actor_clock.set_format(sample_rate, channels);
                        let _ = reply.send(Ok(()));
                    }
                    SinkControlMsg::Start { reply } => {
                        actor_clock.resume();
                        let _ = reply.send(Ok(()));
                    }
                    SinkControlMsg::Pause { reply } => {
                        let _ = reply.send(Ok(()));
                    }
                    SinkControlMsg::Stop { reply } => {
                        let _ = reply.send(Ok(()));
                    }
                    SinkControlMsg::SimulateStreamError {
                        fallback_device_name,
                        reply,
                    } => {
                        actor_clock.flush_and_reset(actor_clock.current_position());
                        actor_recoveries.fetch_add(1, Ordering::SeqCst);
                        if let Some(name) = fallback_device_name {
                            if let Ok(mut g) = actor_name.lock() {
                                *g = name;
                            }
                        }
                        let _ = reply.send(Ok(()));
                    }
                    SinkControlMsg::Terminate => break,
                }
            }
        });

        let sink = Self {
            control_tx: Some(cmd_tx),
            sample_rate: sr,
            channels: ch,
            device_buffer_frames,
            ring_capacity,
            device_name,
            recovery_count,
            clock,
        };

        (sink, producer, consumer)
    }

    /// Hardware-free sink for tests, CI runners without audio devices, and headless mode.
    pub fn new_null(sample_rate: u32, channels: u16) -> (Self, Producer<f32>) {
        let (sink, producer, consumer) = Self::new_mock(sample_rate, channels);
        if let Ok(mut g) = sink.device_name.lock() {
            *g = "Null Output (no audio device)".to_string();
        }
        spawn_realtime_drain(
            consumer,
            sample_rate,
            channels,
            Arc::clone(&sink.clock),
            None,
        );
        (sink, producer)
    }

    /// Like [`Self::new_null`], but records every drained sample so tests can assert on
    /// what the pipeline actually rendered.
    pub fn new_capture(sample_rate: u32, channels: u16) -> (Self, Producer<f32>, CapturedOutput) {
        let (sink, producer, consumer) = Self::new_mock(sample_rate, channels);
        if let Ok(mut g) = sink.device_name.lock() {
            *g = "Capture Output (no audio device)".to_string();
        }
        let captured = CapturedOutput::default();
        spawn_realtime_drain(
            consumer,
            sample_rate,
            channels,
            Arc::clone(&sink.clock),
            Some(captured.clone()),
        );
        (sink, producer, captured)
    }

    /// Returns the shared lock-free output clock and visualizer tap for this sink.
    pub fn clock(&self) -> Arc<OutputClock> {
        Arc::clone(&self.clock)
    }

    /// Returns the currently active output device name.
    pub fn device_name(&self) -> String {
        self.device_name
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|_| "Unknown Device".to_string())
    }

    /// Number of hot-plug / stream error recoveries performed since creation (`TASK-133`).
    pub fn recovery_count(&self) -> u32 {
        self.recovery_count.load(Ordering::SeqCst)
    }

    /// Simulates a stream error / device hot-plug migration (`TASK-133`).
    pub fn simulate_stream_error(
        &self,
        fallback_device_name: Option<String>,
    ) -> Result<(), AudioSinkError> {
        if let Some(tx) = &self.control_tx {
            let (reply_tx, reply_rx) = mpsc::channel();
            tx.send(SinkControlMsg::SimulateStreamError {
                fallback_device_name,
                reply: reply_tx,
            })
            .map_err(|e| AudioSinkError::StreamError(e.to_string()))?;
            reply_rx
                .recv_timeout(Duration::from_millis(500))
                .map_err(|e| AudioSinkError::StreamError(format!("Recovery timeout: {e}")))?
        } else {
            Ok(())
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate.load(Ordering::Relaxed)
    }

    pub fn channels(&self) -> u16 {
        self.channels.load(Ordering::Relaxed)
    }

    pub fn ring_capacity(&self) -> usize {
        self.ring_capacity
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
    fn configure(&mut self, sample_rate: u32, channels: u16) -> Result<(), AudioSinkError> {
        self.sample_rate.store(sample_rate, Ordering::Relaxed);
        self.channels.store(channels, Ordering::Relaxed);
        self.clock.set_format(sample_rate, channels);

        if let Some(tx) = &self.control_tx {
            let (reply_tx, reply_rx) = mpsc::channel();
            tx.send(SinkControlMsg::Configure {
                sample_rate,
                channels,
                reply: reply_tx,
            })
            .map_err(|e| AudioSinkError::StreamError(e.to_string()))?;

            reply_rx
                .recv_timeout(Duration::from_millis(500))
                .map_err(|e| AudioSinkError::StreamError(format!("Configure timeout: {e}")))?
        } else {
            Ok(())
        }
    }

    fn start(&mut self) -> Result<(), AudioSinkError> {
        self.clock.resume();
        if let Some(tx) = &self.control_tx {
            let (reply_tx, reply_rx) = mpsc::channel();
            tx.send(SinkControlMsg::Start { reply: reply_tx })
                .map_err(|e| AudioSinkError::StreamError(e.to_string()))?;

            reply_rx
                .recv_timeout(Duration::from_millis(1000))
                .map_err(|e| AudioSinkError::StreamError(format!("Start timeout: {e}")))?
        } else {
            Ok(())
        }
    }

    fn pause(&mut self) -> Result<(), AudioSinkError> {
        let fade_frames = (self.sample_rate() * 5) / 1000;
        self.clock.request_pause(fade_frames.max(64));
        if let Some(tx) = &self.control_tx {
            let (reply_tx, reply_rx) = mpsc::channel();
            tx.send(SinkControlMsg::Pause { reply: reply_tx })
                .map_err(|e| AudioSinkError::StreamError(e.to_string()))?;

            reply_rx
                .recv_timeout(Duration::from_millis(500))
                .map_err(|e| AudioSinkError::StreamError(format!("Pause timeout: {e}")))?
        } else {
            Ok(())
        }
    }

    fn stop(&mut self) -> Result<(), AudioSinkError> {
        self.clock.flush_and_reset(Duration::ZERO);
        if let Some(tx) = &self.control_tx {
            let (reply_tx, reply_rx) = mpsc::channel();
            tx.send(SinkControlMsg::Stop { reply: reply_tx })
                .map_err(|e| AudioSinkError::StreamError(e.to_string()))?;

            reply_rx
                .recv_timeout(Duration::from_millis(500))
                .map_err(|e| AudioSinkError::StreamError(format!("Stop timeout: {e}")))?
        } else {
            Ok(())
        }
    }

    fn get_buffer_latency(&self) -> Duration {
        let sr = f64::from(self.sample_rate().max(1));
        let ch = usize::from(self.channels().max(1));
        let queued_frames = (self.clock.queued_samples() as usize) / ch;
        let hw_frames = self.device_buffer_frames.load(Ordering::Relaxed) as usize;
        Duration::from_secs_f64((queued_frames + hw_frames) as f64 / sr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_cpal_sink_lifecycle_and_latency() {
        let (mut sink, mut producer, mut consumer) = CpalAudioSink::new_mock(44_100, 2);
        assert_eq!(sink.sample_rate(), 44_100);
        assert_eq!(sink.channels(), 2);

        assert!(sink.configure(48_000, 2).is_ok());
        assert_eq!(sink.sample_rate(), 48_000);
        assert!(sink.start().is_ok());

        for i in 0..960 {
            producer.push(i as f32 * 0.001).unwrap();
        }
        sink.clock().record_pushed(960);

        // 480 queued frames + 512 hardware buffer frames at 48 kHz = ~20.66 ms
        let lat = sink.get_buffer_latency();
        assert!(lat >= Duration::from_millis(20) && lat <= Duration::from_millis(22));

        let mut popped = Vec::with_capacity(960);
        while let Ok(val) = consumer.pop() {
            popped.push(val);
        }
        assert_eq!(popped.len(), 960);

        assert!(sink.pause().is_ok());
        assert!(sink.stop().is_ok());
    }

    #[test]
    fn recovery_on_stream_error_flushes_epoch_rebuilds_stream_and_migrates_to_default() {
        let (sink, mut producer, mut consumer) = CpalAudioSink::new_mock(48_000, 2);
        for _ in 0..64 {
            producer.push(0.75).unwrap();
        }
        sink.clock().record_pushed(64);
        let initial_epoch = sink.clock().epoch();

        // Simulate headphone unplug / stream error migrating to built-in speakers
        sink.simulate_stream_error(Some("Built-in Speakers".to_string()))
            .expect("recovery should succeed");

        assert_eq!(sink.recovery_count(), 1);
        assert_eq!(sink.device_name(), "Built-in Speakers");
        assert!(sink.clock().epoch() > initial_epoch);
        assert_eq!(sink.clock().flush_until(), 64);

        // Push new-epoch audio and verify callback discards the 64 stale samples
        producer.push(-0.5).unwrap();
        producer.push(0.5).unwrap();
        sink.clock().record_pushed(2);

        let mut out = [0.0f32; 4];
        actor::fill_output_with_clock(&mut out, 2, 2, &mut consumer, &sink.clock());
        assert_eq!(out, [-0.5, 0.5, 0.0, 0.0]);
    }

    #[test]
    fn null_sink_drains_at_realtime_pace_without_hardware() {
        let (mut sink, mut producer) = CpalAudioSink::new_null(48_000, 2);
        sink.start().unwrap();
        let initial_free = producer.slots();
        for _ in 0..4_800 {
            producer.push(0.25).unwrap();
        }
        sink.clock().record_pushed(4_800);
        assert!(producer.slots() < initial_free);

        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while producer.slots() < initial_free && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(producer.slots(), initial_free);
    }

    #[test]
    fn capture_sink_records_rendered_samples_and_peaks() {
        let (mut sink, mut producer, captured) = CpalAudioSink::new_capture(48_000, 2);
        sink.start().unwrap();
        for i in 0..960 {
            let s = (i as f32 * 0.1).sin() * 0.8;
            producer.push(s).unwrap();
        }
        sink.clock().record_pushed(960);
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while captured.len() < 960 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(captured.len(), 960);
        assert!((captured.peak() - 0.8).abs() < 0.01);
    }
}
