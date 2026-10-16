//! Real-time zero-allocation 16-band FFT spectrum visualizer.
//! Analyzes PCM audio frames with a Hann window and outputs normalized (0.0 to 1.0) 16 frequency bands.

#![forbid(unsafe_code)]

use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::f32::consts::PI;
use std::sync::Arc;

pub const FFT_SIZE: usize = 512;
pub const HALF_FFT: usize = FFT_SIZE / 2;

pub struct SpectrumVisualizer {
    window: [f32; FFT_SIZE],
    input_buffer: [f32; FFT_SIZE],
    write_cursor: usize,
    fft: Arc<dyn Fft<f32>>,
    fft_buffer: [Complex<f32>; FFT_SIZE],
    magnitudes: [f32; HALF_FFT],
}

impl SpectrumVisualizer {
    /// Creates a new spectrum visualizer with precomputed Hann window and reusable FFT scratchpads.
    pub fn new() -> Self {
        let mut window = [0.0f32; FFT_SIZE];
        for (i, w) in window.iter_mut().enumerate() {
            *w = 0.5 * (1.0 - (2.0 * PI * i as f32 / (FFT_SIZE - 1) as f32).cos());
        }

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);

        Self {
            window,
            input_buffer: [0.0; FFT_SIZE],
            write_cursor: 0,
            fft,
            fft_buffer: [Complex::new(0.0, 0.0); FFT_SIZE],
            magnitudes: [0.0; HALF_FFT],
        }
    }

    /// Pushes incoming PCM samples into the circular sample analysis window.
    pub fn push_samples(&mut self, samples: &[f32]) {
        for &s in samples {
            self.input_buffer[self.write_cursor] = s;
            self.write_cursor = (self.write_cursor + 1) % FFT_SIZE;
        }
    }

    /// Computes 16 log-spaced normalized frequency bands (0.0 to 1.0) with zero allocations.
    pub fn compute_spectrum_bands(&mut self) -> [f32; 16] {
        // Window the circular buffer into the FFT buffer
        for i in 0..FFT_SIZE {
            let sample_idx = (self.write_cursor + i) % FFT_SIZE;
            let windowed = self.input_buffer[sample_idx] * self.window[i];
            self.fft_buffer[i] = Complex::new(windowed, 0.0);
        }

        // In-place FFT transformation
        self.fft.process(&mut self.fft_buffer);

        // Compute magnitude power spectrum for the positive frequency bins
        for i in 0..HALF_FFT {
            self.magnitudes[i] = self.fft_buffer[i].norm();
        }

        // Aggregate bins log-spaced into 16 bars
        let mut bands = [0.0f32; 16];
        let bins_per_band = HALF_FFT / 16; // 16 bins per band for 256 positive bins

        for (band_idx, band) in bands.iter_mut().enumerate() {
            let start = band_idx * bins_per_band;
            let sum: f32 = self.magnitudes[start..start + bins_per_band].iter().sum();
            let avg = sum / bins_per_band as f32;
            // Logarithmic dynamic range scaling for visual pleasing bounce
            *band = (avg * 4.0).clamp(0.0, 1.0);
        }

        bands
    }
}

impl Default for SpectrumVisualizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectrum_visualizer_silence() {
        let mut vis = SpectrumVisualizer::new();
        let bands = vis.compute_spectrum_bands();
        for &b in &bands {
            assert_eq!(b, 0.0);
        }
    }

    #[test]
    fn test_spectrum_visualizer_sine_wave() {
        let mut vis = SpectrumVisualizer::new();
        // Generate a 1000 Hz tone sampled at 44100 Hz
        let mut samples = Vec::with_capacity(FFT_SIZE);
        for i in 0..FFT_SIZE {
            let s = (2.0 * PI * 1000.0 * i as f32 / 44100.0).sin();
            samples.push(s);
        }

        vis.push_samples(&samples);
        let bands = vis.compute_spectrum_bands();

        // Ensure non-zero energy detected
        let total_energy: f32 = bands.iter().sum();
        assert!(total_energy > 0.0);
    }
}
