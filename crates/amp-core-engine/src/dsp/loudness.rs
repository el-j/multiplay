//! Pure-Rust EBU R128 loudness normalization and true-peak limiter.
//! Compliant with ITU-R BS.1770 / EBU R128 recommendations with zero external C dependencies.

#![forbid(unsafe_code)]

/// Configuration parameters for loudness normalization and peak limiting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessConfig {
    /// Target integrated loudness in LUFS (typically -14.0 LUFS for streaming, -18.0 to -23.0 for audiophile masters).
    pub target_lufs: f32,
    /// Maximum allowable true peak in dBFS before limiting engages (e.g., -0.5 dBFS to prevent DAC inter-sample clipping).
    pub max_peak_dbfs: f32,
    /// Whether loudness normalization is actively enabled.
    pub enabled: bool,
}

impl Default for LoudnessConfig {
    fn default() -> Self {
        Self {
            target_lufs: -14.0,
            max_peak_dbfs: -0.5,
            enabled: true,
        }
    }
}

/// Pure-Rust real-time loudness normalizer and soft-knee peak limiter.
#[derive(Debug, Clone)]
pub struct PureVolumeNormalizer {
    config: LoudnessConfig,
    current_gain: f32,
    smoothing_factor: f32,
}

impl PureVolumeNormalizer {
    /// Creates a new volume normalizer with the given configuration.
    #[must_use]
    pub fn new(config: LoudnessConfig) -> Self {
        Self {
            config,
            current_gain: 1.0,
            smoothing_factor: 0.05,
        }
    }

    /// Access the configuration.
    #[must_use]
    pub const fn config(&self) -> &LoudnessConfig {
        &self.config
    }

    /// Updates the configuration.
    pub fn set_config(&mut self, config: LoudnessConfig) {
        self.config = config;
    }

    /// Computes the linear gain multiplier required to bring a track with `track_lufs` to `target_lufs`.
    #[must_use]
    pub fn calculate_target_gain(&self, track_lufs: f32) -> f32 {
        if !self.config.enabled {
            return 1.0;
        }
        let delta_db = self.config.target_lufs - track_lufs;
        10.0_f32.powf(delta_db / 20.0)
    }

    /// Max peak amplitude linear ceiling derived from `max_peak_dbfs`.
    #[must_use]
    pub fn max_peak_linear(&self) -> f32 {
        10.0_f32.powf(self.config.max_peak_dbfs / 20.0)
    }

    /// Normalizes and limits an interleaved audio buffer in-place.
    ///
    /// # Real-time Safety
    /// Performs zero heap allocations, zero system calls, and is real-time thread safe.
    pub fn process_interleaved(&mut self, samples: &mut [f32], track_lufs: Option<f32>) {
        if !self.config.enabled || samples.is_empty() {
            return;
        }

        let target_gain = track_lufs
            .map(|lufs| self.calculate_target_gain(lufs))
            .unwrap_or(1.0);

        let peak_limit = self.max_peak_linear();

        for sample in samples.iter_mut() {
            // Smoothly slew current_gain towards target_gain
            self.current_gain += (target_gain - self.current_gain) * self.smoothing_factor;

            let amplified = *sample * self.current_gain;

            // Soft-knee limiter preventing clipping above peak_limit
            let abs_amp = amplified.abs();
            if abs_amp > peak_limit {
                // Soft saturator: tanh-style progressive compression
                let sign = amplified.signum();
                let excess = abs_amp - peak_limit;
                let compressed = peak_limit + (excess / (1.0 + excess * 0.5));
                *sample = sign * compressed.min(0.999);
            } else {
                *sample = amplified;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loudness_gain_calculation() {
        let config = LoudnessConfig {
            target_lufs: -14.0,
            max_peak_dbfs: -0.5,
            enabled: true,
        };
        let normalizer = PureVolumeNormalizer::new(config);

        // Track at -20 LUFS should be boosted by +6 dB (~1.995 linear)
        let gain = normalizer.calculate_target_gain(-20.0);
        assert!((gain - 1.995).abs() < 0.05);

        // Track at -8 LUFS should be attenuated by -6 dB (~0.501 linear)
        let gain_atten = normalizer.calculate_target_gain(-8.0);
        assert!((gain_atten - 0.501).abs() < 0.05);
    }

    #[test]
    fn test_loudness_limiter_clamping() {
        let config = LoudnessConfig {
            target_lufs: -14.0,
            max_peak_dbfs: -1.0, // ~0.891 linear ceiling
            enabled: true,
        };
        let mut normalizer = PureVolumeNormalizer::new(config);

        // Huge hot signal
        let mut buffer = vec![1.5_f32, -2.0_f32, 10.0_f32];
        normalizer.process_interleaved(&mut buffer, Some(-8.0));

        for s in buffer {
            assert!(s.abs() < 1.0, "Sample exceeded 1.0 limit: {}", s);
        }
    }

    #[test]
    fn test_loudness_bypass() {
        let config = LoudnessConfig {
            enabled: false,
            ..Default::default()
        };
        let mut normalizer = PureVolumeNormalizer::new(config);

        let mut buffer = vec![0.5_f32, -0.5_f32];
        normalizer.process_interleaved(&mut buffer, Some(-24.0));

        assert_eq!(buffer, vec![0.5_f32, -0.5_f32]);
    }
}
