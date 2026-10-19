//! Auto mix (TASK-208): a four-track playlist with deck assignments mixes end to end with no
//! silence, synced transitions keep beats within 10 ms, and the order follows the assignments.

use amp_core_engine::dj::automix::{
    Assignment, AutoMix, AutoMixConfig, AutoMixState, AutoMixTrack, PlaylistSource, TransitionStart,
};
use amp_core_engine::dj::engine::sync::{phase_error, DeckBeat};
use amp_core_engine::dj::engine::{DjControl, DjEngine, TrackBuffer};
use amp_domain::{TrackAnalysis, ANALYSIS_VERSION};

const RATE: u32 = 48_000;
const SOURCE_RATE: u32 = 22_050;

/// A steady hum (never silent) with a click on every beat, plus its exact analysis.
fn track(bpm: f64, secs: f64, hum_hz: f64) -> AutoMixTrack {
    let frames = (f64::from(SOURCE_RATE) * secs) as usize;
    let period = 60.0 / bpm;
    let s: Vec<f32> = (0..frames)
        .flat_map(|i| {
            let t = i as f64 / f64::from(SOURCE_RATE);
            let since = t.rem_euclid(period);
            let click = if since < 0.004 {
                0.4 * (2.0 * std::f64::consts::PI * 1_500.0 * since).sin()
            } else {
                0.0
            };
            let v = (0.15 * (2.0 * std::f64::consts::PI * hum_hz * t).sin() + click) as f32;
            [v, v]
        })
        .collect();
    AutoMixTrack {
        title: format!("{bpm} BPM"),
        buffer: TrackBuffer::from_samples(SOURCE_RATE, &s),
        analysis: Some(TrackAnalysis {
            version: ANALYSIS_VERSION,
            duration_secs: secs,
            bpm: Some(bpm),
            first_downbeat_secs: 0.0,
            beats_per_bar: 4,
            overview: Vec::new(),
            detail: Vec::new(),
            detail_rate: 100.0,
            loudness_lufs: Some(-14.0),
        }),
        pending: None,
    }
}

struct Run {
    played: Vec<String>,
    silent_gap: f64,
    worst_phase_ms: f64,
    transitions: usize,
    finished_at: Option<f64>,
}

fn run(config: AutoMixConfig, source: &mut PlaylistSource, max_secs: f64) -> Run {
    let (mut engine, mut control) = DjEngine::new(RATE, 2);
    let mut auto = AutoMix::new(config);
    assert!(auto.start(&mut control, source), "first track loads");
    let (mut m, mut c) = (vec![0.0f32; 1_024], vec![0.0f32; 1_024]);
    let mut quiet = 0usize;
    let mut out = Run {
        played: Vec::new(),
        silent_gap: 0.0,
        worst_phase_ms: 0.0,
        transitions: 0,
        finished_at: None,
    };
    let mut was_mixing = false;
    for block in 0..(max_secs * f64::from(RATE) / 512.0) as usize {
        engine.render(&mut m, &mut c);
        let state = auto.tick(&mut control, source);
        let now = block as f64 * 512.0 / f64::from(RATE);
        let rms = (m.iter().map(|v| v * v).sum::<f32>() / m.len() as f32).sqrt();
        quiet = if rms < 0.02 { quiet + 512 } else { 0 };
        if state != AutoMixState::Finished {
            out.silent_gap = out.silent_gap.max(quiet as f64 / f64::from(RATE));
        }
        let mixing = matches!(state, AutoMixState::Mixing { synced: true, .. });
        // From the incoming deck's first rendered block on (on the tick that starts a mix it
        // is still parked at its cue point).
        if mixing && was_mixing {
            out.worst_phase_ms = out.worst_phase_ms.max(phase_ms(&control));
        }
        if mixing && !was_mixing {
            out.transitions += 1;
        }
        was_mixing = mixing;
        if state == AutoMixState::Finished && out.finished_at.is_none() {
            out.finished_at = Some(now);
            break;
        }
    }
    out.played = auto.history().to_vec();
    out
}

/// Beat distance between the two playing decks, in ms.
fn phase_ms(control: &DjControl) -> f64 {
    let beat = |d: usize| {
        let (bpm, anchor) = control.params().deck(d).grid()?;
        let s = control.status().deck(d);
        Some(DeckBeat {
            bpm,
            anchor,
            position: s.position_secs(),
            speed: s.bpm() / bpm,
        })
    };
    match (beat(0), beat(1)) {
        (Some(a), Some(b)) => {
            let (master, other) = if control.status().deck(0).is_master() {
                (a, b)
            } else {
                (b, a)
            };
            phase_error(&master, &other).abs() * 60.0 / other.heard_bpm() * 1_000.0
        }
        _ => 0.0,
    }
}

fn playlist() -> PlaylistSource {
    PlaylistSource::new(vec![
        (track(120.0, 40.0, 110.0), Assignment::Deck(0)),
        (track(124.0, 40.0, 130.0), Assignment::Deck(1)),
        (track(122.0, 40.0, 150.0), Assignment::Any),
        (track(126.0, 40.0, 170.0), Assignment::Any),
    ])
}

#[test]
fn dj_automix_plays_a_four_track_playlist_end_to_end() {
    let config = AutoMixConfig {
        start: TransitionStart::BeatsBeforeEnd(16.0),
        length_beats: 16.0,
        ..Default::default()
    };
    let r = run(config, &mut playlist(), 200.0);
    assert_eq!(r.played, vec!["120 BPM", "124 BPM", "122 BPM", "126 BPM"]);
    assert_eq!(r.transitions, 3, "three synced transitions");
    assert!(
        r.silent_gap < 0.05,
        "never silent: longest quiet {:.3} s",
        r.silent_gap
    );
    assert!(
        r.worst_phase_ms < 10.0,
        "beats within 10 ms: {:.2} ms",
        r.worst_phase_ms
    );
    // 4 × 40 s minus three overlaps of about 8 s each.
    let end = r.finished_at.expect("the last track ends");
    eprintln!(
        "auto mix: worst phase {:.3} ms, longest quiet {:.3} s, finished at {end:.1} s",
        r.worst_phase_ms, r.silent_gap
    );
    assert!((130.0..145.0).contains(&end), "finished at {end:.1} s");
}

#[test]
fn dj_automix_crossfades_without_sync_when_tempos_are_far_apart() {
    let mut source = PlaylistSource::new(vec![
        (track(90.0, 30.0, 110.0), Assignment::Any),
        (track(140.0, 30.0, 130.0), Assignment::Any),
    ]);
    let config = AutoMixConfig {
        start: TransitionStart::SecondsBeforeEnd(6.0),
        ..Default::default()
    };
    let r = run(config, &mut source, 120.0);
    assert_eq!(r.played, vec!["90 BPM", "140 BPM"]);
    assert_eq!(
        r.transitions, 0,
        "no synced transition between 90 and 140 BPM"
    );
    assert!(
        r.silent_gap < 0.05,
        "an equal-power crossfade, no gap: {:.3} s",
        r.silent_gap
    );
}

#[test]
fn dj_automix_takes_the_next_unplayed_track_when_none_is_assigned() {
    let mut source = PlaylistSource::new(vec![
        (track(120.0, 20.0, 110.0), Assignment::Deck(0)),
        (track(121.0, 20.0, 130.0), Assignment::Deck(0)),
        (track(122.0, 20.0, 150.0), Assignment::Deck(0)),
    ]);
    // Deck B has nothing assigned, so it takes the next unplayed track from the list.
    let r = run(AutoMixConfig::default(), &mut source, 120.0);
    assert_eq!(r.played, vec!["120 BPM", "121 BPM", "122 BPM"]);
    assert!(r.finished_at.is_some());
}

#[test]
fn dj_automix_takes_over_a_playing_deck_and_waits_for_late_analysis() {
    use std::sync::{Arc, OnceLock};
    // Tracks whose analysis is still running when they are loaded.
    let late = |bpm: f64, hum: f64| {
        let ready = track(bpm, 30.0, hum);
        let cell = Arc::new(OnceLock::new());
        let analysis = ready.analysis.clone().unwrap();
        (
            AutoMixTrack {
                analysis: None,
                pending: Some(Arc::clone(&cell)),
                ..ready
            },
            cell,
            analysis,
        )
    };
    let (current, current_cell, current_analysis) = late(120.0, 110.0);
    let (next, next_cell, next_analysis) = late(122.0, 130.0);
    let mut source = PlaylistSource::new(vec![(next, Assignment::Any)]);

    let (mut engine, mut control) = DjEngine::new(RATE, 2);
    control.load(0, Arc::clone(&current.buffer));
    control.params().set_crossfader(-1.0);
    control.params().deck(0).set_playing(true);
    let mut auto = AutoMix::new(AutoMixConfig::default());
    auto.take_over(&mut control, &mut source, 0, current);

    let (mut m, mut c) = (vec![0.0f32; 1_024], vec![0.0f32; 1_024]);
    let mut synced = false;
    let mut state = auto.state();
    for block in 0..(80.0 * f64::from(RATE) / 512.0) as usize {
        if block == 200 {
            let _ = current_cell.set(current_analysis.clone());
            let _ = next_cell.set(next_analysis.clone());
        }
        engine.render(&mut m, &mut c);
        state = auto.tick(&mut control, &mut source);
        synced |= matches!(state, AutoMixState::Mixing { synced: true, .. });
        if state == AutoMixState::Finished {
            break;
        }
    }
    assert_eq!(auto.history(), ["120 BPM", "122 BPM"]);
    assert!(
        synced,
        "the late analysis made a synced transition possible"
    );
    assert_eq!(state, AutoMixState::Finished);
}
