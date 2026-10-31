//! TASK-209: switching DJ mode on and off while a track plays is seamless. The audio that
//! actually reached the (captured) sound card has no gap, keeps its pitch and keeps its level.

use amp_adapter_gui::create_capture_engine;
use amp_adapter_gui::dj_session::DjSession;
use amp_domain::{AudioSource, Track, TrackMetadata};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const RATE: u32 = 44_100;

fn write_sine_wav(path: &Path, hz: f32, secs: f32) {
    let frames = (RATE as f32 * secs) as u32;
    let data_len = frames * 4;
    let mut w = Vec::with_capacity(44 + data_len as usize);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&2u16.to_le_bytes());
    w.extend_from_slice(&RATE.to_le_bytes());
    w.extend_from_slice(&(RATE * 4).to_le_bytes());
    w.extend_from_slice(&4u16.to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        let v = (0.5 * (2.0 * std::f32::consts::PI * hz * i as f32 / RATE as f32).sin() * 32_767.0)
            as i16;
        w.extend_from_slice(&v.to_le_bytes());
        w.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::write(path, w).unwrap();
}

fn wait_until(what: &str, secs: u64, f: impl Fn() -> bool) {
    let start = Instant::now();
    while !f() {
        assert!(
            start.elapsed() < Duration::from_secs(secs),
            "timed out waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

struct Dir(PathBuf);
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn dj_mode_toggles_mid_track_without_a_gap() {
    toggle_and_check(1_500);
}

/// Leaving right after entering (before the decks were ever heard) must be just as clean:
/// the switch in progress is cancelled or finishes first.
#[test]
fn dj_mode_toggled_twice_quickly_stays_seamless() {
    toggle_and_check(50);
}

/// Plays a tone, enters DJ mode, stays `dwell_ms`, leaves, and checks the recorded audio.
fn toggle_and_check(dwell_ms: u64) {
    let dir =
        Dir(std::env::temp_dir().join(format!("multiplay-dj-toggle-{}", uuid::Uuid::new_v4())));
    std::fs::create_dir_all(&dir.0).unwrap();
    let wav = dir.0.join("tone.wav");
    write_sine_wav(&wav, 440.0, 12.0);
    let track = Track::new(
        AudioSource::LocalFile {
            path: wav.to_string_lossy().into_owned(),
        },
        TrackMetadata::new("Tone", "Test"),
    );

    let (driver, captured) = create_capture_engine().expect("capture driver");
    let rate = driver.sample_rate() as usize;
    driver.play(track);
    wait_until("normal playback", 10, || captured.len() > rate * 2);
    std::thread::sleep(Duration::from_millis(800));

    let mut dj = DjSession::new(&driver, None);
    dj.enter(&driver);
    wait_until("the DJ engine to take over", 10, || driver.dj_active());
    std::thread::sleep(Duration::from_millis(dwell_ms));
    assert!(
        dwell_ms < 1_000 || dj.status().deck(0).position_secs() > 1.0,
        "deck A carries on mid-track"
    );

    dj.leave(&driver);
    wait_until("normal playback to take back over", 10, || {
        !driver.dj_active()
    });
    std::thread::sleep(Duration::from_millis(1_000));
    let end = captured.len();
    driver.stop();

    let audio = captured.samples();
    let left: Vec<f32> = audio[..end].iter().step_by(2).copied().collect();
    let first = left.iter().position(|v| v.abs() > 0.05).expect("sound");
    let body = &left[first..left.len() - rate / 10];

    // No gap: the tone never stays near zero for more than a couple of samples.
    let mut run = 0;
    let mut worst = 0;
    for v in body {
        run = if v.abs() < 0.01 { run + 1 } else { 0 };
        worst = worst.max(run);
    }
    assert!(worst <= 3, "silent run of {worst} samples");

    // No click: the waveform's curvature never jumps (a smooth 440 Hz tone at this level
    // bends by at most about 0.0015 per sample; a misaligned splice jumps by far more).
    let click = body
        .windows(3)
        .map(|w| (w[2] - 2.0 * w[1] + w[0]).abs())
        .fold(0.0f32, f32::max);
    assert!(click < 0.01, "click: curvature step {click}");

    // Pitch and level hold in every 100 ms window, across both switches.
    let window = rate / 10;
    for (i, w) in body.chunks_exact(window).enumerate() {
        let crossings = w.windows(2).filter(|p| p[0] < 0.0 && p[1] >= 0.0).count();
        let hz = crossings as f32 * 10.0;
        assert!((hz - 440.0).abs() <= 20.0, "window {i}: {hz} Hz");
        let peak = w.iter().fold(0.0f32, |a, v| a.max(v.abs()));
        assert!((peak - 0.48).abs() < 0.12, "window {i}: level {peak}");
    }
}
