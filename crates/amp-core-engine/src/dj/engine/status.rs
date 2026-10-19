//! What the render thread reports (render → control), through atomics only.

#![forbid(unsafe_code)]

use super::loops::{LoopState, HOT_CUES};
use super::params::{AtomicF32, AtomicF64, MAX_DECKS};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// What the render thread reports about one deck.
#[derive(Debug, Default)]
pub struct DeckStatus {
    position_secs: AtomicF64,
    loaded_secs: AtomicF64,
    ended: AtomicBool,
    underruns: AtomicU64,
    bpm: AtomicF64,
    master: AtomicBool,
    loop_start: AtomicF64,
    loop_end: AtomicF64,
    loop_active: AtomicBool,
    cues: [AtomicF64; HOT_CUES],
    peak: AtomicF32,
    rms: AtomicF32,
}

impl DeckStatus {
    /// Playhead in seconds of the track.
    pub fn position_secs(&self) -> f64 {
        self.position_secs.get()
    }
    /// Seconds of the track decoded so far.
    pub fn loaded_secs(&self) -> f64 {
        self.loaded_secs.get()
    }
    /// True once the deck played to the end of its track.
    pub fn ended(&self) -> bool {
        self.ended.load(Ordering::Relaxed)
    }
    /// Tempo the deck is playing at (its grid tempo times its speed), 0 without a grid.
    pub fn bpm(&self) -> f64 {
        self.bpm.get()
    }
    /// True while this deck is the tempo master.
    pub fn is_master(&self) -> bool {
        self.master.load(Ordering::Relaxed)
    }
    /// Level of the last block after trim, EQ and filter (before the fader): `(peak, rms)`.
    pub fn meter(&self) -> (f32, f32) {
        (self.peak.get(), self.rms.get())
    }
    pub(crate) fn publish_meter(&self, (peak, rms): (f32, f32)) {
        self.peak.set(peak);
        self.rms.set(rms);
    }
    /// The current (or last) loop as `(start, end)` seconds.
    pub fn loop_range(&self) -> Option<(f64, f64)> {
        let (s, e) = (self.loop_start.get(), self.loop_end.get());
        (e > s).then_some((s, e))
    }
    /// True while the deck loops.
    pub fn loop_active(&self) -> bool {
        self.loop_active.load(Ordering::Relaxed)
    }
    /// Hot cue `slot` in seconds, if set.
    pub fn hot_cue(&self, slot: usize) -> Option<f64> {
        let v = self.cues.get(slot)?.get();
        (v >= 0.0).then_some(v)
    }
    pub(crate) fn publish_loops(&self, l: &LoopState, rate: f64) {
        let (s, e) = l
            .range
            .filter(|_| rate > 0.0)
            .map_or((0.0, 0.0), |(s, e)| (s / rate, e / rate));
        self.loop_start.set(s);
        self.loop_end.set(e);
        self.loop_active.store(l.active, Ordering::Relaxed);
        for (slot, cue) in self.cues.iter().zip(l.cues) {
            slot.set(cue.unwrap_or(-1.0));
        }
    }
    pub(crate) fn publish_tempo(&self, bpm: f64, master: bool) {
        self.bpm.set(bpm);
        self.master.store(master, Ordering::Relaxed);
    }
    /// Output blocks in which the deck had to play silence because decoding lagged behind.
    pub fn underruns(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }
    pub(crate) fn publish(&self, position: f64, loaded: f64, ended: bool) {
        self.position_secs.set(position);
        self.loaded_secs.set(loaded);
        self.ended.store(ended, Ordering::Relaxed);
    }
    pub(crate) fn count_underrun(&self) {
        self.underruns.fetch_add(1, Ordering::Relaxed);
    }
}

/// Status of every deck and of the master bus.
#[derive(Debug, Default)]
pub struct DjStatus {
    decks: [DeckStatus; MAX_DECKS],
    master_peak: AtomicF32,
    master_rms: AtomicF32,
    limiter_gain: AtomicF32,
}

impl DjStatus {
    /// Status of deck `i`. Panics when `i >= MAX_DECKS`.
    pub fn deck(&self, i: usize) -> &DeckStatus {
        &self.decks[i]
    }
    /// Master output level of the last block after the limiter: `(peak, rms)`.
    pub fn master_meter(&self) -> (f32, f32) {
        (self.master_peak.get(), self.master_rms.get())
    }
    /// Lowest limiter gain in the last block (1 = not limiting).
    pub fn limiter_gain(&self) -> f32 {
        self.limiter_gain.get()
    }
    pub(crate) fn publish_master(&self, (peak, rms): (f32, f32), limiter: f32) {
        self.master_peak.set(peak);
        self.master_rms.set(rms);
        self.limiter_gain.set(limiter);
    }
}

/// Peak and RMS of interleaved samples.
pub(crate) fn measure(buf: &[f32]) -> (f32, f32) {
    let peak = buf.iter().fold(0.0f32, |a, v| a.max(v.abs()));
    let rms = (buf.iter().map(|v| v * v).sum::<f32>() / buf.len().max(1) as f32).sqrt();
    (peak, rms)
}
