//! Lock-free parameters (control → render) and status (render → control) for the DJ engine.
//! Every field is a plain atomic, so either side reads and writes without locks, allocation or
//! waiting, and the render thread never clones deck state.

#![forbid(unsafe_code)]

use super::bus::{CueMix, CueRouting};
use super::mixer_params::{ChannelControls, MasterControls};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};

/// Most decks the engine supports (two are shown today; the model holds four).
pub const MAX_DECKS: usize = 4;
/// Crossfader fully on the left side (deck A).
pub const CROSSFADER_LEFT: f32 = -1.0;
/// Crossfader fully on the right side (deck B).
pub const CROSSFADER_RIGHT: f32 = 1.0;
const AUTO_MASTER: u8 = u8::MAX;
/// Widest pitch range, in percent.
pub const MAX_PITCH_PERCENT: f32 = 50.0;

/// An `f32` stored in an `AtomicU32`.
#[derive(Debug, Default)]
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    /// A new atomic holding `v`.
    pub fn new(v: f32) -> Self {
        Self(AtomicU32::new(v.to_bits()))
    }
    /// Current value.
    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
    /// Replaces the value.
    pub fn set(&self, v: f32) {
        self.0.store(v.to_bits(), Ordering::Relaxed);
    }
}

/// An `f64` stored in an `AtomicU64`.
#[derive(Debug, Default)]
pub struct AtomicF64(AtomicU64);

impl AtomicF64 {
    /// Current value.
    pub fn get(&self) -> f64 {
        f64::from_bits(self.0.load(Ordering::Relaxed))
    }
    /// Replaces the value.
    pub fn set(&self, v: f64) {
        self.0.store(v.to_bits(), Ordering::Relaxed);
    }
}

/// Which side of the crossfader a deck is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossfaderSide {
    /// Full when the crossfader is left.
    Left,
    /// Full when the crossfader is right.
    Right,
    /// Ignores the crossfader.
    Thru,
}

/// Controls of one deck, written by the UI and read by the render thread.
#[derive(Debug)]
pub struct DeckParams {
    playing: AtomicBool,
    pitch_percent: AtomicF32,
    fader: AtomicF32,
    trim: AtomicF32,
    pfl: AtomicBool,
    side: AtomicU8,
    nudge: AtomicF32,
    sync: AtomicBool,
    key_lock: AtomicBool,
    grid_bpm: AtomicF64,
    grid_anchor: AtomicF64,
    quantize: AtomicBool,
    channel: ChannelControls,
}

impl DeckParams {
    /// EQ, kills, filter and auto-gain of this deck.
    pub fn channel(&self) -> &ChannelControls {
        &self.channel
    }
    fn new(side: CrossfaderSide) -> Self {
        Self {
            playing: AtomicBool::new(false),
            pitch_percent: AtomicF32::new(0.0),
            fader: AtomicF32::new(1.0),
            trim: AtomicF32::new(1.0),
            pfl: AtomicBool::new(false),
            side: AtomicU8::new(side as u8),
            nudge: AtomicF32::new(0.0),
            sync: AtomicBool::new(false),
            key_lock: AtomicBool::new(false),
            grid_bpm: AtomicF64::default(),
            grid_anchor: AtomicF64::default(),
            quantize: AtomicBool::new(true),
            channel: ChannelControls::default(),
        }
    }
    /// Snaps loop points and cues to the beat grid, and keeps the beat phase on cue jumps.
    pub fn set_quantize(&self, on: bool) {
        self.quantize.store(on, Ordering::Relaxed);
    }
    /// True when quantize is on (the default).
    pub fn quantize(&self) -> bool {
        self.quantize.load(Ordering::Relaxed)
    }
    /// Temporary tempo bend while a nudge button is held, as a fraction (±0.04 = ±4 %).
    pub fn set_nudge(&self, v: f32) {
        self.nudge.set(v.clamp(-0.1, 0.1));
    }
    /// Current nudge.
    pub fn nudge(&self) -> f32 {
        self.nudge.get()
    }
    /// Locks tempo and beat phase to the master deck.
    pub fn set_sync(&self, on: bool) {
        self.sync.store(on, Ordering::Relaxed);
    }
    /// True when the deck follows the master deck.
    pub fn sync(&self) -> bool {
        self.sync.load(Ordering::Relaxed)
    }
    /// Keeps the musical pitch when the tempo changes.
    pub fn set_key_lock(&self, on: bool) {
        self.key_lock.store(on, Ordering::Relaxed);
    }
    /// True when key lock is on.
    pub fn key_lock(&self) -> bool {
        self.key_lock.load(Ordering::Relaxed)
    }
    /// The loaded track's beat grid: tempo and the time of a downbeat (from analysis).
    /// A tempo of 0 means no grid.
    pub fn set_grid(&self, bpm: f64, anchor_secs: f64) {
        let bpm = if bpm.is_finite() && bpm > 0.0 {
            bpm
        } else {
            0.0
        };
        self.grid_bpm.set(bpm);
        self.grid_anchor.set(anchor_secs);
    }
    /// The beat grid as `(bpm, downbeat seconds)`, if the track has one.
    pub fn grid(&self) -> Option<(f64, f64)> {
        let bpm = self.grid_bpm.get();
        (bpm > 0.0).then(|| (bpm, self.grid_anchor.get()))
    }
    /// Starts or pauses the deck.
    pub fn set_playing(&self, on: bool) {
        self.playing.store(on, Ordering::Relaxed);
    }
    /// True while the deck is meant to play.
    pub fn playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }
    /// Tempo change in percent; pitch follows unless key lock is on. Clamped to ±50 %.
    pub fn set_pitch_percent(&self, p: f32) {
        let p = if p.is_finite() { p } else { 0.0 };
        self.pitch_percent
            .set(p.clamp(-MAX_PITCH_PERCENT, MAX_PITCH_PERCENT));
    }
    /// Current tempo change in percent.
    pub fn pitch_percent(&self) -> f32 {
        self.pitch_percent.get()
    }
    /// Channel fader, 0 (closed) to 1 (open).
    pub fn set_fader(&self, v: f32) {
        self.fader.set(v.clamp(0.0, 1.0));
    }
    /// Channel fader position.
    pub fn fader(&self) -> f32 {
        self.fader.get()
    }
    /// Input gain as a linear factor (1 = unity, at most 4 = +12 dB).
    pub fn set_trim(&self, v: f32) {
        self.trim.set(v.clamp(0.0, 4.0));
    }
    /// Input gain.
    pub fn trim(&self) -> f32 {
        self.trim.get()
    }
    /// Sends the deck to the headphone cue bus (pre-fader listen).
    pub fn set_pfl(&self, on: bool) {
        self.pfl.store(on, Ordering::Relaxed);
    }
    /// True when the deck is on the cue bus.
    pub fn pfl(&self) -> bool {
        self.pfl.load(Ordering::Relaxed)
    }
    /// Assigns the deck to a crossfader side.
    pub fn set_side(&self, side: CrossfaderSide) {
        self.side.store(side as u8, Ordering::Relaxed);
    }
    /// The deck's crossfader side.
    pub fn side(&self) -> CrossfaderSide {
        match self.side.load(Ordering::Relaxed) {
            0 => CrossfaderSide::Left,
            1 => CrossfaderSide::Right,
            _ => CrossfaderSide::Thru,
        }
    }
}

/// All mixer controls.
#[derive(Debug)]
pub struct DjParams {
    decks: [DeckParams; MAX_DECKS],
    crossfader: AtomicF32,
    cue_mix: AtomicF32,
    cue_volume: AtomicF32,
    cue_routing: AtomicU8,
    master_deck: AtomicU8,
    master: MasterControls,
}

impl Default for DjParams {
    fn default() -> Self {
        Self {
            decks: [
                DeckParams::new(CrossfaderSide::Left),
                DeckParams::new(CrossfaderSide::Right),
                DeckParams::new(CrossfaderSide::Thru),
                DeckParams::new(CrossfaderSide::Thru),
            ],
            crossfader: AtomicF32::new(0.0),
            cue_mix: AtomicF32::new(0.0),
            cue_volume: AtomicF32::new(0.8),
            cue_routing: AtomicU8::new(CueRouting::Off as u8),
            master_deck: AtomicU8::new(AUTO_MASTER),
            master: MasterControls::default(),
        }
    }
}

impl DjParams {
    /// Controls of deck `i` (`0` = A). Panics when `i >= MAX_DECKS`.
    pub fn deck(&self, i: usize) -> &DeckParams {
        &self.decks[i]
    }
    /// Crossfader curve and the classic EQ on the master bus.
    pub fn master(&self) -> &MasterControls {
        &self.master
    }
    /// Crossfader position, -1 (left) to +1 (right).
    pub fn set_crossfader(&self, v: f32) {
        self.crossfader.set(v.clamp(-1.0, 1.0));
    }
    /// Current crossfader position.
    pub fn crossfader(&self) -> f32 {
        self.crossfader.get()
    }
    /// Headphone blend: 0 = cue only, 1 = master only.
    pub fn set_cue_mix(&self, v: f32) {
        self.cue_mix.set(v.clamp(0.0, 1.0));
    }
    /// Headphone level, 0 to 1.
    pub fn set_cue_volume(&self, v: f32) {
        self.cue_volume.set(v.clamp(0.0, 1.0));
    }
    /// Current headphone blend and level.
    pub fn cue_mix(&self) -> CueMix {
        CueMix {
            mix: self.cue_mix.get(),
            volume: self.cue_volume.get(),
        }
    }
    /// Where the headphone mix goes.
    pub fn set_cue_routing(&self, r: CueRouting) {
        self.cue_routing.store(r as u8, Ordering::Relaxed);
    }
    /// Makes deck `i` the tempo master, or `None` for automatic (the loudest playing deck).
    pub fn set_master_deck(&self, i: Option<usize>) {
        let v = i
            .filter(|i| *i < MAX_DECKS)
            .map_or(AUTO_MASTER, |i| i as u8);
        self.master_deck.store(v, Ordering::Relaxed);
    }
    /// The explicitly chosen master deck, `None` when automatic.
    pub fn master_deck(&self) -> Option<usize> {
        let v = self.master_deck.load(Ordering::Relaxed);
        (v != AUTO_MASTER).then_some(usize::from(v))
    }
    /// Current headphone routing.
    pub fn cue_routing(&self) -> CueRouting {
        CueRouting::from_u8(self.cue_routing.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dj_params_clamp_to_their_ranges() {
        let p = DjParams::default();
        p.deck(0).set_pitch_percent(80.0);
        assert_eq!(p.deck(0).pitch_percent(), MAX_PITCH_PERCENT);
        p.deck(0).set_pitch_percent(f32::NAN);
        assert_eq!(p.deck(0).pitch_percent(), 0.0);
        p.deck(1).set_fader(1.5);
        assert_eq!(p.deck(1).fader(), 1.0);
        p.set_crossfader(-3.0);
        assert_eq!(p.crossfader(), CROSSFADER_LEFT);
    }

    #[test]
    fn dj_decks_start_on_classic_crossfader_sides() {
        let p = DjParams::default();
        assert_eq!(p.deck(0).side(), CrossfaderSide::Left);
        assert_eq!(p.deck(1).side(), CrossfaderSide::Right);
        assert_eq!(p.deck(2).side(), CrossfaderSide::Thru);
        p.deck(2).set_side(CrossfaderSide::Right);
        assert_eq!(p.deck(2).side(), CrossfaderSide::Right);
    }
}
