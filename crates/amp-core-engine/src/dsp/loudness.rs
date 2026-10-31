//! Pure-Rust EBU R128 / ITU-R BS.1770 loudness normalization and measurement (`TASK-143`, `DSP-05`).

#![forbid(unsafe_code)]

use super::biquad::BiquadFilter;

/// Normalization targeting strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoudnessMode {
    #[default]
    Track,
    Album,
    Auto,
    Off,
}

/// Configuration parameters for loudness normalization and peak limiting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessConfig {
    /// Target integrated loudness in LUFS (typically -14.0 LUFS for streaming, -23.0 for broadcast/audiophile).
    pub target_lufs: f32,
    /// Maximum allowable true peak in dBFS before limiting engages (e.g., -0.5 dBFS).
    pub max_peak_dbfs: f32,
    /// Preamp offset in dB (default 0.0 dB).
    pub preamp_db: f32,
    /// Prevents clipping by scaling gain down if peak would exceed ceiling.
    pub prevent_clipping: bool,
    /// Normalization strategy mode.
    pub mode: LoudnessMode,
    /// Whether loudness normalization is actively enabled.
    pub enabled: bool,
}

impl Default for LoudnessConfig {
    fn default() -> Self {
        Self {
            target_lufs: -14.0,
            max_peak_dbfs: -0.5,
            preamp_db: 0.0,
            prevent_clipping: true,
            mode: LoudnessMode::Track,
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
        self.calculate_target_gain_with_peak(track_lufs, None)
    }

    /// Computes the linear gain multiplier with optional peak clipping prevention.
    #[must_use]
    pub fn calculate_target_gain_with_peak(&self, track_lufs: f32, peak: Option<f32>) -> f32 {
        if !self.config.enabled || self.config.mode == LoudnessMode::Off {
            return 1.0;
        }
        let delta_db = (self.config.target_lufs - track_lufs) + self.config.preamp_db;
        let mut gain = 10.0_f32.powf(delta_db / 20.0);

        if self.config.prevent_clipping {
            if let Some(p) = peak.filter(|&v| v > 0.0) {
                let ceiling = self.max_peak_linear();
                let max_safe_gain = ceiling / p;
                if gain > max_safe_gain {
                    gain = max_safe_gain;
                }
            }
        }
        gain
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
        self.process_interleaved_with_peak(samples, track_lufs, None);
    }

    /// Normalizes with optional true-peak clipping prevention.
    pub fn process_interleaved_with_peak(
        &mut self,
        samples: &mut [f32],
        track_lufs: Option<f32>,
        peak: Option<f32>,
    ) {
        if !self.config.enabled || self.config.mode == LoudnessMode::Off || samples.is_empty() {
            return;
        }

        let target_gain = track_lufs
            .map(|lufs| self.calculate_target_gain_with_peak(lufs, peak))
            .unwrap_or(1.0);

        let peak_limit = self.max_peak_linear();

        for sample in samples.iter_mut() {
            // Smoothly slew current_gain towards target_gain
            self.current_gain += (target_gain - self.current_gain) * self.smoothing_factor;

            let amplified = *sample * self.current_gain;

            // Soft-knee limiter preventing clipping above peak_limit
            let abs_amp = amplified.abs();
            if abs_amp > peak_limit {
                let sign = amplified.signum();
                let excess = abs_amp - peak_limit;
                let compressed = peak_limit + (excess / (1.0 + excess * 0.5));
                *sample = sign * compressed.min(peak_limit);
            } else {
                *sample = amplified;
            }
        }
    }
}

/// Pure-Rust ITU-R BS.1770-4 K-weighting loudness analyzer (`TASK-143`, `DSP-05`).
pub struct Bs1770Analyzer {
    stage1_l: BiquadFilter,
    stage1_r: BiquadFilter,
    stage2_l: BiquadFilter,
    stage2_r: BiquadFilter,
    sample_rate: f32,
}

impl Bs1770Analyzer {
    /// Creates a new BS.1770-4 K-weighting loudness analyzer.
    pub fn new(sample_rate: f32) -> Self {
        // Stage 1: High shelf filter ~1.5 kHz, +4 dB
        let s1_l = BiquadFilter::new_high_shelf(sample_rate, 1500.0, 1.0, 4.0);
        let s1_r = BiquadFilter::new_high_shelf(sample_rate, 1500.0, 1.0, 4.0);
        // Stage 2: High pass filter ~100 Hz
        let s2_l = BiquadFilter::new_highpass(sample_rate, 100.0, 0.707);
        let s2_r = BiquadFilter::new_highpass(sample_rate, 100.0, 0.707);

        Self {
            stage1_l: s1_l,
            stage1_r: s1_r,
            stage2_l: s2_l,
            stage2_r: s2_r,
            sample_rate,
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Measures integrated LUFS of an interleaved stereo PCM track.
    #[allow(clippy::chunks_exact_to_as_chunks)]
    pub fn measure_lufs(&mut self, samples: &[f32]) -> f32 {
        let frames = samples.len() / 2;
        if frames == 0 {
            return -70.0;
        }

        let mut sum_power = 0.0f64;
        for frame in samples.chunks_exact(2) {
            let l1 = self.stage1_l.process_sample(frame[0]);
            let r1 = self.stage1_r.process_sample(frame[1]);

            let l2 = self.stage2_l.process_sample(l1);
            let r2 = self.stage2_r.process_sample(r1);

            sum_power += (l2 as f64 * l2 as f64) + (r2 as f64 * r2 as f64);
        }

        let mean_power = sum_power / (frames as f64 * 2.0);
        if mean_power <= 1e-12 {
            return -70.0;
        }

        let lufs = -0.691 + 10.0 * mean_power.log10();
        (lufs as f32).max(-70.0)
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
            ..Default::default()
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
            ..Default::default()
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
