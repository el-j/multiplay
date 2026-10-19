//! TASK-213: prelisten through the real app path. In split mode the sound card's left channel
//! carries the master mix and the right channel the headphone cue, with deck B silent on master
//! but heard in the headphones.

use amp_adapter_gui::audio_driver::AudioPlaybackDriver;
use amp_adapter_gui::dj_session::DjSession;
use amp_core_engine::dj::engine::CueRouting;
use amp_domain::{AudioSource, Track, TrackMetadata};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const RATE: u32 = 48_000;

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
        let v = (0.4 * (2.0 * std::f32::consts::PI * hz * i as f32 / RATE as f32).sin() * 32_767.0)
            as i16;
        w.extend_from_slice(&v.to_le_bytes());
        w.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::write(path, w).unwrap();
}

struct Dir(PathBuf);
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn hz(s: &[f32]) -> f32 {
    s.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f32 * RATE as f32 / s.len() as f32
}

#[test]
fn dj_split_routing_puts_master_left_and_prelisten_right() {
    let dir =
        Dir(std::env::temp_dir().join(format!("multiplay-dj-route-{}", uuid::Uuid::new_v4())));
    std::fs::create_dir_all(&dir.0).unwrap();
    let mut tracks = Vec::new();
    for (name, f) in [("a.wav", 300.0f32), ("b.wav", 900.0)] {
        let path = dir.0.join(name);
        write_sine_wav(&path, f, 10.0);
        tracks.push(Track::new(
            AudioSource::LocalFile {
                path: path.to_string_lossy().into_owned(),
            },
            TrackMetadata::new(name, "Test"),
        ));
    }
    let (driver, captured) = AudioPlaybackDriver::new_capture().expect("capture driver");
    let mut dj = DjSession::new(&driver, None);
    dj.enter(&driver);
    let start = Instant::now();
    while !driver.dj_active() {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "DJ mode did not start"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let a = dj.load(&driver, 0, tracks[0].clone());
    let b = dj.load(&driver, 1, tracks[1].clone());
    while !(a.is_complete() && b.is_complete()) {
        std::thread::sleep(Duration::from_millis(5));
    }
    let p = dj.params();
    p.set_cue_routing(CueRouting::Split);
    p.set_cue_volume(1.0);
    p.set_crossfader(-1.0);
    p.deck(1).set_fader(0.0);
    p.deck(1).set_pfl(true);
    p.deck(0).set_playing(true);
    p.deck(1).set_playing(true);
    let from = captured.len();
    while captured.len() < from + RATE as usize * 2 * 2 {
        assert!(start.elapsed() < Duration::from_secs(15), "no audio");
        std::thread::sleep(Duration::from_millis(10));
    }
    driver.stop();
    let audio = captured.samples();
    // Skip the first half second (queue left from before the switch, gain ramps).
    let body = &audio[from + RATE as usize..from + RATE as usize * 4];
    let left: Vec<f32> = body.iter().step_by(2).copied().collect();
    let right: Vec<f32> = body.iter().skip(1).step_by(2).copied().collect();
    assert!(
        (hz(&left) - 300.0).abs() < 10.0,
        "left = master (deck A): {} Hz",
        hz(&left)
    );
    assert!(
        (hz(&right) - 900.0).abs() < 10.0,
        "right = headphones (deck B): {} Hz",
        hz(&right)
    );
}
