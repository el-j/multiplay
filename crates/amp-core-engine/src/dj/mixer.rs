//! Professional dual-deck DJ mixing engine.
//! Zero-allocation real-time audio channel processing, crossfader curves,
//! 3-band isolator EQs, and WSOLA pitch-preserving time stretching.

#![forbid(unsafe_code)]

use super::beat_detector::RealtimeBpmDetector;
use super::deck::{DeckChannelState, DeckId};
use super::wsola::{CrossfaderCurveProfile, DjCrossfader, WsolaTimeStretcher};

/// Professional dual-deck DJ mixer engine.
#[derive(Debug)]
pub struct DualDeckMixer {
    pub deck_a: DeckChannelState,
    pub deck_b: DeckChannelState,
    pub crossfader: DjCrossfader,
    pub deck_a_stretcher: WsolaTimeStretcher,
    pub deck_b_stretcher: WsolaTimeStretcher,
    pub key_lock_a: bool,
    pub key_lock_b: bool,
    pub deck_a_bpm: RealtimeBpmDetector,
    pub deck_b_bpm: RealtimeBpmDetector,
    scratch_a: Vec<f32>,
    scratch_b: Vec<f32>,
    stretched_a: Vec<f32>,
    stretched_b: Vec<f32>,
}

impl DualDeckMixer {
    /// Creates a new dual-deck DJ mixer configured for given sample rate and frame buffer capacity.
    #[must_use]
    pub fn new(sample_rate: f32, max_frames: usize) -> Self {
        let max_samples = max_frames * 2;
        let window_size = 512;

        Self {
            deck_a: DeckChannelState::new(DeckId::DeckA, sample_rate),
            deck_b: DeckChannelState::new(DeckId::DeckB, sample_rate),
            crossfader: DjCrossfader::new(),
            deck_a_stretcher: WsolaTimeStretcher::new(window_size),
            deck_b_stretcher: WsolaTimeStretcher::new(window_size),
            key_lock_a: true,
            key_lock_b: true,
            deck_a_bpm: RealtimeBpmDetector::new(sample_rate, 512),
            deck_b_bpm: RealtimeBpmDetector::new(sample_rate, 512),
            scratch_a: vec![0.0; max_samples],
            scratch_b: vec![0.0; max_samples],
            stretched_a: vec![0.0; max_samples],
            stretched_b: vec![0.0; max_samples],
        }
    }

    /// Sets crossfader position (-1.0 = 100% Deck A, 0.0 = Center, +1.0 = 100% Deck B)
    /// and transition curve profile.
    pub fn set_crossfader(&mut self, position: f32, profile: CrossfaderCurveProfile) {
        self.crossfader.set_position(position);
        self.crossfader.set_profile(profile);
    }

    /// Synchronizes Deck B tempo to match Deck A's current BPM.
    /// Returns the computed pitch percent applied to Deck B.
    pub fn sync_deck_b_to_a(&mut self) -> Option<f32> {
        let bpm_a = self
            .deck_a
            .bpm
            .unwrap_or_else(|| self.deck_a_bpm.estimated_bpm());
        let bpm_b = self
            .deck_b
            .bpm
            .unwrap_or_else(|| self.deck_b_bpm.estimated_bpm());

        if bpm_b > 10.0 {
            let ratio = bpm_a / bpm_b;
            let pitch_percent = ((ratio - 1.0) * 100.0).clamp(-16.0, 16.0);
            self.deck_b.set_pitch_percent(pitch_percent);
            Some(pitch_percent)
        } else {
            None
        }
    }

    /// Synchronizes Deck A tempo to match Deck B's current BPM.
    /// Returns the computed pitch percent applied to Deck A.
    pub fn sync_deck_a_to_b(&mut self) -> Option<f32> {
        let bpm_a = self
            .deck_a
            .bpm
            .unwrap_or_else(|| self.deck_a_bpm.estimated_bpm());
        let bpm_b = self
            .deck_b
            .bpm
            .unwrap_or_else(|| self.deck_b_bpm.estimated_bpm());

        if bpm_a > 10.0 {
            let ratio = bpm_b / bpm_a;
            let pitch_percent = ((ratio - 1.0) * 100.0).clamp(-16.0, 16.0);
            self.deck_a.set_pitch_percent(pitch_percent);
            Some(pitch_percent)
        } else {
            None
        }
    }

    /// Real-time zero-allocation processing and mixing of Deck A and Deck B streams.
    /// Applies WSOLA time stretching, 3-band isolator EQ, fader volumes, and crossfade curves.
    pub fn process_and_mix(
        &mut self,
        deck_a_in: &[f32],
        deck_b_in: &[f32],
        output: &mut [f32],
    ) -> usize {
        let out_len = output.len();
        if out_len == 0 {
            return 0;
        }

        // 1. Process Deck A
        let samples_a = self.process_deck_channel(true, deck_a_in, out_len);

        // 2. Process Deck B
        let samples_b = self.process_deck_channel(false, deck_b_in, out_len);

        let max_samples = samples_a.max(samples_b).min(out_len);
        let (gain_a, gain_b) = self.crossfader.compute_gains();

        // 3. Sum and soft-saturate outputs
        for (i, out_sample) in output.iter_mut().enumerate().take(max_samples) {
            let sa = if i < samples_a {
                self.scratch_a[i]
            } else {
                0.0
            };
            let sb = if i < samples_b {
                self.scratch_b[i]
            } else {
                0.0
            };
            let mixed = (sa * gain_a) + (sb * gain_b);

            // Clean musical soft-saturation prevents harsh digital clipping
            *out_sample = if mixed > 1.0 {
                1.0
            } else if mixed < -1.0 {
                -1.0
            } else {
                mixed - (mixed * mixed * mixed) * 0.1667
            };
        }

        max_samples
    }

    #[inline]
    fn process_deck_channel(&mut self, is_deck_a: bool, input: &[f32], out_cap: usize) -> usize {
        let is_playing = if is_deck_a {
            self.deck_a.is_playing
        } else {
            self.deck_b.is_playing
        };

        if !is_playing || input.is_empty() {
            return 0;
        }

        let pitch = if is_deck_a {
            self.deck_a.pitch_percent
        } else {
            self.deck_b.pitch_percent
        };
        let key_lock = if is_deck_a {
            self.key_lock_a
        } else {
            self.key_lock_b
        };
        let vol_fader = if is_deck_a {
            self.deck_a.volume_fader
        } else {
            self.deck_b.volume_fader
        };

        let target_len = input.len().min(out_cap);

        // Time stretching if pitch is adjusted with key lock
        let processed_samples = if key_lock && pitch.abs() > 0.05 {
            let speed = (1.0 + pitch / 100.0).clamp(0.5, 2.0);
            if is_deck_a {
                self.deck_a_stretcher.set_speed_ratio(speed);
                self.deck_a_stretcher
                    .process(&input[..target_len], &mut self.stretched_a[..target_len])
            } else {
                self.deck_b_stretcher.set_speed_ratio(speed);
                self.deck_b_stretcher
                    .process(&input[..target_len], &mut self.stretched_b[..target_len])
            }
        } else {
            if is_deck_a {
                self.stretched_a[..target_len].copy_from_slice(&input[..target_len]);
            } else {
                self.stretched_b[..target_len].copy_from_slice(&input[..target_len]);
            }
            target_len
        };

        let src = if is_deck_a {
            &self.stretched_a[..processed_samples]
        } else {
            &self.stretched_b[..processed_samples]
        };

        // Process isolator EQ and volume fader
        for (i, &raw_sample) in src.iter().enumerate().take(processed_samples) {
            let eq_sample = if is_deck_a {
                self.deck_a.isolator_eq.process_sample(raw_sample)
            } else {
                self.deck_b.isolator_eq.process_sample(raw_sample)
            };

            let final_sample = eq_sample * vol_fader;
            if is_deck_a {
                self.scratch_a[i] = final_sample;
            } else {
                self.scratch_b[i] = final_sample;
            }
        }

        // Live BPM transient detection
        if is_deck_a {
            let _ = self
                .deck_a_bpm
                .process_block(&self.scratch_a[..processed_samples]);
            self.deck_a.bpm = Some(self.deck_a_bpm.estimated_bpm());
        } else {
            let _ = self
                .deck_b_bpm
                .process_block(&self.scratch_b[..processed_samples]);
            self.deck_b.bpm = Some(self.deck_b_bpm.estimated_bpm());
        }

        processed_samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_deck_mixer_crossfading() {
        let mut mixer = DualDeckMixer::new(44100.0, 1024);
        mixer.deck_a.set_playing(true);
        mixer.deck_b.set_playing(true);

        // Deck A: pure 0.8, Deck B: pure 0.4
        let deck_a_in = vec![0.8_f32; 256];
        let deck_b_in = vec![0.4_f32; 256];
        let mut output = vec![0.0_f32; 256];

        // 1. Center fader with SmoothDip
        mixer.set_crossfader(0.0, CrossfaderCurveProfile::SmoothDip);
        let n = mixer.process_and_mix(&deck_a_in, &deck_b_in, &mut output);
        assert_eq!(n, 256);
        assert!(output[0] > 0.0);

        // 2. Full Deck A (-1.0)
        mixer.set_crossfader(-1.0, CrossfaderCurveProfile::Linear);
        mixer.process_and_mix(&deck_a_in, &deck_b_in, &mut output);
        assert!((output[0] - 0.715).abs() < 0.1); // Deck A with soft clip

        // 3. Full Deck B (+1.0)
        mixer.set_crossfader(1.0, CrossfaderCurveProfile::Linear);
        mixer.process_and_mix(&deck_a_in, &deck_b_in, &mut output);
        assert!((output[0] - 0.39).abs() < 0.05); // Deck B with soft clip
    }

    #[test]
    fn test_dual_deck_mixer_kill_eq() {
        let mut mixer = DualDeckMixer::new(44100.0, 512);
        mixer.deck_a.set_playing(true);
        mixer.deck_a.isolator_eq.kill_low = true;
        mixer.deck_a.isolator_eq.kill_mid = true;
        mixer.deck_a.isolator_eq.kill_high = true;

        let deck_a_in = vec![0.5_f32; 128];
        let deck_b_in = vec![0.0_f32; 128];
        let mut output = vec![0.0_f32; 128];

        mixer.set_crossfader(-1.0, CrossfaderCurveProfile::Linear);
        let n = mixer.process_and_mix(&deck_a_in, &deck_b_in, &mut output);
        assert_eq!(n, 128);
        assert_eq!(output[0], 0.0, "Triple kill should mute Deck A completely");
    }

    #[test]
    fn test_dual_deck_mixer_sync() {
        let mut mixer = DualDeckMixer::new(44100.0, 512);
        mixer.deck_a.bpm = Some(126.0);
        mixer.deck_b.bpm = Some(120.0);

        let pitch = mixer.sync_deck_b_to_a().expect("Sync should compute pitch");
        // 126 / 120 = 1.05 -> +5.0%
        assert!((pitch - 5.0).abs() < 0.01);
        assert!((mixer.deck_b.pitch_percent - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_dual_deck_mixer_concurrent_streaming() {
        let mut mixer = DualDeckMixer::new(44100.0, 512);
        mixer.deck_a.set_playing(true);
        mixer.deck_b.set_playing(true);
        mixer.deck_a.set_pitch_percent(4.0);
        mixer.deck_b.set_pitch_percent(-2.0);
        let da = vec![0.3_f32; 256];
        let db = vec![0.2_f32; 256];
        let mut out = vec![0.0_f32; 256];
        let n = mixer.process_and_mix(&da, &db, &mut out);
        assert_eq!(n, 256);
        assert!(out.iter().any(|&s| s != 0.0));
    }
}
