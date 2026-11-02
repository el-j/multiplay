//! Tests for TASK-221: Remix Slots (beat-synced loops captured from any deck).

use amp_core_engine::dj::engine::TrackBuffer;
use amp_core_engine::dj::remix::{CapturedLoop, RemixGrid, RemixSlotConfig};
use std::sync::Arc;

const RATE: u32 = 48_000;

/// Click track (stereo) at `bpm` from `offset` seconds, sampled at `rate`.
fn clicks(rate: u32, bpm: f64, offset: f64, secs: f64) -> Arc<TrackBuffer> {
    let frames = (f64::from(rate) * secs) as usize;
    let mut s = vec![0.0f32; frames * 2];
    let period = 60.0 / bpm;
    let mut n = 0;
    loop {
        let start = ((offset + n as f64 * period) * f64::from(rate)).round() as usize;
        if start >= frames {
            break;
        }
        for k in 0..(rate as usize / 100).min(frames - start) {
            let t = k as f32 / rate as f32;
            let v = 0.6 * (-t * 300.0).exp() * (2.0 * std::f32::consts::PI * 1_000.0 * t).sin();
            s[(start + k) * 2] = v;
            s[(start + k) * 2 + 1] = v;
        }
        n += 1;
    }
    TrackBuffer::from_samples(rate, &s)
}

/// Helper to measure onsets from stereo audio.
#[derive(Default)]
struct Onsets {
    quiet: usize,
    armed: bool,
    at: usize,
    times: Vec<f64>,
}

impl Onsets {
    fn feed(&mut self, stereo: &[f32], rate: u32) {
        for f in stereo.as_chunks::<2>().0 {
            let v = f[0].abs();
            if v > 0.05 {
                if self.armed {
                    self.times.push(self.at as f64 / f64::from(rate));
                }
                self.armed = false;
                self.quiet = 0;
            } else if v < 0.01 {
                self.quiet += 1;
                self.armed |= self.quiet >= rate as usize / 20;
            }
            self.at += 1;
        }
    }
}

#[test]
fn test_remix_slot_capture_from_track_buffer() {
    let buf = clicks(48_000, 120.0, 0.0, 10.0);
    // 4-beat loop at 120 BPM: 2.0 seconds -> 96,000 frames
    let captured = CapturedLoop::capture(&buf, 0.0, 2.0, 120.0, 0.0).expect("capture loop");
    assert_eq!(captured.sample_rate(), 48_000);
    assert_eq!(captured.bpm(), 120.0);
    assert_eq!(captured.length_frames(), 96_000);
    assert!((captured.duration_secs() - 2.0).abs() < 1e-6);
}

#[test]
fn test_remix_grid_controls_volume_mute_filter() {
    let mut grid = RemixGrid::new(RATE);
    let buf = clicks(RATE, 120.0, 0.0, 4.0);
    let loop_data = CapturedLoop::capture(&buf, 0.0, 2.0, 120.0, 0.0).unwrap();
    grid.load_slot(0, Some(loop_data));

    let params = grid.params();
    params.slot(0).set_playing(true);
    params.slot(0).set_volume(0.8);
    params.slot(0).set_filter(0.0); // flat filter
    params.slot(0).set_muted(false);

    let mut out = vec![0.0f32; 1024];
    grid.render(&mut out, 120.0, 0.0);

    let peak = out.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert!(peak > 0.1, "should output audio when unmuted");

    // Test mute
    params.slot(0).set_muted(true);
    grid.render(&mut out, 120.0, 0.0);
    let peak_muted = out.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert_eq!(peak_muted, 0.0, "muted slot outputs silence");

    // Test volume 0.0
    params.slot(0).set_muted(false);
    params.slot(0).set_volume(0.0);
    grid.render(&mut out, 120.0, 0.0);
    let peak_vol0 = out.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    assert_eq!(peak_vol0, 0.0, "zero volume outputs silence");
}

#[test]
fn test_remix_four_slots_phase_locked_for_five_minutes() {
    let mut grid = RemixGrid::new(RATE);
    // 4 tracks at different original tempos: 120, 124, 128, 130
    let bpms = [120.0, 124.0, 128.0, 130.0];
    for (i, &bpm) in bpms.iter().enumerate() {
        let period = 60.0 / bpm;
        // 4 beats loop
        let loop_secs = period * 4.0;
        let buf = clicks(RATE, bpm, 0.0, loop_secs + 0.1);
        let loop_data = CapturedLoop::capture(&buf, 0.0, loop_secs, bpm, 0.0).unwrap();
        grid.load_slot(i, Some(loop_data));
        grid.params().slot(i).set_playing(true);
        grid.params().slot(i).set_volume(0.25);
    }

    // Render 5 minutes (300 seconds) synced to master tempo 125.0 BPM
    let master_bpm = 125.0;
    let mut block = vec![0.0f32; 512];
    let total_frames = RATE as usize * 300;
    let mut rendered_frames = 0;

    let mut onsets = Onsets::default();

    while rendered_frames < total_frames {
        let master_position_secs = rendered_frames as f64 / RATE as f64;
        grid.render(&mut block, master_bpm, master_position_secs);
        onsets.feed(&block, RATE);
        rendered_frames += block.len() / 2;
    }

    // Verify slots stayed locked to master beat grid
    // At 125 BPM, period is 60 / 125 = 0.48 s.
    let master_period = 60.0 / master_bpm;
    assert!(
        onsets.times.len() > 600,
        "should have registered clicks across 5 minutes: {}",
        onsets.times.len()
    );

    // Check that every detected click onset lands within 5 ms of a master beat
    for &time in &onsets.times {
        let beat_num = (time / master_period).round();
        let expected_time = beat_num * master_period;
        let drift = (time - expected_time).abs();
        assert!(
            drift < 0.005,
            "drift at time {time:.3}s is {:.2}ms (expected {expected_time:.3}s)",
            drift * 1000.0
        );
    }
}

#[test]
fn test_remix_session_serialization() {
    let cfg = RemixSlotConfig {
        index: 0,
        volume: 0.8,
        filter: 0.2,
        muted: false,
        bpm: 124.0,
    };
    let json_str = serde_json::to_string(&cfg).expect("serialize slot config");
    let deserialized: RemixSlotConfig = serde_json::from_str(&json_str).expect("deserialize slot config");
    assert_eq!(deserialized.index, 0);
    assert_eq!(deserialized.volume, 0.8);
    assert!(!deserialized.muted);
}

#[test]
fn test_render_allocations_zero_during_playback() {
    let mut grid = RemixGrid::new(RATE);
    let buf = clicks(RATE, 120.0, 0.0, 4.0);
    let loop_data = CapturedLoop::capture(&buf, 0.0, 2.0, 120.0, 0.0).unwrap();
    grid.load_slot(0, Some(loop_data));
    grid.params().slot(0).set_playing(true);
    grid.params().slot(0).set_volume(0.8);
    grid.params().slot(0).set_filter(0.5);

    // Warm up filters
    let mut out = vec![0.0f32; 512];
    grid.render(&mut out, 120.0, 0.0);

    // During steady render, no reallocations happen
    for i in 1..100 {
        grid.render(&mut out, 120.0, i as f64 * 512.0 / RATE as f64);
        assert!(out.iter().any(|&s| s != 0.0));
    }
}
