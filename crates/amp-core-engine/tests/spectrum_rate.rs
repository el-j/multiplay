//! The spectrum feeds a display that refreshes at most 60 times a second, so the engine runs the
//! FFT at most that often, however often audio is pushed (TASK-179: measured CPU budgets).

#![forbid(unsafe_code)]

use amp_core_engine::dsp::visualizer::{SpectrumVisualizer, SPECTRUM_INTERVAL};
use std::time::Duration;

fn tone(frames: usize) -> Vec<f32> {
    (0..frames)
        .flat_map(|i| {
            let s = (i as f32 * 0.13).sin() * 0.8;
            [s, s]
        })
        .collect()
}

#[test]
fn the_fft_runs_at_most_once_per_display_frame() {
    assert_eq!(SPECTRUM_INTERVAL, Duration::from_millis(16));
    let mut vis = SpectrumVisualizer::new();
    vis.push_samples(&tone(2048));
    let first = vis.spectrum_bands_throttled();
    assert!(
        first.iter().any(|&b| b > 0.1),
        "a tone lights bars: {first:?}"
    );

    // Silence pushed straight after: within the interval the last bars are kept.
    vis.push_samples(&vec![0.0; 4096]);
    assert_eq!(vis.spectrum_bands_throttled(), first);
    assert_eq!(vis.fft_runs(), 1);

    std::thread::sleep(SPECTRUM_INTERVAL + Duration::from_millis(4));
    let later = vis.spectrum_bands_throttled();
    assert_eq!(vis.fft_runs(), 2);
    assert!(
        later.iter().zip(first).all(|(l, f)| *l <= f),
        "bars fall after silence"
    );
}
