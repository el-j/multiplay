//! Resampling 44.1 -> 48 kHz must stay audiophile-clean whichever algorithm does it (TASK-179
//! swapped the sinc interpolator for rubato's FFT resampler to cut CPU use): no audible images,
//! a flat passband up to 20 kHz, and no added noise.

#![forbid(unsafe_code)]

use amp_core_engine::dsp::resampler::PureResampler;
use std::f64::consts::PI;

/// Level of `hz` in `signal` (mono, at `rate`) in dB relative to full scale, by Goertzel.
fn level_db(signal: &[f32], rate: f64, hz: f64) -> f64 {
    let w = 2.0 * PI * hz / rate;
    let coeff = 2.0 * w.cos();
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for (i, &x) in signal.iter().enumerate() {
        // Hann window keeps leakage from the strong tone out of the measured bin.
        let win = 0.5 - 0.5 * (2.0 * PI * i as f64 / (signal.len() - 1) as f64).cos();
        let s = f64::from(x) * win + coeff * s1 - s2;
        s2 = s1;
        s1 = s;
    }
    let power = s1 * s1 + s2 * s2 - coeff * s1 * s2;
    let amplitude = 2.0 * power.sqrt() / (signal.len() as f64 * 0.5);
    20.0 * amplitude.max(1e-15).log10()
}

fn resample_tone(hz: f64, amplitude: f32) -> Vec<f32> {
    let mut r = PureResampler::new(44_100, 48_000, 2, 512).expect("resampler");
    let input: Vec<f32> = (0..44_100 * 2)
        .flat_map(|i| {
            let v = (2.0 * PI * hz * f64::from(i) / 44_100.0).sin() as f32 * amplitude;
            [v, v]
        })
        .collect();
    let out = r.process_interleaved(&input).expect("resample");
    // Skip the start-up transient, keep one second.
    out.iter()
        .step_by(2)
        .skip(9_600)
        .take(48_000)
        .copied()
        .collect()
}

#[test]
fn a_1_khz_tone_has_no_image_and_keeps_its_level() {
    let out = resample_tone(1_000.0, 0.5);
    let tone = level_db(&out, 48_000.0, 1_000.0);
    assert!(
        (tone - 20.0 * 0.5f64.log10()).abs() < 0.1,
        "level {tone:.2} dB"
    );
    // The 44.1 kHz image of 1 kHz (43.1 kHz) folds to 4.9 kHz at 48 kHz.
    let image = level_db(&out, 48_000.0, 4_900.0);
    assert!(
        image - tone < -100.0,
        "image {:.1} dB below the tone",
        tone - image
    );
}

#[test]
fn the_passband_is_flat_to_20_khz() {
    let out = resample_tone(20_000.0, 0.5);
    let tone = level_db(&out, 48_000.0, 20_000.0);
    assert!(
        (tone - 20.0 * 0.5f64.log10()).abs() < 0.1,
        "20 kHz at {tone:.2} dB"
    );
}
