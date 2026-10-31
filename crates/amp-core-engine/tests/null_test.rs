//! CI Bit-Transparent Null Test (`TASK-140`, `DSP-01`, `DSP-02`).
//!
//! Asserts that when DSP is neutral (unity volume, flat EQ / disabled, crossfeed off,
//! matching sample rate, and no loudness gain), the output is an exact bit-for-bit
//! match to the input (null test difference is exactly 0.0, identical bit representations).
//!
//! Also asserts that enabling a +3 dB band changes output, and disabling returns to exact identity.

#![forbid(unsafe_code)]

use amp_core_engine::dsp::{CrossfeedProfile, PlaybackDspChain};

#[test]
fn bit_transparent_default_path_exact_null_test() {
    let rate = 48000;
    let mut chain = PlaybackDspChain::new(rate, 512);
    chain.set_source_sample_rate(rate);
    chain.set_crossfeed_profile(CrossfeedProfile::Off);

    // Realistic audio samples including near-full-scale values (0.95 FS)
    // that previously triggered the soft-limit clipper above 0.85 FS.
    let mut input = Vec::with_capacity(1024);
    for i in 0..512 {
        let t = i as f32 / rate as f32;
        let l = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.95;
        let r = (2.0 * std::f32::consts::PI * 880.0 * t).cos() * 0.95;
        input.push(l);
        input.push(r);
    }

    let flat_gains = [0.0f32; 10];

    // 1. Path is bit-perfect: volume 1.0, preamp 0.0, flat gains, EQ enabled/disabled
    assert!(
        chain.is_bit_perfect(1.0, 0.0, &flat_gains, true, None),
        "DSP chain should report bit-perfect for neutral settings with flat EQ"
    );

    let (output_flat, _) = chain.process_interleaved(&input, 1.0, 0.0, &flat_gains, true);

    // Exact bit-for-bit null test
    assert_eq!(output_flat.len(), input.len());
    for (i, (&in_samp, &out_samp)) in input.iter().zip(output_flat.iter()).enumerate() {
        assert_eq!(
            in_samp.to_bits(),
            out_samp.to_bits(),
            "Sample at index {i} failed exact null test: in={in_samp} out={out_samp}"
        );
    }

    // 2. Enabling a +3 dB band modifies the audio
    let mut boosted_gains = flat_gains;
    boosted_gains[4] = 3.0; // +3 dB at 1 kHz
    let (output_boosted, _) = chain.process_interleaved(&input, 1.0, 0.0, &boosted_gains, true);
    assert_ne!(
        output_boosted, input,
        "Boosting a band must alter the output"
    );

    // 3. Disabling EQ returns to exact identity
    let (output_disabled, _) = chain.process_interleaved(&input, 1.0, 0.0, &boosted_gains, false);
    for (i, (&in_samp, &out_samp)) in input.iter().zip(output_disabled.iter()).enumerate() {
        assert_eq!(
            in_samp.to_bits(),
            out_samp.to_bits(),
            "Disabled EQ failed exact null test at index {i}"
        );
    }
}
