//! TASK-213 soak: ten minutes of mixing (rendered faster than real time) while every control
//! keeps changing: tempo, faders, crossfader, EQ, kills, filter, key lock, sync, loops and hot
//! cues. No underruns, no sample past the limiter's ceiling, and not a single allocation in any
//! render call.

use amp_core_engine::dj::engine::{Band, DjEngine, TrackBuffer};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;

struct CountingAlloc;
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}
// SAFETY: forwards to the system allocator unchanged; only bumps a thread-local counter.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.with(Cell::get) {
            COUNT.with(|c| c.set(c.get() + 1));
        }
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if COUNTING.with(Cell::get) {
            COUNT.with(|c| c.set(c.get() + 1));
        }
        System.dealloc(ptr, layout)
    }
}
#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

/// Tone plus a click on every beat, so loops, sync and key lock have something to work on.
fn track(rate: u32, bpm: f64, hz: f64, secs: f64) -> Arc<TrackBuffer> {
    let frames = (f64::from(rate) * secs) as usize;
    let period = 60.0 / bpm;
    let s: Vec<f32> = (0..frames)
        .flat_map(|i| {
            let t = i as f64 / f64::from(rate);
            let click = if t.rem_euclid(period) < 0.003 {
                0.5
            } else {
                0.0
            };
            let v = (0.3 * (2.0 * std::f64::consts::PI * hz * t).sin() + click) as f32;
            [v, v * 0.8]
        })
        .collect();
    TrackBuffer::from_samples(rate, &s)
}

/// Small deterministic pseudo-random generator.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Runs in release builds (a separate CI step); a debug build takes about a minute.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-build soak; CI runs it with --release"
)]
fn dj_soak_ten_minutes_of_constant_changes() {
    const RATE: u32 = 48_000;
    let (mut engine, mut control) = DjEngine::new(RATE, 2);
    control.load(0, track(11_025, 124.0, 110.0, 640.0));
    control.load(1, track(8_000, 120.0, 165.0, 640.0));
    let p = control.params_handle();
    p.deck(0).set_grid(124.0, 0.0);
    p.deck(1).set_grid(120.0, 0.0);
    p.deck(1).set_sync(true);
    p.master().set_eq_enabled(true);
    for d in 0..2 {
        p.deck(d).set_playing(true);
        p.deck(d).channel().set_auto_gain_db(3.0);
    }
    let mut rng = Lcg(42);
    let (mut m, mut c) = (vec![0.0f32; 1_024], vec![0.0f32; 1_024]);
    let blocks = 600 * RATE as usize / 512;
    let mut allocations = 0;
    let mut peak = 0.0f32;
    for i in 0..blocks {
        let d = i % 2;
        let deck = p.deck(d);
        // Continuous controls every block.
        deck.set_fader(0.5 + 0.5 * rng.next());
        p.set_crossfader(rng.next() * 2.0 - 1.0);
        deck.channel().set_eq(Band::High, rng.next() * 12.0 - 9.0);
        deck.channel().set_filter((rng.next() - 0.5) * 1.6);
        deck.set_trim(0.8 + rng.next());
        p.master().set_eq_band(i % 10, rng.next() * 8.0 - 4.0);
        // Discrete actions every few hundred milliseconds.
        if i % 37 == 0 {
            deck.set_pitch_percent(rng.next() * 16.0 - 8.0);
            deck.channel().set_kill(Band::Low, rng.next() > 0.7);
        }
        if i % 97 == 0 {
            deck.set_key_lock(rng.next() > 0.5);
        }
        if i % 251 == 0 {
            control.beat_loop(d, [0.5, 1.0, 4.0][i % 3], i % 2 == 0);
        }
        if i % 251 == 120 {
            control.exit_loop(d);
        }
        if i % 400 == 0 {
            control.mark_hot_cue(d, i % 8);
        }
        if i % 400 == 200 {
            control.jump_hot_cue(d, (i - 200) % 8);
        }
        COUNT.with(|n| n.set(0));
        COUNTING.with(|on| on.set(true));
        engine.render(&mut m, &mut c);
        COUNTING.with(|on| on.set(false));
        allocations += COUNT.with(Cell::get);
        for s in m.iter().chain(c.iter()) {
            assert!(s.is_finite(), "block {i}: non-finite sample");
        }
        peak = m.iter().fold(peak, |a, v| a.max(v.abs()));
    }
    assert_eq!(allocations, 0, "render allocated {allocations} times");
    assert!(peak <= 0.98 + 1e-6, "master peaked at {peak}");
    for d in 0..2 {
        assert_eq!(control.status().deck(d).underruns(), 0, "deck {d} underran");
        assert!(
            !control.status().deck(d).ended(),
            "deck {d} still playing after 10 min"
        );
    }
}
