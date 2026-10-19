//! Mixer controls (TASK-207), as lock-free atomics: per-deck EQ, kills, filter and auto-gain,
//! and master-section crossfader curve and classic 10-band EQ.

#![forbid(unsafe_code)]

use super::params::AtomicF32;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// An EQ band of a deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// Below 250 Hz.
    Low = 0,
    /// 250 Hz to 2.5 kHz.
    Mid = 1,
    /// Above 2.5 kHz.
    High = 2,
}

/// Crossfader curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossfaderCurve {
    /// Equal power: constant loudness through a blend (default).
    Smooth,
    /// Both sides full except near the ends: quick cuts for scratching.
    Sharp,
}

/// Per-deck mixer controls.
#[derive(Debug)]
pub struct ChannelControls {
    eq_db: [AtomicF32; 3],
    kill: [AtomicBool; 3],
    filter: AtomicF32,
    auto_gain: AtomicBool,
    auto_gain_db: AtomicF32,
}

impl Default for ChannelControls {
    fn default() -> Self {
        Self {
            eq_db: Default::default(),
            kill: Default::default(),
            filter: AtomicF32::new(0.0),
            auto_gain: AtomicBool::new(true),
            auto_gain_db: AtomicF32::new(0.0),
        }
    }
}

impl ChannelControls {
    /// Sets a band's gain in dB (-40 to +6).
    pub fn set_eq(&self, band: Band, db: f32) {
        let db = if db.is_finite() { db } else { 0.0 };
        self.eq_db[band as usize].set(db.clamp(-40.0, 6.0));
    }
    /// A band's gain in dB.
    pub fn eq(&self, band: Band) -> f32 {
        self.eq_db[band as usize].get()
    }
    /// Kills (silences) or restores a band.
    pub fn set_kill(&self, band: Band, on: bool) {
        self.kill[band as usize].store(on, Ordering::Relaxed);
    }
    /// True when the band is killed.
    pub fn kill(&self, band: Band) -> bool {
        self.kill[band as usize].load(Ordering::Relaxed)
    }
    /// One-knob filter: -1 low-pass … 0 off … +1 high-pass.
    pub fn set_filter(&self, knob: f32) {
        let k = if knob.is_finite() { knob } else { 0.0 };
        self.filter.set(k.clamp(-1.0, 1.0));
    }
    /// Filter knob position.
    pub fn filter(&self) -> f32 {
        self.filter.get()
    }
    /// Applies the track's loudness correction (on by default).
    pub fn set_auto_gain(&self, on: bool) {
        self.auto_gain.store(on, Ordering::Relaxed);
    }
    /// Loudness correction for the loaded track in dB (from analysis; clamped to ±12 dB).
    pub fn set_auto_gain_db(&self, db: f32) {
        let db = if db.is_finite() { db } else { 0.0 };
        self.auto_gain_db.set(db.clamp(-12.0, 12.0));
    }
    /// Linear gain from auto-gain (1 when off).
    pub fn auto_gain_linear(&self) -> f32 {
        if self.auto_gain.load(Ordering::Relaxed) {
            10.0f32.powf(self.auto_gain_db.get() / 20.0)
        } else {
            1.0
        }
    }
}

/// Master-section controls.
#[derive(Debug)]
pub struct MasterControls {
    curve: AtomicU8,
    eq_on: AtomicBool,
    eq_preamp_db: AtomicF32,
    eq_db: [AtomicF32; 10],
}

impl Default for MasterControls {
    fn default() -> Self {
        Self {
            curve: AtomicU8::new(CrossfaderCurve::Smooth as u8),
            eq_on: AtomicBool::new(false),
            eq_preamp_db: AtomicF32::new(0.0),
            eq_db: Default::default(),
        }
    }
}

impl MasterControls {
    /// Chooses the crossfader curve.
    pub fn set_curve(&self, c: CrossfaderCurve) {
        self.curve.store(c as u8, Ordering::Relaxed);
    }
    /// The crossfader curve.
    pub fn curve(&self) -> CrossfaderCurve {
        match self.curve.load(Ordering::Relaxed) {
            1 => CrossfaderCurve::Sharp,
            _ => CrossfaderCurve::Smooth,
        }
    }
    /// Turns the classic 10-band EQ on the master bus on or off.
    pub fn set_eq_enabled(&self, on: bool) {
        self.eq_on.store(on, Ordering::Relaxed);
    }
    /// True when the master EQ is on.
    pub fn eq_enabled(&self) -> bool {
        self.eq_on.load(Ordering::Relaxed)
    }
    /// Master EQ preamp in dB (±12).
    pub fn set_eq_preamp(&self, db: f32) {
        self.eq_preamp_db.set(db.clamp(-12.0, 12.0));
    }
    /// Master EQ band `i` (0 = 60 Hz … 9 = 16 kHz) in dB (±12).
    pub fn set_eq_band(&self, i: usize, db: f32) {
        if let Some(b) = self.eq_db.get(i) {
            b.set(db.clamp(-12.0, 12.0));
        }
    }
    /// Master EQ preamp and band gains in dB.
    pub fn eq(&self) -> (f32, [f32; 10]) {
        (
            self.eq_preamp_db.get(),
            std::array::from_fn(|i| self.eq_db[i].get()),
        )
    }
}

/// Left and right crossfader gains at `pos` (-1 left … +1 right) for `curve`.
pub fn curve_gains(curve: CrossfaderCurve, pos: f32) -> (f32, f32) {
    let p = pos.clamp(-1.0, 1.0);
    match curve {
        CrossfaderCurve::Smooth => {
            let t = (p + 1.0) * 0.25 * std::f32::consts::PI;
            (t.cos(), t.sin())
        }
        CrossfaderCurve::Sharp => {
            // Full until the last 5 % of travel, then a fast cut.
            let edge = |x: f32| ((1.0 - x) / 0.05).clamp(0.0, 1.0);
            (edge(p), edge(-p))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dj_mixer_smooth_curve_is_equal_power_at_centre() {
        let (l, r) = curve_gains(CrossfaderCurve::Smooth, 0.0);
        assert!((l * l + r * r - 1.0).abs() < 1e-6);
        assert!((l - r).abs() < 1e-6);
    }

    #[test]
    fn dj_mixer_sharp_curve_keeps_both_sides_full_until_the_edge() {
        assert_eq!(curve_gains(CrossfaderCurve::Sharp, 0.0), (1.0, 1.0));
        assert_eq!(curve_gains(CrossfaderCurve::Sharp, 0.9), (1.0, 1.0));
        assert_eq!(curve_gains(CrossfaderCurve::Sharp, 1.0), (0.0, 1.0));
        assert_eq!(curve_gains(CrossfaderCurve::Sharp, -1.0), (1.0, 0.0));
    }

    #[test]
    fn dj_mixer_controls_clamp() {
        let c = ChannelControls::default();
        c.set_eq(Band::Low, 20.0);
        assert_eq!(c.eq(Band::Low), 6.0);
        c.set_auto_gain_db(-30.0);
        assert!((c.auto_gain_linear() - 10.0f32.powf(-12.0 / 20.0)).abs() < 1e-6);
        c.set_auto_gain(false);
        assert_eq!(c.auto_gain_linear(), 1.0);
    }
}
