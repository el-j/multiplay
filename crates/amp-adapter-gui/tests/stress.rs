//! Stress and failure injection (TASK-186): the real engine with the real-time null output,
//! under rapid skipping, a trickling server, a dropped connection and a resolver that never
//! answers. Each must end in a playing, stopped or clearly failed state, never a hang or a leak.
//! The long soak is opt-in: `SOAK_MINUTES=60 cargo test --release -p amp-adapter-gui --test
//! stress -- --ignored --nocapture`.

#![forbid(unsafe_code)]

use amp_adapter_cpal::CpalAudioSink;
use amp_adapter_decoder::SymphoniaDecoderFactory;
use amp_adapter_http::HttpStreamResolver;
use amp_core_engine::player::{default_runtime_handle, OutputDeviceInfo, PlayerEngineConfig};
use amp_core_engine::{EventBus, PlayerEngine, PlayerEvent};
use amp_domain::{AudioSource, PlaybackState, Track, TrackMetadata};
use amp_ports::{async_trait, AudioSourceStream, ProviderError, StreamResolverPort};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast::error::TryRecvError;

fn engine(
    resolvers: Vec<Arc<dyn StreamResolverPort>>,
    resolve_timeout: Duration,
) -> (PlayerEngine, Arc<EventBus>) {
    let (sink, producer) = CpalAudioSink::new_null(48_000, 2);
    let bus = Arc::new(EventBus::new(4096).0);
    let engine = PlayerEngine::new_with_config(PlayerEngineConfig {
        sample_rate: sink.sample_rate(),
        device_name: sink.device_name().to_string(),
        available_devices: vec![OutputDeviceInfo {
            name: "null".into(),
            is_default: true,
        }],
        clock: sink.clock(),
        audio_sink: Box::new(sink),
        producer,
        decoder_factory: Arc::new(SymphoniaDecoderFactory),
        resolvers,
        device_switcher: None,
        runtime_handle: default_runtime_handle(),
        event_bus: Some(Arc::clone(&bus)),
        cmd_rx: None,
        resolve_timeout,
    })
    .expect("engine");
    (engine, bus)
}

fn local(file: &str) -> Track {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../amp-adapter-decoder/tests/fixtures")
        .join(file);
    Track::new(
        AudioSource::LocalFile {
            path: path.to_string_lossy().into_owned(),
        },
        TrackMetadata::new(file, "stress"),
    )
}

fn stream(url: String) -> Track {
    Track::new(
        AudioSource::DirectHttp { url },
        TrackMetadata::new("stream", "stress"),
    )
}

/// Threads of this process (`None` where the OS does not say cheaply).
fn threads() -> Option<usize> {
    if cfg!(target_os = "linux") {
        return std::fs::read_dir("/proc/self/task")
            .ok()
            .map(Iterator::count);
    }
    if cfg!(target_os = "macos") {
        let out = std::process::Command::new("ps")
            .args(["-M", "-p", &std::process::id().to_string()])
            .output()
            .ok()?;
        return Some(
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .count()
                .saturating_sub(1),
        );
    }
    None
}

/// Resident memory of this process in MiB (`None` on Windows).
fn rss_mib() -> Option<f64> {
    if cfg!(target_os = "linux") {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let kib: f64 = status
            .lines()
            .find_map(|l| l.strip_prefix("VmRSS:"))?
            .split_whitespace()
            .next()?
            .parse()
            .ok()?;
        return Some(kib / 1024.0);
    }
    if cfg!(target_os = "macos") {
        let out = std::process::Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()?;
        let kib: f64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
        return Some(kib / 1024.0);
    }
    None
}

fn wait_until(what: &str, limit: Duration, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + limit;
    while !done() {
        assert!(
            Instant::now() < deadline,
            "timed out after {limit:?} waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// An HTTP server on 127.0.0.1 that answers once with `body`, `chunk` bytes every `every`, then
/// closes (dropping the connection early when `cut_after` is set).
fn serve(body: Vec<u8>, chunk: usize, every: Duration, cut_after: Option<usize>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for conn in listener.incoming().take(4) {
            let Ok(mut conn) = conn else { continue };
            let mut req = [0u8; 2048];
            let _ = conn.read(&mut req);
            let head = "HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\nConnection: close\r\n\r\n";
            if conn.write_all(head.as_bytes()).is_err() {
                continue;
            }
            let end = cut_after.unwrap_or(body.len()).min(body.len());
            for part in body[..end].chunks(chunk) {
                if conn.write_all(part).is_err() {
                    break;
                }
                std::thread::sleep(every);
            }
        }
    });
    format!("http://{addr}/stream.mp3")
}

fn http() -> Vec<Arc<dyn StreamResolverPort>> {
    vec![Arc::new(HttpStreamResolver::new().expect("http resolver"))]
}

fn saw_error(events: &mut tokio::sync::broadcast::Receiver<PlayerEvent>) -> bool {
    loop {
        match events.try_recv() {
            Ok(PlayerEvent::Error { .. }) => return true,
            Ok(_) | Err(TryRecvError::Lagged(_)) => {}
            Err(_) => return false,
        }
    }
}

#[test]
fn a_thousand_rapid_skips_leave_no_threads_or_memory_behind() {
    let (engine, _bus) = engine(Vec::new(), Duration::from_secs(5));
    let files = [
        "wav_16.wav",
        "mp3_cbr_128.mp3",
        "flac_16_44k.flac",
        "aac.m4a",
        "opus_stereo.opus",
    ];
    engine.play(local(files[0]));
    wait_until("first track", Duration::from_secs(5), || {
        engine.stream_info().is_some()
    });
    std::thread::sleep(Duration::from_millis(500));
    let (threads_before, rss_before) = (threads(), rss_mib());

    for i in 0..1_000 {
        engine.play(local(files[i % files.len()]));
        if i % 50 == 0 {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    let last = files[999 % files.len()];
    wait_until("the last skip to play", Duration::from_secs(10), || {
        engine
            .current_track
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|t| t.metadata.title == last)
            && engine.stream_info().is_some()
    });
    std::thread::sleep(Duration::from_secs(2));

    if let (Some(before), Some(after)) = (threads_before, threads()) {
        assert!(
            after <= before + 4,
            "threads: {before} before, {after} after 1,000 skips"
        );
    }
    if let (Some(before), Some(after)) = (rss_before, rss_mib()) {
        assert!(
            after - before < 30.0,
            "resident memory grew {:.1} MiB",
            after - before
        );
    }
}

#[test]
fn a_trickling_server_buffers_and_stop_still_answers_at_once() {
    let mp3 = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bench/bench_20s.mp3"),
    )
    .unwrap();
    // 1 KB/s against a 256 kbps file: a quarter of real time at best.
    let url = serve(mp3, 128, Duration::from_millis(125), None);
    let (engine, bus) = engine(http(), Duration::from_secs(4));
    let mut events = bus.subscribe();
    engine.play(stream(url));
    let mut buffering = false;
    let mut failed = false;
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !buffering && !failed {
        buffering |= matches!(engine.playback_state(), PlaybackState::Buffering { .. });
        failed |= saw_error(&mut events);
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        buffering || failed,
        "the slow stream is shown as buffering or as an error"
    );
    let started = Instant::now();
    engine.stop();
    wait_until("stop", Duration::from_secs(2), || {
        engine.playback_state() == PlaybackState::Stopped
    });
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "stop took {:?}",
        started.elapsed()
    );
}

#[test]
fn a_dropped_connection_ends_the_track_instead_of_hanging() {
    let mp3 = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bench/bench_20s.mp3"),
    )
    .unwrap();
    // About 1 s of audio, as fast as possible, then the server hangs up.
    let url = serve(mp3, 4096, Duration::ZERO, Some(32_000));
    let (engine, bus) = engine(http(), Duration::from_secs(5));
    let mut events = bus.subscribe();
    engine.play(stream(url));
    let mut failed = false;
    wait_until(
        "the cut stream to finish or fail",
        Duration::from_secs(20),
        || {
            failed |= saw_error(&mut events);
            failed
                || engine.playback_state() == PlaybackState::Stopped
                    && engine
                        .track_finished
                        .load(std::sync::atomic::Ordering::SeqCst)
        },
    );
}

struct NeverAnswers;

#[async_trait]
impl StreamResolverPort for NeverAnswers {
    fn handles_source(&self, source: &AudioSource) -> bool {
        matches!(source, AudioSource::YouTube { .. })
    }
    async fn resolve_stream(
        &self,
        _source: &AudioSource,
    ) -> Result<Box<dyn AudioSourceStream>, ProviderError> {
        std::future::pending::<()>().await;
        unreachable!()
    }
}

#[test]
fn a_resolver_that_never_answers_times_out_with_an_error() {
    let (engine, bus) = engine(vec![Arc::new(NeverAnswers)], Duration::from_millis(800));
    let mut events = bus.subscribe();
    engine.play(Track::new(
        AudioSource::YouTube {
            video_id: "dQw4w9WgXcQ".into(),
        },
        TrackMetadata::new("hangs", "stress"),
    ));
    let mut failed = false;
    wait_until("the resolve timeout", Duration::from_secs(5), || {
        failed |= saw_error(&mut events);
        failed
    });
    // And the engine still plays the next thing.
    engine.play(local("wav_16.wav"));
    wait_until("the next track", Duration::from_secs(5), || {
        engine.stream_info().is_some()
    });
}

#[test]
#[ignore = "long soak; set SOAK_MINUTES and run with --ignored in release"]
fn soak_memory_stays_flat() {
    let minutes: u64 = std::env::var("SOAK_MINUTES")
        .ok()
        .and_then(|m| m.parse().ok())
        .unwrap_or(10);
    let (engine, _bus) = engine(Vec::new(), Duration::from_secs(5));
    let bench = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bench");
    let tracks: Vec<Track> = ["bench_20s.flac", "bench_20s.mp3"]
        .iter()
        .map(|f| {
            Track::new(
                AudioSource::LocalFile {
                    path: bench.join(f).to_string_lossy().into_owned(),
                },
                TrackMetadata::new(*f, "soak"),
            )
        })
        .collect();
    let start = Instant::now();
    let mut samples: Vec<(f64, f64)> = Vec::new();
    let mut next = 0;
    while start.elapsed() < Duration::from_secs(minutes * 60) {
        if engine.playback_state() != PlaybackState::Playing {
            engine.play(tracks[next % tracks.len()].clone());
            next += 1;
        }
        if let Some(rss) = rss_mib() {
            let at = start.elapsed().as_secs_f64() / 3600.0;
            if samples.last().is_none_or(|(t, _)| at - t >= 1.0 / 60.0) {
                samples.push((at, rss));
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    // Growth per hour after a 2-minute warm-up, by least squares.
    let warm: Vec<_> = samples
        .iter()
        .filter(|(t, _)| *t > 2.0 / 60.0)
        .copied()
        .collect();
    if warm.len() < 3 {
        return;
    }
    let n = warm.len() as f64;
    let (sx, sy) = warm
        .iter()
        .fold((0.0, 0.0), |(a, b), (x, y)| (a + x, b + y));
    let (mx, my) = (sx / n, sy / n);
    let slope = warm.iter().map(|(x, y)| (x - mx) * (y - my)).sum::<f64>()
        / warm.iter().map(|(x, _)| (x - mx).powi(2)).sum::<f64>();
    println!(
        "soak: {minutes} min, {} tracks, RSS {:.1} → {:.1} MiB, growth {slope:.2} MiB/h",
        next,
        warm[0].1,
        warm[warm.len() - 1].1
    );
    assert!(slope < 5.0, "memory grows {slope:.2} MiB per hour");
}
