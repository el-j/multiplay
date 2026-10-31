//! Tests for Perceptual Volume Taper and Smoothing (`TASK-146`, `DSP-08`).

#![forbid(unsafe_code)]

use amp_core_engine::dsp::chain::{
    linear_to_volume_slider, volume_slider_to_linear, PlaybackDspChain,
};

#[test]
fn test_volume_slider_midpoint_is_minus_20db() {
    let linear_mid = volume_slider_to_linear(0.5);
    let db_mid = 20.0 * linear_mid.log10();

    // Midpoint should be approximately -20 dB (linear 0.10)
    assert!(
        (db_mid - (-20.0)).abs() < 1.5,
        "Midpoint {linear_mid} was {db_mid} dB, expected ~ -20 dB"
    );

    // Endpoints
    assert_eq!(volume_slider_to_linear(0.0), 0.0);
    assert!((volume_slider_to_linear(1.0) - 1.0).abs() < 1e-6);

    // Roundtrip
    let back = linear_to_volume_slider(linear_mid);
    assert!((back - 0.5).abs() < 1e-4);
}

#[test]
fn test_volume_smoothing_prevents_zipper_noise() {
    let rate = 48000;
    let mut chain = PlaybackDspChain::new(rate, 512);

    // Generate 1 kHz sine tone
    let frames = 512;
    let mut pcm = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let s = (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / rate as f32).sin() * 0.8;
        pcm.push(s);
        pcm.push(s);
    }

    let flat_gains = [0.0f32; 10];

    // Slew down to 0.0 (~0.2s of audio)
    for _ in 0..20 {
        let _ = chain.process_interleaved(&pcm, 0.0, 0.0, &flat_gains, false);
    }
    let (out_muted, _) = chain.process_interleaved(&pcm, 0.0, 0.0, &flat_gains, false);
    assert!(out_muted.iter().all(|&s| s.abs() < 1e-5));

    // Jump volume to 1.0
    let (out_stepped, _) = chain.process_interleaved(&pcm, 1.0, 0.0, &flat_gains, false);

    // Ensure volume ramps smoothly without a massive instantaneous derivative discontinuity
    let mut max_delta = 0.0f32;
    for w in out_stepped.windows(2) {
        let delta = (w[1] - w[0]).abs();
        if delta > max_delta {
            max_delta = delta;
        }
    }
    // High-frequency zipper clicks produce delta spikes > 1.0; smooth ramp stays well below
    assert!(
        max_delta < 0.8,
        "Max delta {max_delta} indicates zipper noise"
    );
}
