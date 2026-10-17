//! Pure-Rust audiophile headphone crossfeed DSP (Bauer / Jan Meier binaural simulation).
//! Simulates loudspeaker acoustic delay and head-shadow filtering to reduce headphone listening fatigue.

#![forbid(unsafe_code)]

use std::f32::consts::PI;

/// Selectable crossfeed intensity profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum CrossfeedProfile {
    #[default]
    Off,
    Subtle,
    Standard,
    Deep,
}

/// Zero-allocation real-time stereophonic crossfeed filter for headphones.
#[derive(Debug, Clone)]
pub struct HeadphoneCrossfeed {
    profile: CrossfeedProfile,
    sample_rate: f32,
    lpf_coeff: f32,
    delay_samples: usize,
    prev_low_l: f32,
    prev_low_r: f32,
    delay_buf_l: [f32; 128],
    delay_buf_r: [f32; 128],
    delay_idx: usize,
}

impl HeadphoneCrossfeed {
    /// Creates a new headphone crossfeed processor for the specified sample rate.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let mut xfeed = Self {
            profile: CrossfeedProfile::Off,
            sample_rate: sample_rate.max(8000.0),
            lpf_coeff: 0.0,
            delay_samples: 14,
            prev_low_l: 0.0,
            prev_low_r: 0.0,
            delay_buf_l: [0.0; 128],
            delay_buf_r: [0.0; 128],
            delay_idx: 0,
        };
        xfeed.update_coefficients();
        xfeed
    }

    /// Updates the sample rate and recalculates internal filter coefficients and delays.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(8000.0);
        self.update_coefficients();
    }

    /// Sets the active crossfeed intensity profile.
    pub fn set_profile(&mut self, profile: CrossfeedProfile) {
        self.profile = profile;
    }

    /// Returns the currently active profile.
    #[must_use]
    pub const fn profile(&self) -> CrossfeedProfile {
        self.profile
    }

    fn update_coefficients(&mut self) {
        // Human head acoustic shadow cutoff ~ 700 Hz
        let cutoff_hz = 700.0f32;
        let omega = 2.0 * PI * cutoff_hz / self.sample_rate;
        self.lpf_coeff = (omega / (1.0 + omega)).clamp(0.001, 0.999);

        // ~ 280 microseconds interaural delay (approx 17-21 cm ear distance)
        let delay = (0.000280 * self.sample_rate).round() as usize;
        self.delay_samples = delay.clamp(1, 127);
    }

    /// Resets internal delay lines and filter registers.
    pub fn reset(&mut self) {
        self.prev_low_l = 0.0;
        self.prev_low_r = 0.0;
        self.delay_buf_l = [0.0; 128];
        self.delay_buf_r = [0.0; 128];
        self.delay_idx = 0;
    }

    /// Processes interleaved stereo PCM samples in-place.
    ///
    /// # Real-time Safety
    /// Performs zero allocations, zero lock waits, and is real-time thread safe.
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        if self.profile == CrossfeedProfile::Off || samples.len() < 2 {
            return;
        }

        let (cross_gain, direct_gain) = match self.profile {
            CrossfeedProfile::Off => return,
            CrossfeedProfile::Subtle => (0.24, 0.94),
            CrossfeedProfile::Standard => (0.35, 0.90),
            CrossfeedProfile::Deep => (0.47, 0.85),
        };

        let frames = samples.len() / 2;
        for f in 0..frames {
            let idx_l = f * 2;
            let idx_r = f * 2 + 1;
            let in_l = samples[idx_l];
            let in_r = samples[idx_r];

            // 1st order IIR low-pass filtering on each ear
            self.prev_low_l += self.lpf_coeff * (in_l - self.prev_low_l);
            self.prev_low_r += self.lpf_coeff * (in_r - self.prev_low_r);

            // Store in circular delay buffer
            self.delay_buf_l[self.delay_idx] = self.prev_low_l;
            self.delay_buf_r[self.delay_idx] = self.prev_low_r;

            // Fetch delayed opposite-channel shadow signal
            let read_idx = (self.delay_idx + 128 - self.delay_samples) % 128;
            let delayed_low_l = self.delay_buf_l[read_idx];
            let delayed_low_r = self.delay_buf_r[read_idx];

            self.delay_idx = (self.delay_idx + 1) % 128;

            // Mix: direct attenuated channel + delayed low-passed opposite channel
            samples[idx_l] = direct_gain * in_l + cross_gain * delayed_low_r;
            samples[idx_r] = direct_gain * in_r + cross_gain * delayed_low_l;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crossfeed_bypass() {
        let mut xf = HeadphoneCrossfeed::new(48000.0);
        xf.set_profile(CrossfeedProfile::Off);

        let mut samples = vec![1.0, 0.0, 1.0, 0.0];
        let original = samples.clone();
        xf.process_interleaved(&mut samples);
        assert_eq!(samples, original);
    }

    #[test]
    fn test_crossfeed_hard_panned_signal_bleed() {
        let mut xf = HeadphoneCrossfeed::new(48000.0);
        xf.set_profile(CrossfeedProfile::Standard);

        // Hard-panned signal on left channel (100 frames)
        let mut samples = Vec::new();
        for _ in 0..100 {
            samples.push(1.0); // Left
            samples.push(0.0); // Right
        }

        xf.process_interleaved(&mut samples);

        // After interaural delay (14 samples), right channel should have non-zero low-pass bleed
        let right_sample_later = samples[51]; // Right channel around frame 25
        assert!(
            right_sample_later > 0.05,
            "Expected crossfeed bleed into right channel, got {}",
            right_sample_later
        );
    }
}
