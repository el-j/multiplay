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

/// The DJ handover (`dj_host`) lines decks up using the reported delay, so it has to be the
/// real one: an impulse comes out `output_delay()` frames late (to within one frame).
#[test]
fn the_reported_delay_is_the_real_delay() {
    let mut r = PureResampler::new(44_100, 48_000, 2, 512).expect("resampler");
    let mut input = vec![0.0f32; 44_100 * 2];
    let at_in = 10_000;
    input[at_in * 2] = 1.0;
    input[at_in * 2 + 1] = 1.0;
    let out = r.process_interleaved(&input).expect("resample");
    let left: Vec<f32> = out.iter().step_by(2).copied().collect();
    let peak = left
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .unwrap()
        .0 as f64;
    let expected = at_in as f64 * 48_000.0 / 44_100.0 + r.output_delay() as f64;
    assert!(
        (peak - expected).abs() <= 1.0,
        "impulse at {peak}, reported delay predicts {expected:.1} (delay {})",
        r.output_delay()
    );
}

/// Audio fed in but still waiting for a full block counts as delay too: the DJ handover asks
/// "which source frame is audible now", and frames still inside the resampler are not.
#[test]
fn buffered_input_counts_towards_the_delay() {
    let mut r = PureResampler::new(44_100, 48_000, 2, 512).expect("resampler");
    let empty = r.latency_frames();
    assert_eq!(empty, r.output_delay() as f64);
    // 100 frames: less than one block, so nothing comes out yet.
    let out = r.process_interleaved(&[0.1f32; 200]).expect("resample");
    assert!(out.is_empty());
    let expected = r.output_delay() as f64 + 100.0 * 48_000.0 / 44_100.0;
    assert!(
        (r.latency_frames() - expected).abs() < 1e-9,
        "{}",
        r.latency_frames()
    );
}

/// What the DJ handover relies on: after feeding audio in any chunk sizes, the next output frame
/// is the source at `fed / source_rate - latency_frames() / target_rate`.
#[test]
fn latency_says_which_source_moment_comes_out_next() {
    let tone = |t: f64| (2.0 * PI * 440.0 * t).sin() * 0.5;
    for split in [333usize, 512, 700, 1001] {
        let mut r = PureResampler::new(44_100, 48_000, 2, 512).expect("resampler");
        let mut fed = 0usize;
        // Run in, then check at several points along the way.
        for round in 0..30 {
            let chunk: Vec<f32> = (fed..fed + split)
                .flat_map(|i| {
                    let v = tone(i as f64 / 44_100.0) as f32;
                    [v, v]
                })
                .collect();
            let latency = r.latency_frames();
            let next_at = fed as f64 / 44_100.0 - latency / 48_000.0;
            let out = r.process_interleaved(&chunk).expect("resample");
            fed += split;
            if round > 4 && !out.is_empty() {
                let expected = tone(next_at);
                assert!(
                    (f64::from(out[0]) - expected).abs() < 0.005,
                    "split {split}, round {round}: got {}, the source at {next_at:.6}s is {expected:.4}",
                    out[0]
                );
            }
        }
    }
}
