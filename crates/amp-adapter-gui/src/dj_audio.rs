//! DJ live audio shared state between GUI console and real-time audio playback worker.

#![forbid(unsafe_code)]

use amp_core_engine::dj::CrossfaderCurveProfile;
use amp_domain::Track;

/// Shared controls and metrics for dual-deck DJ mixing.
#[derive(Debug, Clone)]
pub struct DjAudioSharedState {
    pub enabled: bool,

    // Deck A Controls
    pub deck_a_track: Option<Track>,
    pub deck_a_playing: bool,
    pub deck_a_pitch: f32,
    pub deck_a_volume: f32,
    pub deck_a_cue: bool,
    pub deck_a_kill_low: bool,
    pub deck_a_kill_mid: bool,
    pub deck_a_kill_high: bool,
    pub deck_a_bpm: f32,
    pub deck_a_change: bool,

    // Deck B Controls
    pub deck_b_track: Option<Track>,
    pub deck_b_playing: bool,
    pub deck_b_pitch: f32,
    pub deck_b_volume: f32,
    pub deck_b_cue: bool,
    pub deck_b_kill_low: bool,
    pub deck_b_kill_mid: bool,
    pub deck_b_kill_high: bool,
    pub deck_b_bpm: f32,
    pub deck_b_change: bool,

    // Master DJ Controls
    pub crossfader_pos: f32,
    pub curve_profile: CrossfaderCurveProfile,
    pub key_lock: bool,
}

impl Default for DjAudioSharedState {
    fn default() -> Self {
        Self {
            enabled: false,
            deck_a_track: None,
            deck_a_playing: false,
            deck_a_pitch: 0.0,
            deck_a_volume: 1.0,
            deck_a_cue: false,
            deck_a_kill_low: false,
            deck_a_kill_mid: false,
            deck_a_kill_high: false,
            deck_a_bpm: 128.0,
            deck_a_change: false,

            deck_b_track: None,
            deck_b_playing: false,
            deck_b_pitch: 0.0,
            deck_b_volume: 1.0,
            deck_b_cue: false,
            deck_b_kill_low: false,
            deck_b_kill_mid: false,
            deck_b_kill_high: false,
            deck_b_bpm: 124.0,
            deck_b_change: false,

            crossfader_pos: 0.0,
            curve_profile: CrossfaderCurveProfile::SmoothDip,
            key_lock: true,
        }
    }
}

impl DjAudioSharedState {
    /// Loads a track into Deck A.
    pub fn load_deck_a(&mut self, track: Track) {
        self.deck_a_track = Some(track);
        self.deck_a_change = true;
        self.deck_a_playing = true;
        self.enabled = true;
    }

    /// Loads a track into Deck B.
    pub fn load_deck_b(&mut self, track: Track) {
        self.deck_b_track = Some(track);
        self.deck_b_change = true;
        self.deck_b_playing = true;
        self.enabled = true;
    }

    /// Toggles play/pause on Deck A.
    pub fn toggle_deck_a_play(&mut self) {
        self.deck_a_playing = !self.deck_a_playing;
    }

    /// Toggles play/pause on Deck B.
    pub fn toggle_deck_b_play(&mut self) {
        self.deck_b_playing = !self.deck_b_playing;
    }

    /// Calculates tempo ratio and synchronizes Deck B to Deck A BPM.
    pub fn sync_deck_b_to_a(&mut self) -> f32 {
        if self.deck_b_bpm > 10.0 {
            let ratio = self.deck_a_bpm / self.deck_b_bpm;
            let pitch = ((ratio - 1.0) * 100.0).clamp(-16.0, 16.0);
            self.deck_b_pitch = pitch;
            pitch
        } else {
            0.0
        }
    }

    /// Calculates tempo ratio and synchronizes Deck A to Deck B BPM.
    pub fn sync_deck_a_to_b(&mut self) -> f32 {
        if self.deck_a_bpm > 10.0 {
            let ratio = self.deck_b_bpm / self.deck_a_bpm;
            let pitch = ((ratio - 1.0) * 100.0).clamp(-16.0, 16.0);
            self.deck_a_pitch = pitch;
            pitch
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::{AudioSource, TrackMetadata};

    #[test]
    fn test_dj_audio_sync() {
        let mut state = DjAudioSharedState {
            deck_a_bpm: 130.0,
            deck_b_bpm: 125.0,
            ..Default::default()
        };

        let pitch = state.sync_deck_b_to_a();
        // 130 / 125 = 1.04 -> +4%
        assert!((pitch - 4.0).abs() < 0.01);
        assert!((state.deck_b_pitch - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_bpm_sync_ratio_calculation() {
        let mut state = DjAudioSharedState {
            deck_a_bpm: 120.0,
            deck_b_bpm: 126.0,
            ..Default::default()
        };

        let pitch_a = state.sync_deck_a_to_b();
        // 126 / 120 = 1.05 -> +5%
        assert!((pitch_a - 5.0).abs() < 0.01);
        assert!((state.deck_a_pitch - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_dj_load_tracks() {
        let mut state = DjAudioSharedState::default();
        let track_a = Track::new(
            AudioSource::LocalFile { path: "/a.mp3".into() },
            TrackMetadata::new("A", "Artist"),
        );
        let track_b = Track::new(
            AudioSource::LocalFile { path: "/b.mp3".into() },
            TrackMetadata::new("B", "Artist"),
        );

        state.load_deck_a(track_a);
        assert!(state.deck_a_track.is_some());
        assert!(state.deck_a_playing);
        assert!(state.enabled);

        state.load_deck_b(track_b);
        assert!(state.deck_b_track.is_some());
        assert!(state.deck_b_playing);
    }
}
