//! 10-Band graphic equalizer with standard Winamp center frequencies.
//! 60 Hz, 170 Hz, 310 Hz, 600 Hz, 1 kHz, 3 kHz, 6 kHz, 12 kHz, 14 kHz, 16 kHz.

#![forbid(unsafe_code)]

use super::biquad::BiquadFilter;

pub const EQ_FREQUENCIES: [f32; 10] = [
    60.0, 170.0, 310.0, 600.0, 1000.0, 3000.0, 6000.0, 12000.0, 14000.0, 16000.0,
];

pub struct GraphicEqualizer {
    left_filters: [BiquadFilter; 10],
    right_filters: [BiquadFilter; 10],
    sample_rate: f32,
    gains: [f32; 10], // dB gains (-12.0 to +12.0 dB)
    preamp_gain: f32, // Preamp dB gain (-12.0 to +12.0 dB)
    preamp_linear: f32,
    is_enabled: bool,
}

impl GraphicEqualizer {
    /// Creates a new 10-band stereo graphic equalizer initialized to 0 dB flat response.
    pub fn new(sample_rate: f32) -> Self {
        let dummy = BiquadFilter::new_peaking_eq(sample_rate, 1000.0, 1.414, 0.0);
        let mut eq = Self {
            left_filters: [dummy; 10],
            right_filters: [dummy; 10],
            sample_rate,
            gains: [0.0; 10],
            preamp_gain: 0.0,
            preamp_linear: 1.0,
            is_enabled: true,
        };
        eq.recalculate();
        eq
    }

    /// Sets the preamp gain in dB (-12.0 to +12.0 dB).
    pub fn set_preamp_gain(&mut self, gain_db: f32) {
        let clamped = gain_db.clamp(-12.0, 12.0);
        if (self.preamp_gain - clamped).abs() > 0.005 {
            self.preamp_gain = clamped;
            self.preamp_linear = 10.0f32.powf(clamped / 20.0);
        }
    }

    pub fn preamp_gain(&self) -> f32 {
        self.preamp_gain
    }

    /// Sets the gain in dB for a specific band index (0 to 9). Clamped between -12.0 and +12.0 dB.
    pub fn set_band_gain(&mut self, band_idx: usize, gain_db: f32) {
        if band_idx < 10 {
            let clamped = gain_db.clamp(-12.0, 12.0);
            if (self.gains[band_idx] - clamped).abs() > 0.005 {
                self.gains[band_idx] = clamped;
                self.recalculate_band(band_idx);
            }
        }
    }

    /// Returns the active gains for all 10 bands.
    pub fn gains(&self) -> [f32; 10] {
        self.gains
    }

    /// Enables or disables equalizer filtering (bypass switch).
    pub fn set_enabled(&mut self, enabled: bool) {
        self.is_enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    /// Updates the sample rate and recalculates all filter coefficients.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.recalculate();
    }

    fn recalculate(&mut self) {
        for i in 0..10 {
            self.recalculate_band(i);
        }
    }

    fn recalculate_band(&mut self, idx: usize) {
        let freq = EQ_FREQUENCIES[idx];
        self.left_filters[idx].update_peaking_eq(self.sample_rate, freq, 1.414, self.gains[idx]);
        self.right_filters[idx].update_peaking_eq(self.sample_rate, freq, 1.414, self.gains[idx]);
    }

    /// In-place processing of interleaved stereo PCM audio samples `[L, R, L, R, ...]`.
    #[inline(always)]
    #[allow(clippy::chunks_exact_to_as_chunks)]
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        if !self.is_enabled {
            return;
        }

        for frame in samples.chunks_exact_mut(2) {
            let mut l = frame[0] * self.preamp_linear;
            let mut r = frame[1] * self.preamp_linear;

            for i in 0..10 {
                l = self.left_filters[i].process_sample(l);
                r = self.right_filters[i].process_sample(r);
            }

            frame[0] = soft_limit(l);
            frame[1] = soft_limit(r);
        }
    }

    /// Resets internal filter delay states.
    pub fn reset(&mut self) {
        for f in &mut self.left_filters {
            f.reset_state();
        }
        for f in &mut self.right_filters {
            f.reset_state();
        }
    }
}

/// Audiophile transparent soft-knee saturation/limiter curve.
/// Linear up to 0.85 amplitude; smoothly compresses above 0.85, guaranteeing
/// output never clips or wraps past [-1.0, 1.0].
#[inline(always)]
fn soft_limit(x: f32) -> f32 {
    if x.abs() <= 0.85 {
        x
    } else if x > 0.85 {
        let excess = x - 0.85;
        0.85 + (excess / (1.0 + excess * 2.0)) * 0.15
    } else {
        let excess = -x - 0.85;
        -(0.85 + (excess / (1.0 + excess * 2.0)) * 0.15)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equalizer_flat_response() {
        let mut eq = GraphicEqualizer::new(44100.0);
        let mut buffer = [0.5f32, -0.5f32, 0.25f32, -0.25f32];
        let original = buffer;

        eq.process_interleaved(&mut buffer);

        // At flat gain, processing should not blow up or invert
        for i in 0..buffer.len() {
            assert!((buffer[i] - original[i]).abs() < 0.1);
        }
    }

    #[test]
    fn test_equalizer_gain_clamping() {
        let mut eq = GraphicEqualizer::new(48000.0);
        eq.set_band_gain(0, 25.0);
        assert_eq!(eq.gains()[0], 12.0);

        eq.set_band_gain(0, -30.0);
        assert_eq!(eq.gains()[0], -12.0);
    }

    #[test]
    fn test_eq_headroom_and_limiter() {
        let mut eq = GraphicEqualizer::new(44100.0);
        // Boost all bands by maximum +12 dB
        for i in 0..10 {
            eq.set_band_gain(i, 12.0);
        }
        eq.set_preamp_gain(6.0); // Extreme boost

        let mut loud_buffer = [1.0f32, -1.0f32, 0.95f32, -0.95f32];
        eq.process_interleaved(&mut loud_buffer);

        // Soft limiter must guarantee all output samples strictly within [-1.0, 1.0]
        for &s in &loud_buffer {
            assert!(s.is_finite());
            assert!((-1.0..=1.0).contains(&s), "Sample out of bounds: {s}");
        }
    }
}

