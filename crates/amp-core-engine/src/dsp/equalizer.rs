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
            is_enabled: true,
        };
        eq.recalculate();
        eq
    }

    /// Sets the gain in dB for a specific band index (0 to 9). Clamped between -12.0 and +12.0 dB.
    pub fn set_band_gain(&mut self, band_idx: usize, gain_db: f32) {
        if band_idx < 10 {
            self.gains[band_idx] = gain_db.clamp(-12.0, 12.0);
            self.recalculate_band(band_idx);
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
        let filter = BiquadFilter::new_peaking_eq(self.sample_rate, freq, 1.414, self.gains[idx]);
        self.left_filters[idx] = filter;
        self.right_filters[idx] = filter;
    }

    /// In-place processing of interleaved stereo PCM audio samples `[L, R, L, R, ...]`.
    #[inline(always)]
    #[allow(clippy::chunks_exact_to_as_chunks)]
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        if !self.is_enabled {
            return;
        }

        for frame in samples.chunks_exact_mut(2) {
            let mut l = frame[0];
            let mut r = frame[1];

            for i in 0..10 {
                l = self.left_filters[i].process_sample(l);
                r = self.right_filters[i].process_sample(r);
            }

            frame[0] = l;
            frame[1] = r;
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
}
