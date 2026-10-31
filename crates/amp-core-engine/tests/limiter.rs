//! Tests for Lookahead Oversampled True-Peak Limiter (`TASK-141`, `DSP-03`).

#![forbid(unsafe_code)]

use amp_core_engine::dsp::limiter::{LimiterConfig, LookaheadTruePeakLimiter};

#[test]
fn test_limiter_transparent_below_threshold() {
    let mut limiter = LookaheadTruePeakLimiter::new(48000.0, LimiterConfig::default());

    // Clean sine at -6 dBFS (linear ~0.5), well below the -1.0 dBTP ceiling (~0.891)
    let freq = 1000.0;
    let frames = 2048;
    let mut pcm = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let s = (2.0 * std::f32::consts::PI * freq * i as f32 / 48000.0).sin() * 0.5;
        pcm.push(s);
        pcm.push(s);
    }

    let original = pcm.clone();
    limiter.process_interleaved(&mut pcm);

    // After lookahead delay settles (~96 frames = 192 samples), output must match input
    let delay_samples = limiter.lookahead_frames() * 2;
    for i in delay_samples..pcm.len() {
        let diff = (pcm[i] - original[i - delay_samples]).abs();
        assert!(
            diff < 1e-4,
            "Sample at {i} distorted below threshold: diff={diff}"
        );
    }
}

#[test]
fn test_limiter_sine_sweep_plus_6db_stays_below_ceiling() {
    let mut limiter = LookaheadTruePeakLimiter::new(
        48000.0,
        LimiterConfig {
            ceiling_dbtp: -1.0,
            lookahead_ms: 2.0,
            release_ms: 50.0,
        },
    );

    let ceiling_linear = 10.0f32.powf(-1.0 / 20.0); // ~0.89125

    // Sine sweep at +6 dB gain (peak amplitude ~2.0)
    let frames = 48000; // 1 second
    let mut pcm = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let freq = 100.0 + (i as f32 / frames as f32) * 5000.0;
        let s = (2.0 * std::f32::consts::PI * freq * i as f32 / 48000.0).sin() * 2.0; // +6 dB hot
        pcm.push(s);
        pcm.push(s);
    }

    limiter.process_interleaved(&mut pcm);

    // Verify true-peak oversampled ceiling is strictly observed across the whole sweep
    let delay_samples = limiter.lookahead_frames() * 2;
    for &sample in &pcm[delay_samples..] {
        assert!(
            sample.abs() <= ceiling_linear + 0.005,
            "Sample {sample} exceeded -1 dBTP ceiling ({ceiling_linear})"
        );
    }
}
