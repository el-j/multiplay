//! Stem parameters and controls for DJ decks and remix slots.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Number of isolated stems per deck or remix slot.
pub const STEM_COUNT: usize = 4;

/// Four-stem separation targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StemType {
    /// Drums and percussion: kick transients, snare fundamentals (150 Hz - 800 Hz).
    Drums,
    /// Bass and sub-frequencies (< 150 Hz).
    Bass,
    /// Melodic instruments, synths, and harmonic highs (> 4,500 Hz).
    Melody,
    /// Vocal presence and lead melody (800 Hz - 4,500 Hz).
    Vocals,
}

impl StemType {
    /// Zero-based index for array indexing.
    #[must_use]
    pub const fn as_index(&self) -> usize {
        match self {
            Self::Drums => 0,
            Self::Bass => 1,
            Self::Melody => 2,
            Self::Vocals => 3,
        }
    }

    /// Converts a zero-based index back to a `StemType`.
    #[must_use]
    pub const fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(Self::Drums),
            1 => Some(Self::Bass),
            2 => Some(Self::Melody),
            3 => Some(Self::Vocals),
            _ => None,
        }
    }

    /// Returns human-readable label for this stem.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Drums => "Drums",
            Self::Bass => "Bass",
            Self::Melody => "Melody",
            Self::Vocals => "Vocals",
        }
    }
}

/// Parameters for all four stems on a deck or remix slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StemDeckParams {
    volumes: [f32; STEM_COUNT],
    mutes: [bool; STEM_COUNT],
    solos: [bool; STEM_COUNT],
    filters: [f32; STEM_COUNT],
}

impl Default for StemDeckParams {
    fn default() -> Self {
        Self::new()
    }
}

impl StemDeckParams {
    /// Creates default stem parameters with unity gain (1.0), unmuted, and filters centered.
    #[must_use]
    pub fn new() -> Self {
        Self {
            volumes: [1.0; STEM_COUNT],
            mutes: [false; STEM_COUNT],
            solos: [false; STEM_COUNT],
            filters: [0.0; STEM_COUNT],
        }
    }

    /// Sets the volume for a stem (clamped 0.0 to 1.0).
    pub fn set_volume(&mut self, stem: StemType, volume: f32) {
        self.volumes[stem.as_index()] = volume.clamp(0.0, 1.0);
    }

    /// Gets the volume for a stem.
    #[must_use]
    pub fn volume(&self, stem: StemType) -> f32 {
        self.volumes[stem.as_index()]
    }

    /// Sets the mute switch for a stem.
    pub fn set_mute(&mut self, stem: StemType, muted: bool) {
        self.mutes[stem.as_index()] = muted;
    }

    /// Checks if a stem is muted.
    #[must_use]
    pub fn is_muted(&self, stem: StemType) -> bool {
        self.mutes[stem.as_index()]
    }

    /// Sets the solo switch for a stem.
    pub fn set_solo(&mut self, stem: StemType, solo: bool) {
        self.solos[stem.as_index()] = solo;
    }

    /// Checks if a stem is soloed.
    #[must_use]
    pub fn is_solo(&self, stem: StemType) -> bool {
        self.solos[stem.as_index()]
    }

    /// Sets the one-knob filter for a stem (-1.0 to +1.0).
    pub fn set_filter(&mut self, stem: StemType, filter: f32) {
        self.filters[stem.as_index()] = filter.clamp(-1.0, 1.0);
    }

    /// Gets the filter knob value for a stem.
    #[must_use]
    pub fn filter(&self, stem: StemType) -> f32 {
        self.filters[stem.as_index()]
    }

    /// Returns true if any stem is currently in solo mode.
    #[must_use]
    pub fn has_any_solo(&self) -> bool {
        self.solos.iter().any(|&s| s)
    }

    /// Returns true if this stem should produce audible output.
    #[must_use]
    pub fn is_active(&self, stem: StemType) -> bool {
        let idx = stem.as_index();
        if self.mutes[idx] {
            return false;
        }
        if self.has_any_solo() {
            return self.solos[idx];
        }
        true
    }

    /// Returns the effective output gain for this stem, taking volume, mute, and solo into account.
    #[must_use]
    pub fn effective_gain(&self, stem: StemType) -> f32 {
        if self.is_active(stem) {
            self.volume(stem)
        } else {
            0.0
        }
    }
}
