//! DJ mixer through the engine (TASK-207): kills, filter, crossfader curves, the master limiter,
//! meters, auto-gain, the master EQ, and the real-time CPU budget.

use amp_core_engine::dj::engine::{Band, CrossfaderCurve, DjControl, DjEngine, TrackBuffer};
use std::sync::Arc;

const RATE: u32 = 48_000;

fn tone(hz: f64, amp: f64, secs: f64) -> Arc<TrackBuffer> {
    let s: Vec<f32> = (0..(f64::from(RATE) * secs) as usize)
        .flat_map(|i| {
            let v =
                (amp * (2.0 * std::f64::consts::PI * hz * i as f64 / f64::from(RATE)).sin()) as f32;
            [v, v]
        })
        .collect();
    TrackBuffer::from_samples(RATE, &s)
}

fn one_deck(hz: f64, amp: f64) -> (DjEngine, DjControl) {
    let (engine, mut control) = DjEngine::new(RATE, 2);
    control.load(0, tone(hz, amp, 10.0));
    let p = control.params();
    p.set_crossfader(-1.0);
    p.deck(0).set_playing(true);
    (engine, control)
}

/// RMS of the master bus over `secs`, after half a second of settling.
fn master_rms(engine: &mut DjEngine, secs: f64) -> f32 {
    let mut out = |s: f64| {
        let n = (s * f64::from(RATE)) as usize * 2;
        let (mut m, mut c) = (vec![0.0f32; n], vec![0.0f32; n]);
        engine.render(&mut m, &mut c);
        m
    };
    let _ = out(0.5);
    let m = out(secs);
    (m.iter().map(|v| v * v).sum::<f32>() / m.len() as f32).sqrt()
}

fn db(ratio: f32) -> f32 {
    20.0 * ratio.log10()
}

#[test]
fn dj_mixer_kills_and_filter_work_through_the_engine() {
    let (mut engine, control) = one_deck(50.0, 0.5);
    let open = master_rms(&mut engine, 0.5);
    control.params().deck(0).channel().set_kill(Band::Low, true);
    let killed = master_rms(&mut engine, 0.5);
    assert!(
        db(killed / open) < -30.0,
        "low kill: {:.1} dB",
        db(killed / open)
    );

    let (mut engine, control) = one_deck(5_000.0, 0.5);
    let open = master_rms(&mut engine, 0.5);
    control.params().deck(0).channel().set_filter(-0.8);
    let filtered = master_rms(&mut engine, 0.5);
    assert!(
        db(filtered / open) < -20.0,
        "low-pass: {:.1} dB",
        db(filtered / open)
    );
}

#[test]
fn dj_mixer_sharp_curve_keeps_both_decks_full_near_the_middle() {
    let (mut engine, mut control) = DjEngine::new(RATE, 2);
    control.load(0, tone(300.0, 0.25, 10.0));
    control.load(1, tone(300.0, 0.25, 10.0));
    let p = control.params();
    p.master().set_curve(CrossfaderCurve::Sharp);
    p.set_crossfader(0.9);
    p.deck(0).set_playing(true);
    p.deck(1).set_playing(true);
    let both = master_rms(&mut engine, 0.5);
    // Two identical, in-phase decks at full gain: twice the level of one.
    let one = 0.25 / std::f32::consts::SQRT_2;
    assert!(
        (db(both / one) - 6.02).abs() < 0.2,
        "{:.2} dB",
        db(both / one)
    );
}

#[test]
fn dj_mixer_limiter_keeps_a_plus_6_db_mix_below_full_scale() {
    let (mut engine, mut control) = DjEngine::new(RATE, 2);
    control.load(0, tone(220.0, 1.0, 10.0));
    control.load(1, tone(220.0, 1.0, 10.0));
    let p = control.params();
    p.master().set_curve(CrossfaderCurve::Sharp);
    p.set_crossfader(0.0);
    p.deck(0).set_playing(true);
    p.deck(1).set_playing(true);
    let n = RATE as usize * 2 * 3;
    let (mut m, mut c) = (vec![0.0f32; n], vec![0.0f32; n]);
    engine.render(&mut m, &mut c);
    let peak = m.iter().fold(0.0f32, |a, v| a.max(v.abs()));
    assert!(peak <= 0.98 + 1e-6, "master peak {peak}");
    assert!(peak > 0.95, "and still loud: {peak}");
    assert!(control.status().limiter_gain() < 0.6, "limiting about 6 dB");
    let (mp, _) = control.status().master_meter();
    assert!((mp - peak).abs() < 0.05, "master meter {mp} vs {peak}");
}

#[test]
fn dj_mixer_meters_read_the_deck_level_before_the_fader() {
    let (mut engine, control) = one_deck(1_000.0, 0.5);
    control.params().deck(0).set_fader(0.2);
    let _ = master_rms(&mut engine, 0.2);
    let (peak, rms) = control.status().deck(0).meter();
    assert!((peak - 0.5).abs() < 0.02, "deck peak {peak}");
    assert!(
        (rms - 0.5 / std::f32::consts::SQRT_2).abs() < 0.02,
        "deck rms {rms}"
    );
    let (_, master) = control.status().master_meter();
    assert!(
        (master - rms * 0.2).abs() < 0.01,
        "master follows the fader: {master}"
    );
}

#[test]
fn dj_mixer_auto_gain_levels_the_track() {
    let (mut engine, control) = one_deck(1_000.0, 0.5);
    let unity = master_rms(&mut engine, 0.5);
    let ch = control.params().deck(0).channel();
    ch.set_auto_gain_db(-6.0);
    let quieter = master_rms(&mut engine, 0.5);
    assert!((db(quieter / unity) + 6.0).abs() < 0.1);
    ch.set_auto_gain(false);
    let off = master_rms(&mut engine, 0.5);
    assert!(db(off / unity).abs() < 0.1);
}

#[test]
fn dj_mixer_classic_eq_shapes_the_master_bus() {
    let (mut engine, control) = one_deck(1_000.0, 0.25);
    let flat = master_rms(&mut engine, 0.5);
    let m = control.params().master();
    m.set_eq_band(4, 6.0); // the 1 kHz band
    m.set_eq_enabled(true);
    let boosted = master_rms(&mut engine, 0.5);
    assert!(
        (db(boosted / flat) - 6.0).abs() < 0.5,
        "{:.2} dB",
        db(boosted / flat)
    );
    m.set_eq_enabled(false);
    assert!(db(master_rms(&mut engine, 0.5) / flat).abs() < 0.1);
}

/// Real-time budget: four decks (two with key lock), every mixer stage on, rendered 20x faster
/// than real time on one core. Release builds only.
#[test]
#[cfg_attr(debug_assertions, ignore = "timing target applies to release builds")]
fn dj_mixer_four_decks_render_well_within_real_time() {
    let (mut engine, mut control) = DjEngine::new(RATE, 4);
    let p = control.params_handle();
    for d in 0..4 {
        control.load(d, tone(200.0 + 100.0 * d as f64, 0.3, 30.0));
        let deck = p.deck(d);
        deck.set_playing(true);
        deck.set_pitch_percent(3.0);
        deck.set_key_lock(d < 2);
        deck.channel().set_eq(Band::High, -6.0);
        deck.channel().set_filter(0.3);
    }
    p.master().set_eq_enabled(true);
    let (mut m, mut c) = (vec![0.0f32; 512], vec![0.0f32; 512]);
    let secs = 10.0;
    let blocks = (secs * f64::from(RATE) / 256.0) as usize;
    let start = std::time::Instant::now();
    for _ in 0..blocks {
        engine.render(&mut m, &mut c);
    }
    let took = start.elapsed().as_secs_f64();
    assert!(took < secs / 20.0, "{secs} s of audio took {took:.3} s");
}
