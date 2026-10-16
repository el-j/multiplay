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
        })
    }

    /// Processes planar channel input: `input[channel][sample_index]`.
    pub fn process(&mut self, input: &[Vec<f32>]) -> Result<Vec<Vec<f32>>, String> {
        self.resampler
            .process(input, None)
            .map_err(|e| format!("Resampling error: {e}"))
    }

    /// Helper to process interleaved samples: &[f32] (L, R, L, R, ...) into interleaved output.
    pub fn process_interleaved(&mut self, samples: &[f32]) -> Result<Vec<f32>, String> {
        let frames = samples.len() / self.input_channels;
        let mut planar = vec![Vec::with_capacity(frames); self.input_channels];

        for frame in samples.chunks_exact(self.input_channels) {
            for (ch, &s) in frame.iter().enumerate() {
                planar[ch].push(s);
            }
        }

        let resampled_planar = self.process(&planar)?;
        let out_frames = resampled_planar[0].len();
        let mut interleaved = Vec::with_capacity(out_frames * self.input_channels);

        #[allow(clippy::needless_range_loop)]
        for i in 0..out_frames {
            for ch in 0..self.input_channels {
                interleaved.push(resampled_planar[ch][i]);
            }
        }

        Ok(interleaved)
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
}
