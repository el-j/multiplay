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
use std::sync::{Arc, Mutex};
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
    /// The returned queue takes interleaved stereo, whatever the device's channel count.
    pub fn new_with_device(
        target_device_name: Option<&str>,
    ) -> Result<(Self, Producer<f32>), String> {
        Self::open(target_device_name, false)
    }

    /// Like [`Self::new_with_device`], but the queue takes frames with all of the device's
    /// channels (see [`Self::channels`]), e.g. four for a DJ interface with a headphone pair.
    pub fn new_with_device_native(
        target_device_name: Option<&str>,
    ) -> Result<(Self, Producer<f32>), String> {
        Self::open(target_device_name, true)
    }

    fn open(
        target_device_name: Option<&str>,
        native: bool,
    ) -> Result<(Self, Producer<f32>), String> {
        let (device, config, resolved_name) = resolve_output_device(target_device_name)?;
        let source_channels = if native { config.channels } else { 2 };

        let initial_rate = config.sample_rate;
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
                run_cpal_actor(
                    device,
                    config,
                    source_channels,
                    consumer,
                    cmd_rx,
                    sr_clone,
                    ch_clone,
                );
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
                    SinkControlMsg::Configure { reply, .. } => {
                        let _ = reply.send(Ok(()));
                    }
                    SinkControlMsg::Start { reply } => {
                        let _ = reply.send(Ok(()));
                    }
                    SinkControlMsg::Pause { reply } => {
                        let _ = reply.send(Ok(()));
                    }
                    SinkControlMsg::Stop { reply } => {
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
            device_name: "Mock Device".to_string(),
        };

        (sink, producer, consumer)
    }

    /// Hardware-free sink for tests, CI runners without audio devices, and headless mode.
    /// A background thread drains the ring buffer at real-time pace and discards the samples,
    /// so producers see the same back-pressure as with a real device.
    pub fn new_null(sample_rate: u32, channels: u16) -> (Self, Producer<f32>) {
        let (mut sink, producer, consumer) = Self::new_mock(sample_rate, channels);
        sink.device_name = "Null Output (no audio device)".to_string();
        spawn_realtime_drain(consumer, sample_rate, channels, None);
        (sink, producer)
    }

    /// Like [`Self::new_null`], but records every drained sample so tests can assert on
    /// what the pipeline actually rendered.
    pub fn new_capture(sample_rate: u32, channels: u16) -> (Self, Producer<f32>, CapturedOutput) {
        let (mut sink, producer, consumer) = Self::new_mock(sample_rate, channels);
        sink.device_name = "Capture Output (no audio device)".to_string();
        let captured = CapturedOutput::default();
        spawn_realtime_drain(consumer, sample_rate, channels, Some(captured.clone()));
        (sink, producer, captured)
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

/// Samples recorded by a capture sink (see [`CpalAudioSink::new_capture`]).
#[derive(Clone, Default)]
pub struct CapturedOutput(Arc<Mutex<Vec<f32>>>);

impl CapturedOutput {
    /// Copy of every sample drained so far, interleaved.
    pub fn samples(&self) -> Vec<f32> {
        self.0.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Number of samples drained so far.
    pub fn len(&self) -> usize {
        self.0.lock().map(|s| s.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Largest absolute sample value drained so far (0.0 for silence).
    pub fn peak(&self) -> f32 {
        self.0
            .lock()
            .map(|s| s.iter().fold(0.0f32, |m, x| m.max(x.abs())))
            .unwrap_or(0.0)
    }
}

/// Drains `consumer` at real-time pace on a background thread until the producer is dropped.
fn spawn_realtime_drain(
    mut consumer: Consumer<f32>,
    sample_rate: u32,
    channels: u16,
    capture: Option<CapturedOutput>,
) {
    let per_sec = sample_rate as f64 * channels.max(1) as f64;
    thread::Builder::new()
        .name("amp-null-sink".to_string())
        .spawn(move || {
            // Drain by elapsed time, like a sound card, so a thread that oversleeps on a busy
            // machine catches up instead of draining slower than real time.
            let start = std::time::Instant::now();
            let mut drained = 0usize;
            while !consumer.is_abandoned() {
                // Whole frames only, so channels never shift.
                let ch = usize::from(channels.max(1));
                let due = (start.elapsed().as_secs_f64() * per_sec) as usize / ch * ch;
                let n = (consumer.slots() / ch * ch).min(due.saturating_sub(drained));
                // What was not there in time is lost, as on a real card (an underrun).
                drained = due;
                if let Ok(chunk) = consumer.read_chunk(n) {
                    if let Some(capture) = &capture {
                        let (a, b) = chunk.as_slices();
                        if let Ok(mut buf) = capture.0.lock() {
                            buf.extend_from_slice(a);
                            buf.extend_from_slice(b);
                        }
                    }
                    chunk.commit_all();
                }
                thread::sleep(Duration::from_millis(10));
            }
        })
        .ok();
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
    fn null_sink_drains_without_hardware() {
        let (mut sink, mut producer) = CpalAudioSink::new_null(48_000, 2);
        sink.start().expect("null sink start");
        for _ in 0..4096 {
            let _ = producer.push(0.25);
        }
        std::thread::sleep(Duration::from_millis(100));
        assert!(
            producer.slots() > DEFAULT_RING_BUFFER_FRAMES - 4096,
            "samples must be drained"
        );
        sink.stop().expect("null sink stop");
    }

    #[test]
    fn capture_sink_keeps_frames_whole() {
        let (_sink, mut producer, captured) = CpalAudioSink::new_capture(44_100, 2);
        for i in 0..20_000 {
            let _ = producer.push(if i % 2 == 0 { 1.0 } else { -1.0 });
            if i % 997 == 0 {
                std::thread::sleep(Duration::from_millis(3));
            }
        }
        std::thread::sleep(Duration::from_millis(400));
        let s = captured.samples();
        assert!(s.len() >= 2 && s.len() % 2 == 0, "{} samples", s.len());
        assert!(
            s.chunks(2).all(|f| f == [1.0, -1.0]),
            "left/right never swap"
        );
    }

    #[test]
    fn capture_sink_records_drained_samples_in_order() {
        let (mut sink, mut producer, captured) = CpalAudioSink::new_capture(48_000, 2);
        sink.start().expect("capture sink start");
        for i in 0..2048 {
            producer.push(i as f32).unwrap();
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while captured.len() < 2048 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let samples = captured.samples();
        assert_eq!(samples.len(), 2048);
        assert!(samples.iter().enumerate().all(|(i, s)| *s == i as f32));
        assert!(captured.peak() >= 2047.0);
    }

    #[test]
    #[ignore = "needs audio hardware (run with `make test-hw`)"]
    fn test_device_enumeration_and_fallback() {
        let devices = CpalAudioSink::list_output_devices();
        assert!(
            !devices.is_empty(),
            "Should detect at least one output device"
        );
    }

    #[test]
    #[ignore = "needs audio hardware (run with `make test-hw`)"]
    fn test_repeated_start_pause_stop_lifecycle() {
        let (mut sink, mut producer) =
            CpalAudioSink::new().expect("Failed to initialize default sink");

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
    #[ignore = "needs audio hardware (run with `make test-hw`)"]
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
