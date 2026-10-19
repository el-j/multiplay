//! Beat matching (TASK-205), checked on the rendered audio: a synced deck's beats land within
//! 5 ms of the master's for five minutes, a synced deck starts in phase, and key lock keeps the
//! pitch within 5 cents at ±8 % tempo.

use amp_core_engine::dj::engine::{DjControl, DjEngine, TrackBuffer};
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

/// Finds click onsets in a stream of blocks (left channel): the first sample above a threshold
/// after at least 50 ms of quiet.
#[derive(Default)]
struct Onsets {
    quiet: usize,
    armed: bool,
    at: usize,
    times: Vec<f64>,
}

impl Onsets {
    fn feed(&mut self, stereo: &[f32]) {
        for f in stereo.as_chunks::<2>().0 {
            let v = f[0].abs();
            if v > 0.05 {
                if self.armed {
                    self.times.push(self.at as f64 / f64::from(RATE));
                }
                self.armed = false;
                self.quiet = 0;
            } else if v < 0.01 {
                self.quiet += 1;
                self.armed |= self.quiet >= RATE as usize / 20;
            }
            self.at += 1;
        }
    }
}

fn run(engine: &mut DjEngine, secs: f64, master: &mut Onsets, cue: &mut Onsets) {
    let mut m = vec![0.0f32; 1_024];
    let mut c = vec![0.0f32; 1_024];
    for _ in 0..(secs * f64::from(RATE) / 512.0) as usize {
        engine.render(&mut m, &mut c);
        master.feed(&m);
        cue.feed(&c);
    }
}

/// Deck A (120 BPM) is the master, heard on the cue bus only; deck B (124 BPM) follows it on
/// the master bus.
fn synced_pair(b_offset: f64, secs: f64) -> (DjEngine, DjControl) {
    let (engine, mut control) = DjEngine::new(RATE, 2);
    control.load(0, clicks(22_050, 120.0, 0.25, secs));
    control.load(1, clicks(22_050, 124.0, b_offset, secs));
    let p = control.params();
    p.deck(0).set_grid(120.0, 0.25);
    p.deck(1).set_grid(124.0, b_offset);
    p.set_master_deck(Some(0));
    p.deck(0).set_fader(0.0);
    p.deck(0).set_pfl(true);
    p.set_crossfader(1.0);
    p.deck(1).set_sync(true);
    (engine, control)
}

fn worst_gap(master: &[f64], cue: &[f64], after: f64) -> f64 {
    assert!(
        master.iter().any(|t| *t > after) && !cue.is_empty(),
        "no beats to compare (master {}, cue {})",
        master.len(),
        cue.len()
    );
    master
        .iter()
        .filter(|t| **t > after)
        .map(|t| cue.iter().map(|c| (c - t).abs()).fold(f64::MAX, f64::min))
        .fold(0.0, f64::max)
}

#[test]
fn dj_sync_keeps_beats_within_5_ms_for_five_minutes() {
    let (mut engine, control) = synced_pair(0.61, 330.0);
    let (mut on_master, mut on_cue) = (Onsets::default(), Onsets::default());
    control.params().deck(0).set_playing(true);
    run(&mut engine, 2.3, &mut on_master, &mut on_cue);
    control.params().deck(1).set_playing(true);
    run(&mut engine, 300.0, &mut on_master, &mut on_cue);
    assert!(
        on_master.times.len() > 550,
        "deck B played: {}",
        on_master.times.len()
    );
    let gap = worst_gap(&on_master.times, &on_cue.times, 3.0);
    assert!(gap < 0.005, "worst beat gap {:.2} ms", gap * 1_000.0);
    let s = control.status().deck(1);
    assert!((s.bpm() - 120.0).abs() < 0.05, "B heard at {} BPM", s.bpm());
    assert!(control.status().deck(0).is_master());
}

#[test]
fn dj_synced_deck_starts_in_phase() {
    let (mut engine, control) = synced_pair(1.37, 30.0);
    let (mut on_master, mut on_cue) = (Onsets::default(), Onsets::default());
    control.params().deck(0).set_playing(true);
    run(&mut engine, 1.1, &mut on_master, &mut on_cue);
    control.params().deck(1).set_playing(true);
    run(&mut engine, 3.0, &mut on_master, &mut on_cue);
    let gap = worst_gap(&on_master.times, &on_cue.times, 0.0);
    assert!(
        gap < 0.002,
        "first beats already aligned: {:.2} ms",
        gap * 1_000.0
    );
}

/// Measured frequency of the left channel (rising zero crossings with sub-sample timing).
fn frequency(stereo: &[f32]) -> f64 {
    let left: Vec<f64> = stereo.iter().step_by(2).map(|v| f64::from(*v)).collect();
    let crossings: Vec<f64> = left
        .windows(2)
        .enumerate()
        .filter(|(_, w)| w[0] < 0.0 && w[1] >= 0.0)
        .map(|(i, w)| i as f64 + w[0] / (w[0] - w[1]))
        .collect();
    let (first, last) = (crossings[0], crossings[crossings.len() - 1]);
    (crossings.len() - 1) as f64 * f64::from(RATE) / (last - first)
}

fn cents(f: f64, reference: f64) -> f64 {
    1_200.0 * (f / reference).log2()
}

#[test]
fn dj_key_lock_keeps_the_pitch_at_plus_and_minus_8_percent() {
    for pitch in [8.0f32, -8.0] {
        let tone: Vec<f32> = (0..RATE as usize * 12)
            .flat_map(|i| {
                let v = 0.5 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / RATE as f32).sin();
                [v, v]
            })
            .collect();
        let (mut engine, mut control) = DjEngine::new(RATE, 2);
        control.load(0, TrackBuffer::from_samples(RATE, &tone));
        let p = control.params();
        p.set_crossfader(-1.0);
        p.deck(0).set_pitch_percent(pitch);
        p.deck(0).set_key_lock(true);
        p.deck(0).set_playing(true);
        let n = RATE as usize * 2 * 4;
        let (mut m, mut c) = (vec![0.0; n], vec![0.0; n]);
        engine.render(&mut m, &mut c);
        let f = frequency(&m[RATE as usize..]);
        assert!(cents(f, 440.0).abs() < 5.0, "{pitch}%: {f:.2} Hz");
        let pos = control.status().deck(0).position_secs();
        let want = 4.0 * (1.0 + f64::from(pitch) / 100.0);
        assert!(
            (pos - want).abs() < 0.01,
            "{pitch}%: played {pos:.3}s, want {want:.3}s"
        );

        // Without key lock the same tempo change moves the pitch with it.
        p.deck(0).set_key_lock(false);
        engine.render(&mut m, &mut c);
        let f = frequency(&m[RATE as usize..]);
        let expected = 1_200.0 * (1.0 + f64::from(pitch) / 100.0).log2();
        assert!(
            (cents(f, 440.0) - expected).abs() < 5.0,
            "varispeed {f:.2} Hz"
        );
    }
}

#[test]
fn dj_nudge_bends_the_tempo_while_held() {
    let (mut engine, control) = synced_pair(0.0, 10.0);
    let p = control.params();
    p.deck(1).set_sync(false);
    p.deck(1).set_nudge(0.04);
    p.deck(1).set_playing(true);
    let (mut a, mut b) = (Onsets::default(), Onsets::default());
    run(&mut engine, 1.0, &mut a, &mut b);
    let played = a.at as f64 / f64::from(RATE);
    let s = control.status().deck(1);
    assert!(
        (s.position_secs() - played * 1.04).abs() < 1e-6,
        "{}",
        s.position_secs()
    );
    assert!((s.bpm() - 124.0 * 1.04).abs() < 0.01);
}

#[test]
fn dj_seeking_a_synced_deck_lands_on_the_beat() {
    let (mut engine, mut control) = synced_pair(0.61, 40.0);
    let (mut on_master, mut on_cue) = (Onsets::default(), Onsets::default());
    control.params().deck(0).set_playing(true);
    control.params().deck(1).set_playing(true);
    run(&mut engine, 4.0, &mut on_master, &mut on_cue);
    control.seek(1, 12.345);
    let (mut after_master, mut after_cue) = (Onsets::default(), Onsets::default());
    after_master.at = on_master.at;
    after_cue.at = on_cue.at;
    run(&mut engine, 3.0, &mut after_master, &mut after_cue);
    let gap = worst_gap(&after_master.times, &after_cue.times, 0.0);
    assert!(gap < 0.005, "after the seek: {:.2} ms", gap * 1_000.0);
}

#[test]
fn dj_followers_steer_back_onto_the_master_beat_after_it_jumps() {
    let (mut engine, mut control) = synced_pair(0.61, 60.0);
    let (mut on_master, mut on_cue) = (Onsets::default(), Onsets::default());
    control.params().deck(0).set_playing(true);
    control.params().deck(1).set_playing(true);
    run(&mut engine, 4.0, &mut on_master, &mut on_cue);
    let a = control.status().deck(0).position_secs();
    control.seek(0, a + 0.1); // the master jumps a fifth of a beat
    run(&mut engine, 12.0, &mut on_master, &mut on_cue);
    let settled = on_master.at as f64 / f64::from(RATE) - 4.0;
    let gap = worst_gap(&on_master.times, &on_cue.times, settled);
    assert!(gap < 0.005, "re-locked: {:.2} ms", gap * 1_000.0);
    let during = worst_gap(&on_master.times, &on_cue.times, 4.1);
    assert!(
        during > 0.01,
        "the jump really put the decks out of phase first"
    );
}
