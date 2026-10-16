//! Digital peaking IIR biquad filter.
//! Computes audio filter coefficients according to the Audio EQ Cookbook.

#![forbid(unsafe_code)]

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy)]
pub struct BiquadFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadFilter {
    /// Creates a peaking EQ biquad filter with specified center frequency, Q-factor, and gain in dB.
    pub fn new_peaking_eq(sample_rate: f32, center_freq: f32, q: f32, db_gain: f32) -> Self {
        let a = 10.0f32.powf(db_gain / 40.0);
        let omega = 2.0 * PI * center_freq / sample_rate;
        let alpha = omega.sin() / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * omega.cos();
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * omega.cos();
        let a2 = 1.0 - alpha / a;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Processes a single sample in-place without memory allocation.
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1 - self.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }

    /// Resets internal delay line buffers (e.g. on seek or track change).
    pub fn reset_state(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biquad_unity_gain() {
        let mut filter = BiquadFilter::new_peaking_eq(44100.0, 1000.0, 1.414, 0.0);
        // At 0 dB gain, input sample should equal output sample asymptotically
        let mut output = 0.0;
        for _ in 0..100 {
            output = filter.process_sample(0.5);
        }
        assert!((output - 0.5).abs() < 1e-4);
    }
}
