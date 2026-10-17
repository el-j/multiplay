//! Bit-perfect polyphase Sinc resampler using pure-Rust Rubato.
//! Converts arbitrary source sample rates (44.1, 48, 96, 192 kHz) to hardware output rates.

#![forbid(unsafe_code)]

use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

pub struct PureResampler {
    resampler: SincFixedIn<f32>,
    input_channels: usize,
    chunk_size: usize,
    source_rate: usize,
    target_rate: usize,
}

impl PureResampler {
    /// Creates a new polyphase Sinc resampler instance.
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
            chunk_size,
            source_rate,
            target_rate,
        })
    }

    /// Returns source sample rate in Hz.
    pub fn source_rate(&self) -> usize {
        self.source_rate
    }

    /// Returns target sample rate in Hz.
    pub fn target_rate(&self) -> usize {
        self.target_rate
    }

    /// Processes planar channel input: `input[channel][sample_index]`.
    pub fn process(&mut self, input: &[Vec<f32>]) -> Result<Vec<Vec<f32>>, String> {
        self.resampler
            .process(input, None)
            .map_err(|e| format!("Resampling error: {e}"))
    }

    /// Helper to process a single block of up to `chunk_size` interleaved frames.
    fn process_interleaved_block(&mut self, samples: &[f32]) -> Result<Vec<f32>, String> {
        let frames = samples.len() / self.input_channels;
        if frames == 0 {
            return Ok(Vec::new());
        }

        let mut planar = vec![vec![0.0f32; self.chunk_size]; self.input_channels];
        for (i, frame) in samples.chunks_exact(self.input_channels).enumerate() {
            if i >= self.chunk_size {
                break;
            }
            for (ch, &s) in frame.iter().enumerate() {
                planar[ch][i] = s;
            }
        }

        let resampled_planar = self.process(&planar)?;
        let expected_out_frames = if frames < self.chunk_size {
            ((frames as f64 * self.target_rate as f64 / self.source_rate as f64).round() as usize)
                .min(resampled_planar[0].len())
        } else {
            resampled_planar[0].len()
        };

        let mut interleaved = Vec::with_capacity(expected_out_frames * self.input_channels);
        #[allow(clippy::needless_range_loop)]
        for i in 0..expected_out_frames {
            for ch in 0..self.input_channels {
                interleaved.push(resampled_planar[ch][i]);
            }
        }

        Ok(interleaved)
    }

    /// Processes arbitrary length interleaved audio samples, automatically chunking as needed.
    pub fn process_interleaved(&mut self, samples: &[f32]) -> Result<Vec<f32>, String> {
        let step = self.chunk_size * self.input_channels;
        if samples.len() <= step {
            return self.process_interleaved_block(samples);
        }

        let mut all_out = Vec::new();
        for chunk in samples.chunks(step) {
            let res = self.process_interleaved_block(chunk)?;
            all_out.extend(res);
        }
        Ok(all_out)
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resampler_44100_to_48000() {
        let chunk_size = 1024;
        let mut resampler = PureResampler::new(44100, 48000, 2, chunk_size)
            .expect("resampler creation failed");

        // Generate 1024 frames of 440 Hz stereo sine wave
        let mut input_planar = vec![vec![0.0f32; chunk_size], vec![0.0f32; chunk_size]];
        #[allow(clippy::needless_range_loop)]
        for i in 0..chunk_size {
            let sample = (i as f32 * 0.05).sin();
            input_planar[0][i] = sample;
            input_planar[1][i] = sample;
        }

        let output = resampler.process(&input_planar).expect("resample failed");
        assert_eq!(output.len(), 2);
        assert!(!output[0].is_empty());

        let interleaved_in = vec![0.1f32; chunk_size * 2];
        let interleaved_out = resampler.process_interleaved(&interleaved_in).expect("interleaved resample failed");
        assert!(!interleaved_out.is_empty());
        assert_eq!(interleaved_out.len() % 2, 0); // Always balanced stereo pairs
    }

    #[test]
    fn test_resampler_arbitrary_lengths_and_rates() {
        let chunk_size = 512;
        let mut resampler = PureResampler::new(44100, 48000, 2, chunk_size)
            .expect("resampler creation failed");

        assert_eq!(resampler.source_rate(), 44100);
        assert_eq!(resampler.target_rate(), 48000);

        // Test with fewer samples than chunk_size (e.g. 100 frames = 200 samples)
        let small_input = vec![0.2f32; 200];
        let small_out = resampler.process_interleaved(&small_input).expect("small resample failed");
        assert!(!small_out.is_empty());
        assert_eq!(small_out.len() % 2, 0);

        // Test with more samples than chunk_size (e.g. 1200 frames = 2400 samples)
        let large_input = vec![0.3f32; 2400];
        let large_out = resampler.process_interleaved(&large_input).expect("large resample failed");
        assert!(!large_out.is_empty());
        assert_eq!(large_out.len() % 2, 0);
    }
}
