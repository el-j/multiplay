//! Integration tests for `PlayerEngine` (`PLAN-037`: `TASK-125`, `TASK-127`, `TASK-128`,
//! `TASK-129`, `TASK-131`).

#![forbid(unsafe_code)]

use amp_core_engine::player::{
    default_runtime_handle, PlayerEngine, PlayerEngineConfig, DEFAULT_RESOLVE_TIMEOUT,
};
use amp_core_engine::{EventBus, PlayerCommand, PlayerEvent, RepeatMode, ShuffleMode};
use amp_domain::{AudioSource, PlaybackState, Track, TrackMetadata};
use amp_ports::{
    async_trait, AudioSinkError, AudioSinkPort, AudioSourceStream, DecodeError, DecoderFactoryPort,
    OutputClock, PcmStreamPort, PlaylistManagementPort, ProviderError, StreamResolverPort,
};
use rtrb::{Consumer, RingBuffer};
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct SyntheticSineStream {
    sample_rate: u32,
    total_frames: usize,
    pos_frame: usize,
    freq_hz: f32,
}

impl SyntheticSineStream {
    fn new(sample_rate: u32, duration: Duration, freq_hz: f32) -> Self {
        let total_frames = (sample_rate as f64 * duration.as_secs_f64()) as usize;
        Self {
            sample_rate,
            total_frames,
            pos_frame: 0,
            freq_hz,
        }
    }
}

impl PcmStreamPort for SyntheticSineStream {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            self.total_frames as f64 / self.sample_rate as f64,
        ))
    }

    fn elapsed(&self) -> Duration {
        Duration::from_secs_f64(self.pos_frame as f64 / self.sample_rate as f64)
    }

    fn seek(&mut self, target: Duration) -> Result<Duration, DecodeError> {
        let frame =
            ((target.as_secs_f64() * self.sample_rate as f64) as usize).min(self.total_frames);
        self.pos_frame = frame;
        Ok(self.elapsed())
    }

    fn read_interleaved(&mut self, out: &mut [f32]) -> Result<usize, DecodeError> {
        let avail_frames = self.total_frames.saturating_sub(self.pos_frame);
        let frames = (out.len() / 2).min(avail_frames);
        for i in 0..frames {
            let t = (self.pos_frame + i) as f32 / self.sample_rate as f32;
            let v = 0.5 * (2.0 * std::f32::consts::PI * self.freq_hz * t).sin();
            out[i * 2] = v;
            out[i * 2 + 1] = v;
        }
        self.pos_frame += frames;
        Ok(frames * 2)
    }
}

struct MockDecoderFactory {
    default_rate: u32,
    default_duration: Duration,
}

impl DecoderFactoryPort for MockDecoderFactory {
    fn open_file(&self, path: &str) -> Result<Box<dyn PcmStreamPort>, DecodeError> {
        if path.contains("missing") {
            return Err(DecodeError::OpenFailed(format!("File not found: {path}")));
        }
        let rate = if path.contains("44100") {
            44_100
        } else if path.contains("96000") {
            96_000
        } else {
            self.default_rate
        };
        Ok(Box::new(SyntheticSineStream::new(
            rate,
            self.default_duration,
            440.0,
        )))
    }

    fn open_stream(
        &self,
        _stream: Box<dyn AudioSourceStream>,
    ) -> Result<Box<dyn PcmStreamPort>, DecodeError> {
        Ok(Box::new(SyntheticSineStream::new(
            self.default_rate,
            self.default_duration,
            880.0,
        )))
    }
}

#[derive(Default)]
struct DummySink;

impl AudioSinkPort for DummySink {
    fn configure(&mut self, _sr: u32, _ch: u16) -> Result<(), AudioSinkError> {
        Ok(())
    }
    fn start(&mut self) -> Result<(), AudioSinkError> {
        Ok(())
    }
    fn pause(&mut self) -> Result<(), AudioSinkError> {
        Ok(())
    }
    fn stop(&mut self) -> Result<(), AudioSinkError> {
        Ok(())
    }
    fn get_buffer_latency(&self) -> Duration {
        Duration::from_millis(150)
    }
}

fn spawn_test_consumer(mut consumer: Consumer<f32>, clock: Arc<OutputClock>, sample_rate: u32) {
    std::thread::spawn(move || {
        let mut last = Instant::now();
        let mut frac = 0.0f64;
        while !consumer.is_abandoned() {
            let now = Instant::now();
            let dt = now.duration_since(last).as_secs_f64();
            last = now;

            let popped = clock.total_popped();
            let flush_until = clock.flush_until();
            if popped < flush_until {
                let stale = (flush_until - popped) as usize;
                let drop_n = stale.min(consumer.slots()) / 2 * 2;
                if drop_n > 0 {
                    if let Ok(chunk) = consumer.read_chunk(drop_n) {
                        chunk.commit_all();
                    }
                    clock.advance_popped(drop_n);
                }
            }

            frac += dt * (sample_rate as f64 * 2.0);
            let due_frames = (frac as usize) / 2;
            frac -= (due_frames * 2) as f64;
            let frames = (consumer.slots() / 2).min(due_frames);
            let mut newly_popped = 0;
            for _ in 0..frames {
                let Some(gain) = clock.next_frame_gain() else {
                    frac = 0.0;
                    break;
                };
                for _ in 0..2 {
                    let s = consumer.pop().unwrap_or(0.0) * gain;
                    newly_popped += 1;
                    clock.write_tap_sample(s);
                }
            }
            if newly_popped > 0 {
                clock.advance_popped(newly_popped);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    });
}

fn build_test_engine(
    resolvers: Vec<Arc<dyn StreamResolverPort>>,
    track_duration: Duration,
) -> (PlayerEngine, Arc<EventBus>, Arc<OutputClock>) {
    let sample_rate = 48_000;
    let (producer, consumer) = RingBuffer::new(14_400);
    let clock = Arc::new(OutputClock::new(sample_rate, 2));
    spawn_test_consumer(consumer, Arc::clone(&clock), sample_rate);
    let (bus, cmd_rx) = EventBus::new(64);
    let bus = Arc::new(bus);

    let engine = PlayerEngine::new_with_config(PlayerEngineConfig {
        sample_rate,
        device_name: "TestSink".into(),
        available_devices: Vec::new(),
        audio_sink: Box::new(DummySink),
        producer,
        clock: Arc::clone(&clock),
        decoder_factory: Arc::new(MockDecoderFactory {
            default_rate: 48_000,
            default_duration: track_duration,
        }),
        resolvers,
        device_switcher: None,
        runtime_handle: default_runtime_handle(),
        event_bus: Some(Arc::clone(&bus)),
        cmd_rx: Some(cmd_rx),
        resolve_timeout: DEFAULT_RESOLVE_TIMEOUT,
    })
    .expect("engine init");

    (engine, bus, clock)
}

fn local_track(name: &str) -> Track {
    Track::new(
        AudioSource::LocalFile {
            path: format!("/virtual/{name}.wav"),
        },
        TrackMetadata::new(name, "TestArtist"),
    )
}

#[test]
fn player_lifecycle_state_transitions_and_clean_drop() {
    let (engine, _bus, _clock) = build_test_engine(Vec::new(), Duration::from_secs(10));
    assert_eq!(engine.playback_state(), PlaybackState::Stopped);

    engine.play(local_track("song_48000"));
    let deadline = Instant::now() + Duration::from_secs(2);
    while engine.playback_state() != PlaybackState::Playing && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(engine.playback_state(), PlaybackState::Playing);

    engine.pause();
    assert_eq!(engine.playback_state(), PlaybackState::Paused);

    engine.resume();
    assert_eq!(engine.playback_state(), PlaybackState::Playing);

    engine.stop();
    assert_eq!(engine.playback_state(), PlaybackState::Stopped);
}

#[test]
fn transport_instant_pause_and_seek_epoch_flush() {
    let (engine, _bus, clock) = build_test_engine(Vec::new(), Duration::from_secs(30));
    engine.play(local_track("song_48000"));
    std::thread::sleep(Duration::from_millis(80));
    assert!(clock.total_pushed() > 0);

    // Pause takes effect immediately and freezes clock consumption after the 5ms fade
    let t0 = Instant::now();
    engine.pause();
    assert!(t0.elapsed() < Duration::from_millis(20));
    std::thread::sleep(Duration::from_millis(20));
    let popped_paused = clock.total_popped();
    std::thread::sleep(Duration::from_millis(40));
    assert_eq!(
        clock.total_popped(),
        popped_paused,
        "output consumption must freeze while paused"
    );

    // Resume continues seamlessly and seek flushes stale queued frames via epoch bump
    engine.resume();
    std::thread::sleep(Duration::from_millis(40));
    let epoch_before = clock.epoch();
    let target = Duration::from_secs(12);
    engine.seek(target);
    assert!(clock.epoch() > epoch_before);

    std::thread::sleep(Duration::from_millis(40));
    let pos = *engine.elapsed.lock().unwrap();
    assert!(
        pos >= target && pos < target + Duration::from_millis(500),
        "expected position near {target:?}, got {pos:?}"
    );
}

#[test]
fn position_accurate_across_44k1_48k_96k_and_freezes_on_pause() {
    for track_name in ["song_44100", "song_48000", "song_96000"] {
        let (engine, _bus, _clock) = build_test_engine(Vec::new(), Duration::from_secs(20));
        engine.play(local_track(track_name));
        std::thread::sleep(Duration::from_millis(180));
        engine.pause();
        std::thread::sleep(Duration::from_millis(15));
        let frozen_a = *engine.elapsed.lock().unwrap();
        std::thread::sleep(Duration::from_millis(40));
        let frozen_b = *engine.elapsed.lock().unwrap();
        assert_eq!(frozen_a, frozen_b, "elapsed must not advance while paused");
        assert!(
            frozen_a >= Duration::from_millis(30) && frozen_a <= Duration::from_millis(500),
            "unexpected elapsed {frozen_a:?} for {track_name}"
        );
    }
}

struct SlowOrFlakyResolver {
    delay: Duration,
    failures_remaining: AtomicUsize,
    calls: AtomicUsize,
}

#[async_trait]
impl StreamResolverPort for SlowOrFlakyResolver {
    fn handles_source(&self, source: &AudioSource) -> bool {
        matches!(source, AudioSource::DirectHttp { .. })
    }

    async fn resolve_stream(
        &self,
        _source: &AudioSource,
    ) -> Result<Box<dyn AudioSourceStream>, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        let prev = self.failures_remaining.load(Ordering::SeqCst);
        if prev > 0 {
            self.failures_remaining.fetch_sub(1, Ordering::SeqCst);
            return Err(ProviderError::Network("transient glitch".into()));
        }
        Ok(Box::new(Cursor::new(vec![0u8; 64])))
    }
}

#[test]
fn preload_non_blocking_resolution_cancellation_and_retry() {
    // 1. Slow stream resolution must never block transport commands (< 5ms response)
    let slow_resolver = Arc::new(SlowOrFlakyResolver {
        delay: Duration::from_millis(500),
        failures_remaining: AtomicUsize::new(0),
        calls: AtomicUsize::new(0),
    });
    let (engine, _bus, _clock) =
        build_test_engine(vec![slow_resolver.clone()], Duration::from_secs(4));

    let remote_track = Track::new(
        AudioSource::DirectHttp {
            url: "https://example.com/slow.ogg".into(),
        },
        TrackMetadata::new("SlowStream", "NetArtist"),
    );
    let t0 = Instant::now();
    engine.play(remote_track);
    assert!(
        t0.elapsed() < Duration::from_millis(10),
        "play() must not block on stream resolution"
    );

    // 2. Skipping to a local track while remote stream is still resolving cancels the slow load
    std::thread::sleep(Duration::from_millis(30));
    engine.play(local_track("fast_48000"));
    std::thread::sleep(Duration::from_millis(60));
    assert_eq!(
        engine
            .current_track
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .title,
        "fast_48000"
    );

    // 3. Lookahead preload retries transient network errors with exponential backoff
    let flaky_resolver = Arc::new(SlowOrFlakyResolver {
        delay: Duration::from_millis(5),
        failures_remaining: AtomicUsize::new(2),
        calls: AtomicUsize::new(0),
    });
    let (engine2, _bus2, _clock2) =
        build_test_engine(vec![flaky_resolver.clone()], Duration::from_secs(2));
    let next_remote = Track::new(
        AudioSource::DirectHttp {
            url: "https://example.com/next.ogg".into(),
        },
        TrackMetadata::new("NextStream", "NetArtist"),
    );
    engine2.set_next_track(Some(next_remote));
    engine2.play(local_track("short_48000"));
    let deadline = Instant::now() + Duration::from_secs(3);
    while flaky_resolver.calls.load(Ordering::SeqCst) < 3 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        flaky_resolver.calls.load(Ordering::SeqCst) >= 3,
        "preload should retry transient network failure and succeed on 3rd attempt"
    );
}

#[tokio::test]
async fn queue_commands_repeat_shuffle_and_enqueue_existing() {
    let (engine, bus, _clock) = build_test_engine(Vec::new(), Duration::from_secs(20));
    let mut events = bus.subscribe();

    let t1 = local_track("track_one");
    let t2 = local_track("track_two");
    let t3_catalog = local_track("catalog_only");
    let t3_id = t3_catalog.id;

    engine.add_to_playlist(t1.clone()).await.unwrap();
    engine.add_to_playlist(t2.clone()).await.unwrap();
    engine.register_known_tracks([t3_catalog.clone()]);

    engine.play(t1.clone());
    tokio::time::sleep(Duration::from_millis(40)).await;

    // Next command advances to t2
    engine.dispatch_command(PlayerCommand::Next);
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert_eq!(
        engine.current_track.lock().unwrap().as_ref().unwrap().id,
        t2.id
    );

    // Previous command when elapsed <= 3s returns to t1
    engine.dispatch_command(PlayerCommand::Previous);
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert_eq!(
        engine.current_track.lock().unwrap().as_ref().unwrap().id,
        t1.id
    );

    // Previous command when elapsed > 3s restarts t1 at 0:00
    engine.seek(Duration::from_secs(8));
    tokio::time::sleep(Duration::from_millis(30)).await;
    engine.dispatch_command(PlayerCommand::Previous);
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert_eq!(
        engine.current_track.lock().unwrap().as_ref().unwrap().id,
        t1.id
    );
    assert!(*engine.elapsed.lock().unwrap() < Duration::from_secs(2));

    // EnqueueExisting adds known catalog track by ID
    engine.dispatch_command(PlayerCommand::EnqueueExisting(t3_id));
    assert_eq!(engine.get_playlist().await.len(), 3);

    // Repeat and Shuffle modes
    engine.set_repeat_mode(RepeatMode::RepeatAll);
    assert_eq!(engine.repeat_mode(), RepeatMode::RepeatAll);
    engine.set_shuffle_mode(ShuffleMode::SmartBalanced { artist_spacing: 1 });
    assert_eq!(
        engine.shuffle_mode(),
        ShuffleMode::SmartBalanced { artist_spacing: 1 }
    );

    // Verify TrackStarted event was broadcast
    let mut saw_started = false;
    while let Ok(evt) = events.try_recv() {
        if matches!(evt, PlayerEvent::TrackStarted(_)) {
            saw_started = true;
            break;
        }
    }
    assert!(saw_started, "expected TrackStarted event on EventBus");
}
