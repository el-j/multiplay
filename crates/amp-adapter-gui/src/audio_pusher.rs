//! Audio thread feeder helper: pushes processed audio to CPAL ring buffer and updates FFT visualizers.

#![forbid(unsafe_code)]

use crate::audio_driver::AudioPlaybackDriver;
use std::time::Duration;

pub fn update_visualizer_and_push(
    driver: &AudioPlaybackDriver,
    processed: &[f32],
    bands: [f32; 16],
) {
    if let Ok(mut spec) = driver.visualizer_spectrum.try_lock() {
        *spec = bands;
    }
    if let Ok(mut wave) = driver.visualizer_waveform.try_lock() {
        let step = (processed.len() / 64).max(1);
        for (dst_i, s) in wave.iter_mut().enumerate() {
            let src_i = (dst_i * step).min(processed.len().saturating_sub(1));
            *s = processed[src_i];
        }
    }

    if let Ok(mut prod) = driver.producer.try_lock() {
        let mut pushed = 0;
        while pushed < processed.len() {
            match prod.push(processed[pushed]) {
                Ok(()) => {
                    pushed += 1;
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        }
    }
}
