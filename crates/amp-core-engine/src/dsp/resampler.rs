//! Bit-perfect polyphase Sinc resampler using pure-Rust Rubato.
//! Converts arbitrary source sample rates (44.1, 48, 96, 192 kHz) to hardware output rates.

#![forbid(unsafe_code)]

use rubato::audioadapter_buffers::direct::{InterleavedSlice, SequentialSliceOfVecs};
use rubato::{
    Async, FixedAsync, Indexing, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

/// Tells rubato how many frames of a short block are real (the rest is silence).
fn partial(frames: usize, chunk: usize) -> Option<Indexing> {
    (frames < chunk).then_some(Indexing {
        input_offset: 0,
        output_offset: 0,
        partial_len: Some(frames),
        active_channels_mask: None,
    })
}

pub struct PureResampler {
    resampler: Async<f32>,
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
            f_cutoff: Some(0.95),
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let resampler = Async::<f32>::new_sinc(
            target_rate as f64 / source_rate as f64,
            2.0,
            &params,
            chunk_size,
            channels,
            FixedAsync::Input,
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

    /// Processes planar channel input `input[channel][sample_index]` (one chunk) and returns
    /// planar output.
    pub fn process(&mut self, input: &[Vec<f32>]) -> Result<Vec<Vec<f32>>, String> {
        let frames = input.first().map_or(0, Vec::len);
        let adapter = SequentialSliceOfVecs::new(input, self.input_channels, frames)
            .map_err(|e| format!("Resampling input error: {e}"))?;
        let indexing = partial(frames, self.chunk_size);
        let out = self
            .resampler
            .process(&adapter, indexing.as_ref())
            .map_err(|e| format!("Resampling error: {e}"))?
            .take_data();
        let ch = self.input_channels.max(1);
        Ok((0..ch)
            .map(|c| out.iter().skip(c).step_by(ch).copied().collect())
            .collect())
    }

    /// Resamples one block of up to `chunk_size` interleaved frames.
    fn process_interleaved_block(&mut self, samples: &[f32]) -> Result<Vec<f32>, String> {
        let frames = samples.len() / self.input_channels;
        if frames == 0 {
            return Ok(Vec::new());
        }
        let input = InterleavedSlice::new(samples, self.input_channels, frames)
            .map_err(|e| format!("Resampling input error: {e}"))?;
        let indexing = partial(frames, self.chunk_size);
        let mut out = self
            .resampler
            .process(&input, indexing.as_ref())
            .map_err(|e| format!("Resampling error: {e}"))?
            .take_data();
        // A short final block is padded with silence inside rubato; keep only its share.
        if frames < self.chunk_size {
            let keep = (frames as f64 * self.target_rate as f64 / self.source_rate as f64).round()
                as usize;
            out.truncate(keep * self.input_channels);
        }
        Ok(out)
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

    /// Output frames by which the resampled audio lags its input.
    pub fn output_delay(&self) -> usize {
        self.resampler.output_delay()
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
        let mut resampler =
            PureResampler::new(44100, 48000, 2, chunk_size).expect("resampler creation failed");

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
        let interleaved_out = resampler
            .process_interleaved(&interleaved_in)
            .expect("interleaved resample failed");
        assert!(!interleaved_out.is_empty());
        assert_eq!(interleaved_out.len() % 2, 0); // Always balanced stereo pairs
    }

    #[test]
    fn resampled_tone_keeps_its_pitch() {
        let mut r = PureResampler::new(44_100, 48_000, 2, 512).expect("resampler");
        let input: Vec<f32> = (0..44_100 * 2)
            .flat_map(|i| {
                let v = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44_100.0).sin() * 0.5;
                [v, v]
            })
            .collect();
        let out = r.process_interleaved(&input).expect("resample");
        let left: Vec<f32> = out.iter().step_by(2).copied().skip(4_800).collect();
        let crossings = left
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count();
        let hz = crossings as f32 * 48_000.0 / left.len() as f32;
        assert!((hz - 440.0).abs() < 2.0, "{hz} Hz");
        let frames = out.len() / 2;
        assert!(
            (frames as i64 - 96_000).abs() < 1_200,
            "{frames} frames for 2 s"
        );
        let peak = left.iter().fold(0.0f32, |a, v| a.max(v.abs()));
        assert!((peak - 0.5).abs() < 0.02, "level kept: {peak}");
    }

    #[test]
    fn test_resampler_arbitrary_lengths_and_rates() {
        let chunk_size = 512;
        let mut resampler =
            PureResampler::new(44100, 48000, 2, chunk_size).expect("resampler creation failed");

        assert_eq!(resampler.source_rate(), 44100);
        assert_eq!(resampler.target_rate(), 48000);

        // Test with fewer samples than chunk_size (e.g. 100 frames = 200 samples)
        let small_input = vec![0.2f32; 200];
        let small_out = resampler
            .process_interleaved(&small_input)
            .expect("small resample failed");
        assert!(!small_out.is_empty());
        assert_eq!(small_out.len() % 2, 0);

        // Test with more samples than chunk_size (e.g. 1200 frames = 2400 samples)
        let large_input = vec![0.3f32; 2400];
        let large_out = resampler
            .process_interleaved(&large_input)
            .expect("large resample failed");
        assert!(!large_out.is_empty());
        assert_eq!(large_out.len() % 2, 0);
    }
}
