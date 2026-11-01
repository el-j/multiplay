//! Four decks (TASK-220): every deck is heard through its own crossfader assignment (A left,
//! B right, C and D through by default, any deck re-assignable), and four decks mix for ten
//! minutes without an underrun.

use amp_core_engine::dj::engine::{
    CrossfaderSide, DjEngine, TrackBuffer, CROSSFADER_LEFT, CROSSFADER_RIGHT, MAX_DECKS,
};
use std::sync::Arc;

fn tone(rate: u32, hz: f32, secs: f32) -> Arc<TrackBuffer> {
    let frames = (rate as f32 * secs) as usize;
    let s: Vec<f32> = (0..frames)
        .flat_map(|i| {
            let v = 0.4 * (2.0 * std::f32::consts::PI * hz * i as f32 / rate as f32).sin();
            [v, v]
        })
        .collect();
    TrackBuffer::from_samples(rate, &s)
}

fn level(engine: &mut DjEngine) -> f32 {
    let (mut m, mut c) = (vec![0.0f32; 2_048], vec![0.0f32; 2_048]);
    let mut peak = 0.0f32;
    for _ in 0..8 {
        engine.render(&mut m, &mut c);
        peak = m.iter().fold(peak, |a, v| a.max(v.abs()));
    }
    peak
}

#[test]
fn dj_four_deck_each_deck_follows_its_crossfader_assignment() {
    assert_eq!(MAX_DECKS, 4);
    let (mut engine, mut control) = DjEngine::new(48_000, 4);
    for d in 0..4 {
        control.load(d, tone(48_000, 220.0 * (d + 1) as f32, 5.0));
    }
    let p = control.params_handle();
    let sides: Vec<_> = (0..4).map(|d| p.deck(d).side()).collect();
    assert_eq!(
        sides,
        [
            CrossfaderSide::Left,
            CrossfaderSide::Right,
            CrossfaderSide::Thru,
            CrossfaderSide::Thru
        ]
    );
    p.deck(2).set_side(CrossfaderSide::Left);
    for (deck, xf, heard) in [
        (0, CROSSFADER_LEFT, true),
        (0, CROSSFADER_RIGHT, false),
        (1, CROSSFADER_RIGHT, true),
        (1, CROSSFADER_LEFT, false),
        (2, CROSSFADER_LEFT, true),
        (2, CROSSFADER_RIGHT, false),
        (3, CROSSFADER_LEFT, true),
        (3, CROSSFADER_RIGHT, true),
    ] {
        for d in 0..4 {
            p.deck(d).set_playing(d == deck);
        }
        p.set_crossfader(xf);
        level(&mut engine); // let the gain ramp settle
        let peak = level(&mut engine);
        assert_eq!(
            peak > 0.1,
            heard,
            "deck {deck} at crossfader {xf}: peak {peak}"
        );
    }
}

/// Runs in release builds (CI's DJ release step); a debug build takes minutes.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-build soak; CI runs it with --release"
)]
fn dj_four_deck_ten_minutes_without_underruns() {
    const RATE: u32 = 48_000;
    let (mut engine, mut control) = DjEngine::new(RATE, 4);
    for (d, rate) in [44_100, 48_000, 22_050, 96_000].into_iter().enumerate() {
        control.load(d, tone(rate, 110.0 * (d + 1) as f32, 640.0));
    }
    let p = control.params_handle();
    for d in 0..4 {
        p.deck(d).set_playing(true);
        p.deck(d).set_fader(0.6);
    }
    let (mut m, mut c) = (vec![0.0f32; 1_024], vec![0.0f32; 1_024]);
    for i in 0..600 * RATE as usize / 512 {
        p.set_crossfader(((i % 2_000) as f32 / 1_000.0) - 1.0);
        if i % 500 == 0 {
            let d = (i / 500) % 4;
            p.deck(d).set_pitch_percent((i % 13) as f32 - 6.0);
        }
        engine.render(&mut m, &mut c);
        assert!(m.iter().all(|s| s.is_finite()), "block {i}");
    }
    for d in 0..4 {
        assert_eq!(control.status().deck(d).underruns(), 0, "deck {d} underran");
        assert!(!control.status().deck(d).ended(), "deck {d} still playing");
    }
}
