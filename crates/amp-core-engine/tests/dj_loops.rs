//! Loops and cues (TASK-206), checked on the rendered audio: beat loops last exactly their
//! beats at the deck's tempo, loop seams are click-free, A-B loops quantize, loop roll resumes in
//! phase, and hot cues keep the beat phase when jumping.

use amp_core_engine::dj::engine::{DjControl, DjEngine, TrackBuffer};
use std::sync::Arc;

const RATE: u32 = 48_000;

fn track(rate: u32, secs: f64, f: impl Fn(f64) -> f32) -> Arc<TrackBuffer> {
    let frames = (f64::from(rate) * secs) as usize;
    let s: Vec<f32> = (0..frames)
        .flat_map(|i| {
            let v = f(i as f64 / f64::from(rate));
            [v, v]
        })
        .collect();
    TrackBuffer::from_samples(rate, &s)
}

/// A short click on every beat at 120 BPM.
fn clicks_120(secs: f64) -> Arc<TrackBuffer> {
    track(44_100, secs, |t| {
        let since = t.rem_euclid(0.5);
        if since < 0.005 {
            (0.6 * (2.0 * std::f64::consts::PI * 1_000.0 * since).sin()) as f32
        } else {
            0.0
        }
    })
}

fn deck_with(t: Arc<TrackBuffer>) -> (DjEngine, DjControl) {
    let (engine, mut control) = DjEngine::new(RATE, 2);
    control.load(0, t);
    let p = control.params();
    p.set_crossfader(-1.0);
    p.deck(0).set_grid(120.0, 0.0);
    p.deck(0).set_quantize(true);
    p.deck(0).set_playing(true);
    (engine, control)
}

fn render(engine: &mut DjEngine, secs: f64) -> Vec<f32> {
    let n = (secs * f64::from(RATE)) as usize * 2;
    let (mut m, mut c) = (vec![0.0; n], vec![0.0; n]);
    engine.render(&mut m, &mut c);
    m
}

fn onsets(stereo: &[f32]) -> Vec<f64> {
    let mut out = Vec::new();
    let mut quiet = RATE as usize;
    for (i, f) in stereo.as_chunks::<2>().0.iter().enumerate() {
        if f[0].abs() > 0.05 {
            if quiet > RATE as usize / 20 {
                out.push(i as f64 / f64::from(RATE));
            }
            quiet = 0;
        } else {
            quiet += 1;
        }
    }
    out
}

fn pos(control: &DjControl) -> f64 {
    control.status().deck(0).position_secs()
}

#[test]
fn dj_loops_beat_loop_lasts_its_beats_at_the_deck_tempo() {
    for beats in [1.0, 4.0] {
        let (mut engine, mut control) = deck_with(clicks_120(30.0));
        control.params().deck(0).set_pitch_percent(5.0);
        let _ = render(&mut engine, 1.3);
        control.beat_loop(0, beats, false);
        let out = render(&mut engine, 6.0);
        let on = onsets(&out);
        assert!(
            on.len() > 8,
            "clicks keep coming while looping: {}",
            on.len()
        );
        let want = 0.5 / 1.05;
        for w in on.windows(2).skip(1) {
            assert!(
                ((w[1] - w[0]) - want).abs() < 0.001,
                "{beats}-beat loop interval {}",
                w[1] - w[0]
            );
        }
        let (a, b) = control.status().deck(0).loop_range().expect("loop on");
        assert!((b - a - beats * 0.5).abs() < 1e-6, "{a}..{b}");
        assert!(pos(&control) >= a && pos(&control) < b);
    }
}

#[test]
fn dj_loops_seam_is_click_free() {
    // A smooth 100 Hz tone looped between two points that cut it at different phases.
    let tone = track(48_000, 10.0, |t| {
        (0.5 * (2.0 * std::f64::consts::PI * 100.0 * t).sin()) as f32
    });
    let (mut engine, mut control) = deck_with(tone);
    control.params().deck(0).set_quantize(false);
    let _ = render(&mut engine, 1.05);
    control.loop_between(0, 1.0, 1.2345);
    let out = render(&mut engine, 3.0);
    let left: Vec<f32> = out.iter().step_by(2).copied().collect();
    let worst = left
        .windows(3)
        .map(|w| (w[2] - 2.0 * w[1] + w[0]).abs())
        .fold(0.0f32, f32::max);
    assert!(worst < 0.001, "loop seam step {worst} (> -60 dBFS)");
    assert!(control.status().deck(0).loop_range().is_some());
}

#[test]
fn dj_loops_ab_sets_a_then_b_on_the_beat_grid() {
    let (mut engine, mut control) = deck_with(clicks_120(30.0));
    let _ = render(&mut engine, 1.1);
    control.ab(0);
    assert!(
        control.status().deck(0).loop_range().is_none(),
        "only A is set"
    );
    let _ = render(&mut engine, 1.2);
    control.ab(0);
    let _ = render(&mut engine, 0.01);
    assert_eq!(control.status().deck(0).loop_range(), Some((1.0, 2.5)));
    let _ = render(&mut engine, 4.0);
    let p = pos(&control);
    assert!((1.0..2.5).contains(&p), "looping between A and B: {p}");
}

#[test]
fn dj_loops_halve_double_exit_and_reloop() {
    let (mut engine, mut control) = deck_with(clicks_120(60.0));
    let _ = render(&mut engine, 2.0);
    control.beat_loop(0, 4.0, false);
    let _ = render(&mut engine, 0.01);
    let (a, b) = control.status().deck(0).loop_range().unwrap();
    control.resize_loop(0, 0.5);
    let _ = render(&mut engine, 0.01);
    assert_eq!(
        control.status().deck(0).loop_range(),
        Some((a, a + (b - a) / 2.0))
    );
    control.resize_loop(0, 4.0);
    let _ = render(&mut engine, 0.01);
    assert_eq!(
        control.status().deck(0).loop_range(),
        Some((a, a + (b - a) * 2.0))
    );
    control.exit_loop(0);
    let _ = render(&mut engine, 6.0);
    assert!(!control.status().deck(0).loop_active());
    assert!(pos(&control) > a + (b - a) * 2.0, "plays on past the loop");
    control.reloop(0);
    let _ = render(&mut engine, 0.01);
    assert!(control.status().deck(0).loop_active());
    assert!(
        (pos(&control) - a).abs() < 0.02,
        "reloop jumps back to the loop start"
    );
}

#[test]
fn dj_loops_roll_resumes_where_the_track_would_have_been() {
    let (mut engine, mut control) = deck_with(clicks_120(30.0));
    let _ = render(&mut engine, 2.0);
    let start = pos(&control);
    control.beat_loop(0, 0.25, true);
    let _ = render(&mut engine, 1.0);
    assert!(pos(&control) < start + 0.2, "rolling in a short loop");
    control.exit_loop(0);
    let _ = render(&mut engine, 0.5);
    assert!(
        (pos(&control) - (start + 1.5)).abs() < 0.002,
        "{}",
        pos(&control)
    );
}

#[test]
fn dj_hot_cues_snap_to_the_grid_and_jumps_keep_the_beat_phase() {
    let (mut engine, mut control) = deck_with(clicks_120(30.0));
    let _ = render(&mut engine, 3.13);
    control.mark_hot_cue(0, 2);
    let _ = render(&mut engine, 0.01);
    assert_eq!(control.status().deck(0).hot_cue(2), Some(3.0));
    let _ = render(&mut engine, 4.13);
    let before = pos(&control);
    let offset = before - (before * 2.0).round() / 2.0;
    control.jump_hot_cue(0, 2);
    let _ = render(&mut engine, 0.001);
    let after = pos(&control);
    assert!(
        (after - (3.0 + offset + 0.001)).abs() < 0.001,
        "{before} -> {after}"
    );

    control.set_hot_cue(0, 5, Some(12.0));
    let _ = render(&mut engine, 0.01);
    assert_eq!(control.status().deck(0).hot_cue(5), Some(12.0));
    control.set_hot_cue(0, 5, None);
    let _ = render(&mut engine, 0.01);
    assert_eq!(control.status().deck(0).hot_cue(5), None);
}
