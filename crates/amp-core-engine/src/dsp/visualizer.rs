//! Real-time zero-allocation log-spaced mono FFT spectrum visualizer (`TASK-144`, `DSP-06`).
//!
//! Downmixes stereo to mono, calculates a 1024-point FFT with Hann window,
//! aggregates positive bins into 16 (or 75) log-spaced frequency bands (40 Hz to 16 kHz),
//! excludes DC, and converts magnitude to dB-scaled bars with peak hold and decay.

#![forbid(unsafe_code)]

use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const FFT_SIZE: usize = 1024;
pub const HALF_FFT: usize = FFT_SIZE / 2;
pub const NUM_BANDS: usize = 16;
pub const NUM_BANDS_75: usize = 75;

pub struct SpectrumVisualizer {
    window: [f32; FFT_SIZE],
    input_buffer: [f32; FFT_SIZE],
    write_cursor: usize,
    fft: Arc<dyn Fft<f32>>,
    fft_buffer: [Complex<f32>; FFT_SIZE],
    magnitudes: [f32; HALF_FFT],
    decaying_bands: [f32; NUM_BANDS],
    decaying_bands_75: [f32; NUM_BANDS_75],
    sample_rate: f32,
    /// When [`Self::spectrum_bands_throttled`] last ran the FFT.
    last_fft: Option<Instant>,
    fft_runs: u64,
}

/// Shortest time between two spectrum FFTs: one 60 Hz display frame. Audio arrives every few
/// milliseconds; running the FFT for each chunk only cost CPU (TASK-179).
pub const SPECTRUM_INTERVAL: Duration = Duration::from_millis(16);

impl SpectrumVisualizer {
    /// Creates a new spectrum visualizer with precomputed Hann window and reusable FFT scratchpads.
    pub fn new() -> Self {
        Self::with_sample_rate(44100.0)
    }

    /// Creates a visualizer configured for the specified sample rate.
    pub fn with_sample_rate(sample_rate: f32) -> Self {
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
            decaying_bands: [0.0; NUM_BANDS],
            decaying_bands_75: [0.0; NUM_BANDS_75],
            last_fft: None,
            fft_runs: 0,
            sample_rate: sample_rate.max(8000.0),
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(8000.0);
    }

    /// Pushes incoming interleaved stereo PCM samples `[L, R, L, R, ...]`.
    /// Downmixes each frame to mono `0.5 * (L + R)` before FFT analysis (`TASK-144`, `DSP-06`).
    #[allow(clippy::chunks_exact_to_as_chunks)]
    pub fn push_samples(&mut self, samples: &[f32]) {
        for frame in samples.chunks_exact(2) {
            let mono = (frame[0] + frame[1]) * 0.5;
            self.input_buffer[self.write_cursor] = mono;
            self.write_cursor = (self.write_cursor + 1) % FFT_SIZE;
        }
    }

    /// Pushes mono PCM samples directly.
    pub fn push_mono_samples(&mut self, mono_samples: &[f32]) {
        for &s in mono_samples {
            self.input_buffer[self.write_cursor] = s;
            self.write_cursor = (self.write_cursor + 1) % FFT_SIZE;
        }
    }

    fn run_fft(&mut self) {
        // Window the circular buffer into the FFT buffer
        for i in 0..FFT_SIZE {
            let sample_idx = (self.write_cursor + i) % FFT_SIZE;
            let windowed = self.input_buffer[sample_idx] * self.window[i];
            self.fft_buffer[i] = Complex::new(windowed, 0.0);
        }

        // In-place FFT transformation
        self.fft.process(&mut self.fft_buffer);

        // Compute normalized magnitude spectrum for the positive frequency bins (bin 0 is DC)
        let norm_factor = 2.0 / FFT_SIZE as f32;
        for i in 0..HALF_FFT {
            self.magnitudes[i] = self.fft_buffer[i].norm() * norm_factor;
        }
    }

    /// The 16 bands, recomputed at most once per [`SPECTRUM_INTERVAL`]; in between, the last
    /// bands. What the playback path calls for every chunk it pushes.
    pub fn spectrum_bands_throttled(&mut self) -> [f32; NUM_BANDS] {
        let now = Instant::now();
        if self
            .last_fft
            .is_some_and(|last| now.duration_since(last) < SPECTRUM_INTERVAL)
        {
            return self.decaying_bands;
        }
        self.last_fft = Some(now);
        self.compute_spectrum_bands()
    }

    /// How many times the FFT has run.
    pub fn fft_runs(&self) -> u64 {
        self.fft_runs
    }

    /// Computes 16 log-spaced normalized frequency bands (0.0 to 1.0) with dB scaling,
    /// peak hold, and smooth decay (`TASK-144`, `DSP-06`).
    pub fn compute_spectrum_bands(&mut self) -> [f32; NUM_BANDS] {
        self.fft_runs += 1;
        self.run_fft();

        let bin_hz = self.sample_rate / FFT_SIZE as f32;
        let min_freq = 40.0f32;
        let max_freq = 16000.0f32.min(self.sample_rate * 0.49);
        let ratio = (max_freq / min_freq).powf(1.0 / NUM_BANDS as f32);

        let mut current_bands = [0.0f32; NUM_BANDS];

        for (band_idx, band_val) in current_bands.iter_mut().enumerate() {
            let f_low = min_freq * ratio.powi(band_idx as i32);
            let f_high = min_freq * ratio.powi((band_idx + 1) as i32);

            // Exclude DC (bin 0)
            let mut start_bin = (f_low / bin_hz).floor() as usize;
            if start_bin == 0 {
                start_bin = 1;
            }
            let mut end_bin = (f_high / bin_hz).ceil() as usize;
            if end_bin <= start_bin {
                end_bin = start_bin + 1;
            }
            let end_bin = end_bin.min(HALF_FFT);

            let mut peak_mag = 0.0f32;
            for i in start_bin..end_bin {
                if self.magnitudes[i] > peak_mag {
                    peak_mag = self.magnitudes[i];
                }
            }

            // Convert to dB (-60 dBFS to 0 dBFS mapped to 0.0 ..= 1.0)
            let db = 20.0 * (peak_mag + 1e-5).log10();
            let norm = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
            *band_val = norm;
        }

        // Peak hold with decay
        let decay_factor = 0.82f32;
        for (i, &b) in current_bands.iter().enumerate() {
            if b >= self.decaying_bands[i] {
                self.decaying_bands[i] = b;
            } else {
                self.decaying_bands[i] = (self.decaying_bands[i] * decay_factor).max(b);
            }
        }

        self.decaying_bands
    }

    /// Computes 75 log-spaced bands for classic Winamp full-width spectrum analyzer.
    pub fn compute_spectrum_bands_75(&mut self) -> [f32; NUM_BANDS_75] {
        self.run_fft();

        let bin_hz = self.sample_rate / FFT_SIZE as f32;
        let min_freq = 40.0f32;
        let max_freq = 16000.0f32.min(self.sample_rate * 0.49);
        let ratio = (max_freq / min_freq).powf(1.0 / NUM_BANDS_75 as f32);

        let mut current_bands = [0.0f32; NUM_BANDS_75];

        for (band_idx, band_val) in current_bands.iter_mut().enumerate() {
            let f_low = min_freq * ratio.powi(band_idx as i32);
            let f_high = min_freq * ratio.powi((band_idx + 1) as i32);

            let mut start_bin = (f_low / bin_hz).floor() as usize;
            if start_bin == 0 {
                start_bin = 1;
            }
            let mut end_bin = (f_high / bin_hz).ceil() as usize;
            if end_bin <= start_bin {
                end_bin = start_bin + 1;
            }
            let end_bin = end_bin.min(HALF_FFT);

            let mut peak_mag = 0.0f32;
            for i in start_bin..end_bin {
                if self.magnitudes[i] > peak_mag {
                    peak_mag = self.magnitudes[i];
                }
            }

            let db = 20.0 * (peak_mag + 1e-5).log10();
            *band_val = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
        }

        let decay_factor = 0.85f32;
        for (i, &b) in current_bands.iter().enumerate() {
            if b >= self.decaying_bands_75[i] {
                self.decaying_bands_75[i] = b;
            } else {
                self.decaying_bands_75[i] = (self.decaying_bands_75[i] * decay_factor).max(b);
            }
        }

        self.decaying_bands_75
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
            assert!(b < 0.01, "Silence must yield near-zero bar: {b}");
        }
    }

    #[test]
    fn test_spectrum_visualizer_mono_downmix_phase_cancellation() {
        let mut vis = SpectrumVisualizer::new();
        // Out of phase stereo: L = 1.0, R = -1.0. Downmix (L + R) / 2 == 0.0
        let stereo: Vec<f32> = (0..FFT_SIZE).flat_map(|_| [0.8f32, -0.8f32]).collect();
        vis.push_samples(&stereo);
        let bands = vis.compute_spectrum_bands();
        for &b in &bands {
            assert!(
                b < 0.01,
                "Out-of-phase stereo downmix must produce 0.0: {b}"
            );
        }
    }

    #[test]
    fn test_spectrum_visualizer_1khz_lights_exact_band() {
        let rate = 48000.0;
        let mut vis = SpectrumVisualizer::with_sample_rate(rate);

        // Generate full-scale 1000 Hz pure sine wave
        let frames = FFT_SIZE * 2;
        let mut samples = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            let s = (2.0 * PI * 1000.0 * i as f32 / rate).sin() * 0.9;
            samples.push(s);
            samples.push(s);
        }

        vis.push_samples(&samples);
        let bands = vis.compute_spectrum_bands();

        // Find the index of the highest band
        let (max_idx, &max_val) = bands
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();

        // At 48 kHz with 16 bands between 40 Hz and 16 kHz:
        // Band 7: 550 Hz .. 800 Hz
        // Band 8: 800 Hz .. 1163 Hz (contains 1000 Hz!)
        assert_eq!(
            max_idx, 8,
            "1 kHz sine must light band 8 (800 Hz..1163 Hz). Maximum was at {max_idx}"
        );
        assert!(max_val > 0.8, "Target 1 kHz band must be high: {max_val}");

        // Distant bands (e.g. sub-bass band 0 and treble band 15) must be low
        assert!(bands[0] < 0.2, "Band 0 (40 Hz) must be low: {}", bands[0]);
        assert!(
            bands[15] < 0.2,
            "Band 15 (16 kHz) must be low: {}",
            bands[15]
        );
    }
}
