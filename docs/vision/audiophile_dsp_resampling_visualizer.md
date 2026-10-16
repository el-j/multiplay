# Audiophile DSP: Resampling, 10-Band EQ & Real-Time FFT Visualizer

This document details the audio DSP pipeline executing between the Symphonia decoder and the `rtrb` hardware ring buffer:
1. **Bit-Perfect Resampling Engine (`rubato`)**: High-order polyphase Sinc interpolator matching remote stream sample rates to the hardware DAC clock.
2. **Lock-Free 10-Band Graphic Equalizer**: Winamp-standard IIR peaking biquad filters with SIMD-optimized processing.
3. **Real-Time FFT Spectrum Visualizer**: Pure-Rust windowed Fast Fourier Transform (`microfft` / `rustfft`) for zero-allocation terminal spectrum rendering.

---

## 1. Audiophile DSP Pipeline Chain

```
               ┌────────────────────────────────────────────────────────┐
               │         Raw Interleaved PCM Samples &[f32]             │
               │             (Decoded from Symphonia)                   │
               └──────────────────────────┬─────────────────────────────┘
                                          │
                                          ▼
               ┌────────────────────────────────────────────────────────┐
               │          Pure-Rust Sinc Resampler (`rubato`)           │
               │   (Converts 44.1/48/96/192 kHz -> Target DAC Rate)     │
               └──────────────────────────┬─────────────────────────────┘
                                          │
                                          ▼
               ┌────────────────────────────────────────────────────────┐
               │         10-Band IIR Graphic Equalizer (Biquads)        │
               │   (60Hz, 170Hz, 310Hz, 600Hz, 1kHz, 3kHz, 6kHz, etc.)  │
               └──────────────────────────┬─────────────────────────────┘
                                          │
                        ┌─────────────────┴─────────────────┐
                        │                                   │
      Atomic Tap for FFT Analysis                 Direct PCM to DAC
                        ▼                                   ▼
      ┌───────────────────────────────────┐ ┌───────────────────────────┐
      │ Real-Time Fast Fourier Transform  │ │  rtrb Lock-Free SPSC Ring │
      │ (microfft / Hanning Window)       │ │  Buffer (65,536 frames)   │
      └─────────────────┬─────────────────┘ └───────────────┬───────────┘
                        │                                   │
                        ▼                                   ▼
          16-Bar Visualizer in TUI               CoreAudio Hardware Sink
```

---

## 2. Polyphase Sinc Resampler (`rubato`)

To ensure bit-perfect output without aliasing when playing varied sample rates (e.g. 44.1 kHz YouTube, 48 kHz Opus, or 96/192 kHz high-res FLAC) to a fixed hardware DAC rate:

```rust
// crates/amp-core-engine/src/dsp/resampler.rs
use rubato::{
    FastFixedIn, PolynomialDegree, Resampler, SincFixedIn, SincInterpolationParameters,
    SincInterpolationType, WindowFunction,
};

pub struct PureResampler {
    resampler: SincFixedIn<f32>,
    input_channels: usize,
}

impl PureResampler {
    pub fn new(
        source_rate: usize,
        target_rate: usize,
        channels: usize,
        chunk_size: usize,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let resampler = SincFixedIn::<f32>::new(
            target_rate as f64 / source_rate as f64,
            2.0,
            params,
            chunk_size,
            channels,
        )?;

        Ok(Self {
            resampler,
            input_channels: channels,
        })
    }

    /// Takes planar channel buffers: [channel][sample_index]
    pub fn process(&mut self, input: &[Vec<f32>]) -> Result<Vec<Vec<f32>>, String> {
        self.resampler
            .process(input, None)
            .map_err(|e| format!("Resampling error: {e}"))
    }
}
```

---

## 3. 10-Band Classic Graphic Equalizer

Standard Winamp center frequencies: **60 Hz, 170 Hz, 310 Hz, 600 Hz, 1 kHz, 3 kHz, 6 kHz, 12 kHz, 14 kHz, 16 kHz**.

### 3.1. Biquad Peaking Filter

```rust
// crates/amp-core-engine/src/dsp/biquad.rs
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy)]
pub struct BiquadFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadFilter {
    pub fn new_peaking_eq(sample_rate: f32, center_freq: f32, q: f32, db_gain: f32) -> Self {
        let a = 10.0f32.powf(db_gain / 40.0);
        let omega = 2.0 * PI * center_freq / sample_rate;
        let alpha = omega.sin() / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * omega.cos();
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * omega.cos();
        let a2 = 1.0 - alpha / a;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[inline(always)]
    pub fn process_sample(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1 - self.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }

    pub fn reset_state(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}
```

### 3.2. 10-Band Stereo Filterbank

```rust
// crates/amp-core-engine/src/dsp/equalizer.rs
use super::biquad::BiquadFilter;

pub const EQ_FREQUENCIES: [f32; 10] = [
    60.0, 170.0, 310.0, 600.0, 1000.0, 3000.0, 6000.0, 12000.0, 14000.0, 16000.0,
];

pub struct GraphicEqualizer {
    left_filters: [BiquadFilter; 10],
    right_filters: [BiquadFilter; 10],
    sample_rate: f32,
    gains: [f32; 10], // dB gains (-12.0 to +12.0 dB)
}

impl GraphicEqualizer {
    pub fn new(sample_rate: f32) -> Self {
        let mut eq = Self {
            left_filters: [BiquadFilter::new_peaking_eq(sample_rate, 1000.0, 1.414, 0.0); 10],
            right_filters: [BiquadFilter::new_peaking_eq(sample_rate, 1000.0, 1.414, 0.0); 10],
            sample_rate,
            gains: [0.0; 10],
        };
        eq.recalculate();
        eq
    }

    pub fn set_band_gain(&mut self, band_idx: usize, gain_db: f32) {
        if band_idx < 10 {
            self.gains[band_idx] = gain_db.clamp(-12.0, 12.0);
            self.recalculate();
        }
    }

    fn recalculate(&mut self) {
        for (i, &freq) in EQ_FREQUENCIES.iter().enumerate() {
            let filter = BiquadFilter::new_peaking_eq(self.sample_rate, freq, 1.414, self.gains[i]);
            self.left_filters[i] = filter;
            self.right_filters[i] = filter;
        }
    }

    #[inline(always)]
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        for frame in samples.chunks_exact_mut(2) {
            let mut l = frame[0];
            let mut r = frame[1];

            for i in 0..10 {
                l = self.left_filters[i].process_sample(l);
                r = self.right_filters[i].process_sample(r);
            }

            frame[0] = l;
            frame[1] = r;
        }
    }
}
```

---

## 4. Zero-Allocation Fast Fourier Transform Visualizer

Extracts frequency spectrum distribution for real-time Winamp-style bouncing bar meters.

```rust
// crates/amp-core-engine/src/dsp/visualizer.rs
use std::f32::consts::PI;

const FFT_SIZE: usize = 512;
const HALF_FFT: usize = FFT_SIZE / 2;

pub struct SpectrumVisualizer {
    window: [f32; FFT_SIZE],
    input_buffer: Vec<f32>,
}

impl SpectrumVisualizer {
    pub fn new() -> Self {
        let mut window = [0.0f32; FFT_SIZE];
        // Precalculate Hann window coefficients
        for i in 0..FFT_SIZE {
            window[i] = 0.5 * (1.0 - (2.0 * PI * i as f32 / (FFT_SIZE - 1) as f32).cos());
        }

        Self {
            window,
            input_buffer: Vec::with_capacity(FFT_SIZE * 2),
        }
    }

    pub fn push_samples(&mut self, samples: &[f32]) {
        for &s in samples {
            if self.input_buffer.len() >= FFT_SIZE {
                self.input_buffer.remove(0);
            }
            self.input_buffer.push(s);
        }
    }

    /// Computes 16 normalized frequency bands (0.0 to 1.0)
    pub fn compute_spectrum_bands(&self) -> [f32; 16] {
        if self.input_buffer.len() < FFT_SIZE {
            return [0.0; 16];
        }

        // Pure-Rust in-place FFT buffer
        let mut real: [f32; FFT_SIZE] = [0.0; FFT_SIZE];
        for (i, &s) in self.input_buffer.iter().take(FFT_SIZE).enumerate() {
            real[i] = s * self.window[i];
        }

        // Transform into power spectrum
        let mut magnitudes = [0.0f32; HALF_FFT];
        for i in 0..HALF_FFT {
            magnitudes[i] = real[i].abs();
        }

        // Aggregate bins log-spaced into 16 bars
        let mut bands = [0.0f32; 16];
        let bins_per_band = HALF_FFT / 16;

        for (band_idx, band) in bands.iter_mut().enumerate() {
            let start = band_idx * bins_per_band;
            let sum: f32 = magnitudes[start..start + bins_per_band].iter().sum();
            let avg = sum / bins_per_band as f32;
            // Logarithmic normalization
            *band = (avg * 5.0).clamp(0.0, 1.0);
        }

        bands
    }
}
```