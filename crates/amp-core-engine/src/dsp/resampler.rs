//! Pure-Rust Rubato resampler: converts source sample rates (44.1, 48, 96, 192 kHz) to the
//! hardware output rate. Track rates are fixed, so this uses rubato's synchronous FFT resampler
//! (windowed-sinc anti-aliasing with a BlackmanHarris2 window, done in the frequency domain):
//! the same cleanliness as the sinc interpolator at a fraction of its CPU (TASK-179;
//! `tests/resampler_quality.rs`).

#![forbid(unsafe_code)]

use rubato::audioadapter_buffers::direct::{InterleavedSlice, SequentialSliceOfVecs};
use rubato::{Fft, FixedSync, Indexing, Resampler};

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
    resampler: Fft<f32>,
    input_channels: usize,
    chunk_size: usize,
    source_rate: usize,
    target_rate: usize,
    input_fifo: Vec<f32>,
    input_block: Vec<f32>,
}

impl PureResampler {
    /// Creates a new polyphase Sinc resampler instance.
    pub fn new(
        source_rate: usize,
        target_rate: usize,
        channels: usize,
        chunk_size: usize,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let resampler = Fft::<f32>::new(
            source_rate,
            target_rate,
            chunk_size,
            channels,
            FixedSync::Input,
        )?;
        // Rounded up to a whole number of the rate pair's smallest blocks (147 frames for
        // 44.1 -> 48 kHz), so every full block is exactly what rubato expects.
        let chunk_size = resampler.input_frames_next();

        let input_block = vec![0.0f32; chunk_size * channels];
        let input_fifo = Vec::with_capacity(chunk_size * channels * 4);

        Ok(Self {
            resampler,
            input_channels: channels,
            chunk_size,
            source_rate,
            target_rate,
            input_fifo,
            input_block,
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

    /// Processes arbitrary length interleaved audio samples, buffering into the input FIFO
    /// and feeding Rubato only full unpadded blocks to eliminate mid-stream clicks (`TASK-142`, `DSP-04`).
    pub fn process_interleaved(&mut self, samples: &[f32]) -> Result<Vec<f32>, String> {
        self.input_fifo.extend_from_slice(samples);
        let block_len = self.chunk_size * self.input_channels;
        let mut all_out = Vec::new();

        while self.input_fifo.len() >= block_len {
            self.input_block[..block_len].copy_from_slice(&self.input_fifo[..block_len]);
            self.input_fifo.drain(..block_len);

            let input =
                InterleavedSlice::new(&self.input_block, self.input_channels, self.chunk_size)
                    .map_err(|e| format!("Resampling input error: {e}"))?;
            let out = self
                .resampler
                .process(&input, None)
                .map_err(|e| format!("Resampling error: {e}"))?
                .take_data();
            all_out.extend(out);
        }

        Ok(all_out)
    }

    /// Flushes any remaining partial block in the input FIFO at end of stream (`TASK-142`, `DSP-04`).
    pub fn flush_interleaved(&mut self) -> Result<Vec<f32>, String> {
        if self.input_fifo.is_empty() {
            return Ok(Vec::new());
        }
        let frames = self.input_fifo.len() / self.input_channels;
        if frames == 0 {
            self.input_fifo.clear();
            return Ok(Vec::new());
        }
        let input = InterleavedSlice::new(&self.input_fifo, self.input_channels, frames)
            .map_err(|e| format!("Resampling flush error: {e}"))?;
        let indexing = partial(frames, self.chunk_size);
        let mut out = self
            .resampler
            .process(&input, indexing.as_ref())
            .map_err(|e| format!("Resampling error: {e}"))?
            .take_data();
        let keep =
            (frames as f64 * self.target_rate as f64 / self.source_rate as f64).round() as usize;
        out.truncate(keep * self.input_channels);
        self.input_fifo.clear();
        Ok(out)
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
        let mut small_out = resampler
            .process_interleaved(&small_input)
            .expect("small resample failed");
        small_out.extend(resampler.flush_interleaved().expect("flush failed"));
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

    #[test]
    fn test_resampler_streaming_chunks_match_single_shot() {
        let chunk_size = 512;
        let mut r_single = PureResampler::new(44100, 48000, 2, chunk_size).unwrap();
        let mut r_chunked = PureResampler::new(44100, 48000, 2, chunk_size).unwrap();

        // 1 second of 440 Hz stereo sine at 44.1 kHz
        let num_frames = 44100;
        let mut input = Vec::with_capacity(num_frames * 2);
        for i in 0..num_frames {
            let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.7;
            input.push(s);
            input.push(s);
        }

        // Single shot
        let mut out_single = r_single.process_interleaved(&input).unwrap();
        out_single.extend(r_single.flush_interleaved().unwrap());

        // Random-sized chunks: 17, 333, 7, 1024, 61, 513...
        let mut out_chunked = Vec::new();
        let chunk_lens = [34, 666, 14, 2048, 122, 1026, 50, 400];
        let mut offset = 0;
        let mut step_idx = 0;
        while offset < input.len() {
            let len = chunk_lens[step_idx % chunk_lens.len()].min(input.len() - offset);
            let part = &input[offset..offset + len];
            out_chunked.extend(r_chunked.process_interleaved(part).unwrap());
            offset += len;
            step_idx += 1;
        }
        out_chunked.extend(r_chunked.flush_interleaved().unwrap());

        assert_eq!(out_single.len(), out_chunked.len());

        // Verify SNR > 120 dB (max absolute sample diff < 1e-4) and no zero-padding mid-stream
        let mut max_diff = 0.0f32;
        for (i, (&s1, &s2)) in out_single.iter().zip(out_chunked.iter()).enumerate() {
            let diff = (s1 - s2).abs();
            if diff > max_diff {
                max_diff = diff;
            }
            assert!(
                diff < 1e-4,
                "Sample diff at {i} too large: {diff} (single={s1}, chunked={s2})"
            );
        }
        assert!(max_diff < 1e-4, "Max diff {max_diff} exceeded threshold");
    }
}
