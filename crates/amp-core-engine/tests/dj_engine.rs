//! DJ engine (TASK-202): decks at any source rate mix at the right pitch, pitch changes tempo,
//! progressive loading never plays garbage, and the render path never allocates. The
//! wall-clock runtime tests live in `dj_runtime.rs`.

use amp_core_engine::dj::engine::{
    spawn_loader, Band, DjEngine, TrackBuffer, CROSSFADER_LEFT, CROSSFADER_RIGHT,
};
use amp_ports::PcmStreamPort;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ---- allocation guard (test only): counts allocations made by the current thread ----

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

fn allocations_during(f: impl FnOnce()) -> usize {
    COUNT.with(|c| c.set(0));
    COUNTING.with(|c| c.set(true));
    f();
    COUNTING.with(|c| c.set(false));
    COUNT.with(Cell::get)
}

// ---- helpers ----

fn sine(rate: u32, hz: f32, secs: f32) -> Arc<TrackBuffer> {
    let frames = (rate as f32 * secs) as usize;
    let mut s = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let v = 0.5 * (2.0 * std::f32::consts::PI * hz * i as f32 / rate as f32).sin();
        s.extend([v, v]);
    }
    TrackBuffer::from_samples(rate, &s)
}

/// Dominant frequency of the left channel by counting rising zero crossings.
fn frequency(stereo: &[f32], rate: u32) -> f32 {
    let left: Vec<f32> = stereo.iter().step_by(2).copied().collect();
    let crossings = left
        .windows(2)
        .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
        .count();
    crossings as f32 * rate as f32 / left.len() as f32
}

fn render(engine: &mut DjEngine, secs: f32) -> (Vec<f32>, Vec<f32>) {
    let n = (engine.sample_rate() as f32 * secs) as usize * 2;
    let (mut master, mut cue) = (vec![0.0; n], vec![0.0; n]);
    engine.render(&mut master, &mut cue);
    (master, cue)
}

fn rms(s: &[f32]) -> f32 {
    (s.iter().map(|v| v * v).sum::<f32>() / s.len().max(1) as f32).sqrt()
}

// ---- engine ----

#[test]
fn dj_two_decks_at_44k1_and_48k_play_at_their_true_pitch() {
    let (mut engine, mut control) = DjEngine::new(48_000, 2);
    control.load(0, sine(44_100, 1_000.0, 3.0));
    control.load(1, sine(48_000, 1_500.0, 3.0));
    for d in 0..2 {
        control.params().deck(d).set_playing(true);
    }
    control.params().set_crossfader(CROSSFADER_LEFT);
    let (a, _) = render(&mut engine, 0.5);
    assert!(
        (frequency(&a, 48_000) - 1_000.0).abs() < 5.0,
        "deck A {}",
        frequency(&a, 48_000)
    );
    control.params().set_crossfader(CROSSFADER_RIGHT);
    let (_, _) = render(&mut engine, 0.05); // gain ramp settles
    let (b, _) = render(&mut engine, 0.5);
    assert!(
        (frequency(&b, 48_000) - 1_500.0).abs() < 5.0,
        "deck B {}",
        frequency(&b, 48_000)
    );
}

#[test]
fn dj_pitch_changes_tempo_and_pitch_together() {
    let (mut engine, mut control) = DjEngine::new(48_000, 2);
    control.load(0, sine(44_100, 1_000.0, 5.0));
    let deck = control.params().deck(0);
    deck.set_pitch_percent(6.0);
    deck.set_playing(true);
    control.params().set_crossfader(CROSSFADER_LEFT);
    let (out, _) = render(&mut engine, 1.0);
    assert!((frequency(&out[9_600..], 48_000) - 1_060.0).abs() < 6.0);
    let pos = control.status().deck(0).position_secs();
    assert!((pos - 1.06).abs() < 0.002, "position {pos}");
}

#[test]
fn dj_prelistened_deck_is_on_cue_only() {
    let (mut engine, mut control) = DjEngine::new(48_000, 2);
    control.load(1, sine(48_000, 440.0, 2.0));
    let deck = control.params().deck(1);
    deck.set_fader(0.0);
    deck.set_pfl(true);
    deck.set_playing(true);
    let (master, cue) = render(&mut engine, 0.5);
    assert!(
        rms(&master) < 1e-6,
        "master must be silent, rms {}",
        rms(&master)
    );
    assert!(rms(&cue) > 0.2, "cue carries the deck, rms {}", rms(&cue));
}

#[test]
fn dj_track_end_stops_the_deck_without_running_past() {
    let (mut engine, mut control) = DjEngine::new(48_000, 2);
    control.load(0, sine(8_000, 200.0, 1.0));
    control.params().deck(0).set_playing(true);
    let _ = render(&mut engine, 1.5);
    let status = control.status().deck(0);
    assert!(status.ended());
    assert!((status.position_secs() - 1.0).abs() < 0.001);
    assert_eq!(status.underruns(), 0);
}

#[test]
fn dj_ten_minutes_of_mixing_without_underruns() {
    let (mut engine, mut control) = DjEngine::new(48_000, 2);
    control.load(0, sine(8_000, 300.0, 610.0));
    control.load(1, sine(11_025, 500.0, 610.0));
    for d in 0..2 {
        control.params().deck(d).set_playing(true);
    }
    let mut master = vec![0.0; 1_024];
    let mut cue = vec![0.0; 1_024];
    let blocks = 600 * 48_000 / 512;
    for i in 0..blocks {
        control
            .params()
            .set_crossfader(((i % 2_000) as f32 / 1_000.0) - 1.0);
        engine.render(&mut master, &mut cue);
        assert!(master.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
    }
    for d in 0..2 {
        let s = control.status().deck(d);
        assert_eq!(s.underruns(), 0);
        assert!(!s.ended());
        assert!(
            (s.position_secs() - 600.0).abs() < 0.01,
            "{}",
            s.position_secs()
        );
    }
}

#[test]
fn dj_render_never_allocates() {
    let (mut engine, mut control) = DjEngine::new(48_000, 4);
    for d in 0..4 {
        control.load(d, sine(44_100, 500.0 + d as f32 * 100.0, 4.0));
        control.params().deck(d).set_playing(true);
        control.params().deck(d).set_pfl(d % 2 == 0);
        control.params().deck(d).set_grid(120.0 + d as f64, 0.1);
        control.params().deck(d).set_sync(d > 0);
        control.params().deck(d).set_key_lock(d % 2 == 1);
        control.beat_loop(d, 0.5 + d as f64, d == 3);
        control.mark_hot_cue(d, d);
        let ch = control.params().deck(d).channel();
        ch.set_kill(Band::Mid, d == 1);
        ch.set_eq(Band::High, -3.0);
        ch.set_filter(if d % 2 == 0 { -0.4 } else { 0.4 });
    }
    control.params().master().set_eq_enabled(true);
    control.params().master().set_eq_band(2, 4.0);
    let mut master = vec![0.0; 2_048];
    let mut cue = vec![0.0; 2_048];
    let n = allocations_during(|| {
        for i in 0..200 {
            control
                .params()
                .deck(i % 4)
                .set_pitch_percent((i % 17) as f32 - 8.0);
            control
                .params()
                .set_crossfader((i % 20) as f32 / 10.0 - 1.0);
            engine.render(&mut master, &mut cue);
        }
    });
    assert_eq!(n, 0, "render allocated {n} times");
}

#[test]
fn dj_replaced_track_is_freed_off_the_render_thread() {
    let (mut engine, mut control) = DjEngine::new(48_000, 2);
    let first = sine(48_000, 440.0, 1.0);
    let weak = Arc::downgrade(&first);
    control.load(0, first);
    let mut master = vec![0.0; 256];
    let mut cue = vec![0.0; 256];
    engine.render(&mut master, &mut cue);
    control.load(0, sine(48_000, 880.0, 1.0));
    let n = allocations_during(|| engine.render(&mut master, &mut cue));
    assert_eq!(
        n, 0,
        "swapping tracks must not free memory on the render thread"
    );
    assert!(
        weak.upgrade().is_some(),
        "old track is parked for the control side"
    );
    control.collect_garbage();
    assert!(weak.upgrade().is_none());
}

// ---- progressive loading and the runtime ----

/// Endless-ish sine source that delivers slowly, like a decoder reading a network stream.
struct SlowSine {
    rate: u32,
    phase: f32,
    left: usize,
}

impl PcmStreamPort for SlowSine {
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn read_interleaved(&mut self, out: &mut [f32]) -> Result<usize, String> {
        std::thread::sleep(Duration::from_millis(1));
        let n = out.len().min(self.left) / 2 * 2;
        for f in out[..n].chunks_mut(2) {
            let v = 0.4 * self.phase.sin();
            self.phase += 2.0 * std::f32::consts::PI * 440.0 / self.rate as f32;
            f[0] = v;
            f[1] = v;
        }
        self.left -= n;
        Ok(n)
    }
}

#[test]
fn dj_deck_plays_while_its_track_is_still_loading() {
    let buf = spawn_loader(|| {
        Ok(Box::new(SlowSine {
            rate: 44_100,
            phase: 0.0,
            left: 44_100 * 2 * 20,
        }) as Box<dyn PcmStreamPort>)
    });
    let (mut engine, mut control) = DjEngine::new(48_000, 2);
    control.load(0, Arc::clone(&buf));
    control.params().deck(0).set_playing(true);
    let start = Instant::now();
    while buf.loaded_frames() < 44_100 && start.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(5));
    }
    let (out, _) = render(&mut engine, 0.5);
    assert!(rms(&out) > 0.1);
    assert!(
        !buf.is_complete(),
        "test needs a track that is still loading"
    );
}
