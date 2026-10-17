//! Multi-Deck DJ channel state machine and 3-band isolator kill EQ engine.
//! Zero-allocation real-time audio channel processing for professional DJ mixing.

#![forbid(unsafe_code)]

use crate::dsp::BiquadFilter;
use amp_domain::Track;
use serde::{Deserialize, Serialize};

/// Identifier for DJ mixing decks (up to 4 independent decks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeckId {
    DeckA,
    DeckB,
    DeckC,
    DeckD,
}

impl DeckId {
    /// Returns the human-readable label for this deck.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::DeckA => "DECK A",
            Self::DeckB => "DECK B",
            Self::DeckC => "DECK C",
            Self::DeckD => "DECK D",
        }
    }
}

/// 3-band isolator EQ with instantaneous kill switches and -60 dB to +6 dB range.
#[derive(Debug, Clone)]
pub struct KillIsolatorEq {
    filter_low: BiquadFilter,
    filter_mid: BiquadFilter,
    filter_high: BiquadFilter,
    pub kill_low: bool,
    pub kill_mid: bool,
    pub kill_high: bool,
}

impl KillIsolatorEq {
    /// Creates a new 3-band isolator EQ calibrated for 44.1 kHz / 48 kHz audio.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            filter_low: BiquadFilter::new_peaking_eq(sample_rate, 100.0, 0.707, 0.0),
            filter_mid: BiquadFilter::new_peaking_eq(sample_rate, 1_000.0, 0.707, 0.0),
            filter_high: BiquadFilter::new_peaking_eq(sample_rate, 10_000.0, 0.707, 0.0),
            kill_low: false,
            kill_mid: false,
            kill_high: false,
        }
    }

    /// Updates the gain in dB for the low, mid, and high bands (-60.0 dB to +6.0 dB).
    pub fn set_gains(&mut self, sample_rate: f32, low_db: f32, mid_db: f32, high_db: f32) {
        self.filter_low =
            BiquadFilter::new_peaking_eq(sample_rate, 100.0, 0.707, low_db.clamp(-60.0, 6.0));
        self.filter_mid =
            BiquadFilter::new_peaking_eq(sample_rate, 1_000.0, 0.707, mid_db.clamp(-60.0, 6.0));
        self.filter_high =
            BiquadFilter::new_peaking_eq(sample_rate, 10_000.0, 0.707, high_db.clamp(-60.0, 6.0));
    }

    /// Processes a single sample through the 3-band isolator chain.
    #[inline(always)]
    #[must_use]
    pub fn process_sample(&mut self, sample: f32) -> f32 {
        if self.kill_low && self.kill_mid && self.kill_high {
            return 0.0;
        }

        let mut out = sample;
        if self.kill_low {
            out -= self.filter_low.process_sample(sample);
        } else {
            out = self.filter_low.process_sample(out);
        }

        if self.kill_mid {
            out -= self.filter_mid.process_sample(sample);
        } else {
            out = self.filter_mid.process_sample(out);
        }

        if self.kill_high {
            out -= self.filter_high.process_sample(sample);
        } else {
            out = self.filter_high.process_sample(out);
        }

        out
    }
}

/// Comprehensive state for an individual DJ channel.
#[derive(Debug, Clone)]
pub struct DeckChannelState {
    pub id: DeckId,
    pub loaded_track: Option<Track>,
    pub is_playing: bool,
    pub cue_pfl: bool,
    pub pitch_percent: f32,
    pub bpm: Option<f32>,
    pub volume_fader: f32,
    pub isolator_eq: KillIsolatorEq,
}

impl DeckChannelState {
    /// Creates an initialized empty deck channel.
    #[must_use]
    pub fn new(id: DeckId, sample_rate: f32) -> Self {
        Self {
            id,
            loaded_track: None,
            is_playing: false,
            cue_pfl: false,
            pitch_percent: 0.0,
            bpm: None,
            volume_fader: 1.0,
            isolator_eq: KillIsolatorEq::new(sample_rate),
        }
    }

    /// Loads a track into this deck channel.
    pub fn load_track(&mut self, track: Track) {
        self.loaded_track = Some(track);
        self.is_playing = false;
    }

    /// Sets playback state.
    pub fn set_playing(&mut self, playing: bool) {
        self.is_playing = playing;
    }

    /// Toggles the pre-fader headphone cue (PFL).
    pub fn toggle_cue_pfl(&mut self) {
        self.cue_pfl = !self.cue_pfl;
    }

    /// Adjusts the pitch tempo slider (-16.0% to +16.0%).
    pub fn set_pitch_percent(&mut self, pitch: f32) {
        self.pitch_percent = pitch.clamp(-16.0, 16.0);
    }

    /// Adjusts the vertical volume fader (0.0 to 1.0).
    pub fn set_volume_fader(&mut self, fader: f32) {
        self.volume_fader = fader.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::{AudioSource, TrackMetadata};

    #[test]
    fn test_dj_deck_state_and_cue() {
        let mut deck = DeckChannelState::new(DeckId::DeckA, 44100.0);
        assert_eq!(deck.id, DeckId::DeckA);
        assert!(!deck.is_playing);
        assert!(!deck.cue_pfl);

        // Load track
        let track = Track::new(
            AudioSource::LocalFile {
                path: "/music/track.mp3".into(),
            },
            TrackMetadata::new("DJ Anthem", "Producer"),
        );
        deck.load_track(track);
        assert!(deck.loaded_track.is_some());

        // Toggle PFL cue
        deck.toggle_cue_pfl();
        assert!(deck.cue_pfl);

        // Pitch slider
        deck.set_pitch_percent(4.5);
        assert!((deck.pitch_percent - 4.5).abs() < f32::EPSILON);

        // Volume fader
        deck.set_volume_fader(0.8);
        assert!((deck.volume_fader - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_dj_deck_kill_eq() {
        let mut eq = KillIsolatorEq::new(44100.0);
        eq.kill_low = true;
        eq.kill_mid = true;
        eq.kill_high = true;

        let sample = 0.5;
        let out = eq.process_sample(sample);
        assert_eq!(out, 0.0, "Triple kill should mute completely");
    }
}
