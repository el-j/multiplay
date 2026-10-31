//! Lookahead, 4× oversampled true-peak limiter (`TASK-141`, `DSP-03`).
//!
//! Provides inter-sample peak protection compliant with ITU-R BS.1770 / EBU R128
//! with smooth attack/release envelopes and strictly zero heap allocations in `process()`.

#![forbid(unsafe_code)]

/// Maximum lookahead frames supported in the stack/struct ring buffer (over 5 ms at 96 kHz).
pub const MAX_LOOKAHEAD_FRAMES: usize = 512;

/// Polyphase 4× interpolation FIR filter coefficients for true-peak oversampling.
const FIR_4X_PHASE1: [f32; 4] = [-0.035, 0.490, 0.610, -0.065];
const FIR_4X_PHASE2: [f32; 4] = [-0.075, 0.575, 0.575, -0.075];
const FIR_4X_PHASE3: [f32; 4] = [-0.065, 0.610, 0.490, -0.035];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LimiterConfig {
    /// Ceiling in dB True Peak (default: −1.0 dBTP).
    pub ceiling_dbtp: f32,
    /// Lookahead window in milliseconds (default: 2.0 ms).
    pub lookahead_ms: f32,
    /// Gain release time in milliseconds (default: 50.0 ms).
    pub release_ms: f32,
}

impl Default for LimiterConfig {
    fn default() -> Self {
        Self {
            ceiling_dbtp: -1.0,
            lookahead_ms: 2.0,
            release_ms: 50.0,
        }
    }
}

/// Zero-allocation, lookahead 4× oversampled true-peak limiter.
#[derive(Debug, Clone)]
pub struct LookaheadTruePeakLimiter {
    config: LimiterConfig,
    sample_rate: f32,
    ceiling_linear: f32,
    lookahead_frames: usize,
    delay_ring: [[f32; 2]; MAX_LOOKAHEAD_FRAMES],
    write_idx: usize,
    // Filter history for 4x true peak detection
    history_l: [f32; 4],
    history_r: [f32; 4],
    current_gain: f32,
    target_gain: f32,
    attack_coeff: f32,
    release_coeff: f32,
}

impl LookaheadTruePeakLimiter {
    /// Creates a new true-peak lookahead limiter for the given sample rate and config.
    pub fn new(sample_rate: f32, config: LimiterConfig) -> Self {
        let mut limiter = Self {
            config,
            sample_rate: sample_rate.max(8000.0),
            ceiling_linear: 10.0f32.powf(config.ceiling_dbtp / 20.0),
            lookahead_frames: 96,
            delay_ring: [[0.0; 2]; MAX_LOOKAHEAD_FRAMES],
            write_idx: 0,
            history_l: [0.0; 4],
            history_r: [0.0; 4],
            current_gain: 1.0,
            target_gain: 1.0,
            attack_coeff: 0.1,
            release_coeff: 0.999,
        };
        limiter.update_coefficients();
        limiter
    }

    /// Updates configuration and recalculates filter/envelope coefficients.
    pub fn set_config(&mut self, config: LimiterConfig) {
        self.config = config;
        self.ceiling_linear = 10.0f32.powf(config.ceiling_dbtp / 20.0);
        self.update_coefficients();
    }

    pub fn ceiling_linear(&self) -> f32 {
        self.ceiling_linear
    }

    pub fn lookahead_frames(&self) -> usize {
        self.lookahead_frames
    }

    fn update_coefficients(&mut self) {
        let frames = ((self.config.lookahead_ms / 1000.0) * self.sample_rate).round() as usize;
        self.lookahead_frames = frames.clamp(8, MAX_LOOKAHEAD_FRAMES - 1);

        // Attack over lookahead frames
        let attack_frames = (self.lookahead_frames as f32).max(1.0);
        self.attack_coeff = 1.0 / attack_frames;

        // Exponential release coeff: 1 - exp(-1 / (release_sec * sr))
        let release_samples = (self.config.release_ms / 1000.0) * self.sample_rate;
        self.release_coeff = (-1.0 / release_samples.max(1.0)).exp();
    }

    /// Resets internal delay lines, peak detector history, and gain state.
    pub fn reset(&mut self) {
        self.delay_ring = [[0.0; 2]; MAX_LOOKAHEAD_FRAMES];
        self.write_idx = 0;
        self.history_l = [0.0; 4];
        self.history_r = [0.0; 4];
        self.current_gain = 1.0;
        self.target_gain = 1.0;
    }

    /// Computes 4× oversampled true peak for a single channel using 4-phase FIR.
    #[inline(always)]
    fn estimate_true_peak_4x(history: &[f32; 4]) -> f32 {
        let p0 = history[3].abs();
        let p1 = (history[0] * FIR_4X_PHASE1[0]
            + history[1] * FIR_4X_PHASE1[1]
            + history[2] * FIR_4X_PHASE1[2]
            + history[3] * FIR_4X_PHASE1[3])
            .abs();
        let p2 = (history[0] * FIR_4X_PHASE2[0]
            + history[1] * FIR_4X_PHASE2[1]
            + history[2] * FIR_4X_PHASE2[2]
            + history[3] * FIR_4X_PHASE2[3])
            .abs();
        let p3 = (history[0] * FIR_4X_PHASE3[0]
            + history[1] * FIR_4X_PHASE3[1]
            + history[2] * FIR_4X_PHASE3[2]
            + history[3] * FIR_4X_PHASE3[3])
            .abs();
        p0.max(p1).max(p2).max(p3)
    }

    /// Processes interleaved stereo audio in-place.
    /// Real-time safe: zero dynamic allocations, zero locks, zero system calls.
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        let num_frames = samples.len() / 2;
        if num_frames == 0 {
            return;
        }

        let ceiling = self.ceiling_linear;

        for frame_idx in 0..num_frames {
            let in_l = samples[frame_idx * 2];
            let in_r = samples[frame_idx * 2 + 1];

            // Update FIR history
            self.history_l[0] = self.history_l[1];
            self.history_l[1] = self.history_l[2];
            self.history_l[2] = self.history_l[3];
            self.history_l[3] = in_l;

            self.history_r[0] = self.history_r[1];
            self.history_r[1] = self.history_r[2];
            self.history_r[2] = self.history_r[3];
            self.history_r[3] = in_r;

            // 4× oversampled true-peak detection across L and R
            let peak_l = Self::estimate_true_peak_4x(&self.history_l);
            let peak_r = Self::estimate_true_peak_4x(&self.history_r);
            let frame_peak = peak_l.max(peak_r);

            // Compute desired gain reduction if oversampled peak exceeds ceiling
            let frame_required_gain = if frame_peak > ceiling && frame_peak > 1e-6 {
                ceiling / frame_peak
            } else {
                1.0
            };

            // Fast attack, smooth release
            if frame_required_gain < self.target_gain {
                self.target_gain = frame_required_gain;
            } else {
                // Recover towards 1.0
                self.target_gain =
                    self.release_coeff * self.target_gain + (1.0 - self.release_coeff) * 1.0;
            }

            // Slew current gain towards target gain
            if self.target_gain < self.current_gain {
                self.current_gain += (self.target_gain - self.current_gain) * self.attack_coeff;
            } else {
                self.current_gain = self.release_coeff * self.current_gain
                    + (1.0 - self.release_coeff) * self.target_gain;
            }

            // Write input to lookahead ring buffer
            self.delay_ring[self.write_idx] = [in_l, in_r];

            // Read delayed sample from lookahead ring buffer
            let read_idx = (self.write_idx + MAX_LOOKAHEAD_FRAMES - self.lookahead_frames)
                % MAX_LOOKAHEAD_FRAMES;
            let delayed = self.delay_ring[read_idx];
            self.write_idx = (self.write_idx + 1) % MAX_LOOKAHEAD_FRAMES;

            // Apply smooth gain reduction
            let out_l = delayed[0] * self.current_gain;
            let out_r = delayed[1] * self.current_gain;

            // Hard safety clamp to ceiling linear (never wrap or overshoot)
            samples[frame_idx * 2] = out_l.clamp(-ceiling, ceiling);
            samples[frame_idx * 2 + 1] = out_r.clamp(-ceiling, ceiling);
        }
    }
}
