//! Unified audiophile DSP processing chain.
//! Integrates polyphase Sinc resampling, 10-band EQ, soft-knee peak limiter, and FFT analyzer.

#![forbid(unsafe_code)]

use super::{
    CrossfeedProfile, GraphicEqualizer, HeadphoneCrossfeed, LoudnessConfig, PureResampler,
    PureVolumeNormalizer, SpectrumVisualizer,
};

/// Unified real-time playback DSP pipeline for audio streaming workers.
pub struct PlaybackDspChain {
    resampler: Option<PureResampler>,
    eq: GraphicEqualizer,
    crossfeed: HeadphoneCrossfeed,
    limiter: PureVolumeNormalizer,
    visualizer: SpectrumVisualizer,
    sink_sample_rate: u32,
    chunk_frames: usize,
}

impl PlaybackDspChain {
    /// Creates a new DSP processing chain for the given hardware sink sample rate.
    pub fn new(sink_sample_rate: u32, chunk_frames: usize) -> Self {
        Self {
            resampler: None,
            eq: GraphicEqualizer::new(sink_sample_rate as f32),
            crossfeed: HeadphoneCrossfeed::new(sink_sample_rate as f32),
            limiter: PureVolumeNormalizer::new(LoudnessConfig {
                target_lufs: -14.0,
                max_peak_dbfs: -0.5,
                enabled: true,
            }),
            visualizer: SpectrumVisualizer::new(),
            sink_sample_rate,
            chunk_frames,
        }
    }

    /// Configures the input stream sample rate, initializing or resetting the polyphase Sinc resampler.
    pub fn set_source_sample_rate(&mut self, source_rate: u32) {
        if source_rate != self.sink_sample_rate && source_rate > 0 {
            self.resampler = PureResampler::new(
                source_rate as usize,
                self.sink_sample_rate as usize,
                2,
                self.chunk_frames,
            )
            .ok();
        } else {
            self.resampler = None;
        }
    }

    /// Configures the headphone crossfeed binaural simulation profile.
    pub fn set_crossfeed_profile(&mut self, profile: CrossfeedProfile) {
        self.crossfeed.set_profile(profile);
    }

    /// Configures loudness normalization and peak limiter settings.
    pub fn set_loudness_config(&mut self, config: LoudnessConfig) {
        self.limiter.set_config(config);
    }

    /// Returns whether the resampler is currently active.
    pub fn is_resampling(&self) -> bool {
        self.resampler.is_some()
    }

    /// Processes interleaved stereo audio samples through resampling, volume, EQ, crossfeed, limiter, and visualizer.
    pub fn process_interleaved(
        &mut self,
        input_pcm: &[f32],
        volume: f32,
        preamp: f32,
        eq_gains: &[f32; 10],
        eq_enabled: bool,
    ) -> (Vec<f32>, [f32; 16]) {
        self.process_interleaved_with_lufs(input_pcm, volume, preamp, eq_gains, eq_enabled, None)
    }

    /// Processes audio samples with optional track integrated LUFS loudness normalization.
    pub fn process_interleaved_with_lufs(
        &mut self,
        input_pcm: &[f32],
        volume: f32,
        preamp: f32,
        eq_gains: &[f32; 10],
        eq_enabled: bool,
        track_lufs: Option<f32>,
    ) -> (Vec<f32>, [f32; 16]) {
        // 1. Resample to hardware rate if rates differ
        let mut samples = if let Some(ref mut resampler) = self.resampler {
            resampler
                .process_interleaved(input_pcm)
                .unwrap_or_else(|_| input_pcm.to_vec())
        } else {
            input_pcm.to_vec()
        };

        // 2. Apply master volume
        let vol_clamped = volume.clamp(0.0, 2.0);
        for s in &mut samples {
            *s *= vol_clamped;
        }

        // 3. Apply 10-band graphic equalizer
        self.eq.set_enabled(eq_enabled);
        self.eq.set_preamp_gain(preamp);
        for (idx, &g) in eq_gains.iter().enumerate() {
            self.eq.set_band_gain(idx, g);
        }
        self.eq.process_interleaved(&mut samples);

        // 4. Apply binaural headphone crossfeed
        self.crossfeed.process_interleaved(&mut samples);

        // 5. Soft-knee true-peak limiter and loudness normalization
        self.limiter.process_interleaved(&mut samples, track_lufs);

        // 6. Real-time 16-band FFT visualizer
        self.visualizer.push_samples(&samples);
        let bands = self.visualizer.compute_spectrum_bands();

        (samples, bands)
    }

    /// Returns the maximum true-peak linear amplitude allowed by the limiter.
    pub fn max_peak_linear(&self) -> f32 {
        self.limiter.max_peak_linear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dsp_chain_passthrough_when_rates_match() {
        let mut chain = PlaybackDspChain::new(48000, 512);
        chain.set_source_sample_rate(48000);
        assert!(!chain.is_resampling());

        let input = vec![0.5f32; 1024]; // 512 stereo frames
        let (output, _bands) = chain.process_interleaved(&input, 1.0, 0.0, &[0.0; 10], false);
        assert_eq!(output.len(), input.len());
        assert!((output[0] - 0.5).abs() < 1e-4);
    }

    #[test]
    fn test_dsp_chain_resampling_44100_to_48000() {
        let mut chain = PlaybackDspChain::new(48000, 512);
        chain.set_source_sample_rate(44100);
        assert!(chain.is_resampling());

        let input = vec![0.3f32; 1024]; // 512 frames at 44.1kHz
        let (output1, _bands) = chain.process_interleaved(&input, 1.0, 0.0, &[0.0; 10], false);
        let (output2, bands) = chain.process_interleaved(&input, 1.0, 0.0, &[0.0; 10], false);
        assert!(!output1.is_empty());
        assert!(output2.len() > input.len());
        assert_eq!(bands.len(), 16);
    }

    #[test]
    fn test_dsp_chain_soft_knee_limiter_prevents_clipping() {
        let mut chain = PlaybackDspChain::new(48000, 512);
        chain.set_source_sample_rate(48000);

        // Massive input with +12 dB EQ preamp and full volume
        let input = vec![1.5f32; 512];
        let max_allowed = chain.max_peak_linear(); // ~ -0.5 dBFS = ~0.944

        let (output, _bands) = chain.process_interleaved(&input, 1.5, 12.0, &[12.0; 10], true);

        for &sample in &output {
            assert!(
                sample.abs() <= 1.0,
                "Sample {sample} clipped above 1.0 digital ceiling"
            );
            assert!(
                sample.abs() <= max_allowed + 0.05,
                "Sample {sample} exceeded peak limiter target {max_allowed}"
            );
        }
    }

    #[test]
    fn test_dsp_chain_with_lufs_normalization() {
        let mut chain = PlaybackDspChain::new(48000, 512);
        let input = vec![0.2f32; 1024];
        // Track at -24 LUFS should be boosted towards -14 LUFS
        let (output, _) =
            chain.process_interleaved_with_lufs(&input, 1.0, 0.0, &[0.0; 10], false, Some(-24.0));
        assert!(output[500].abs() > input[500].abs());
    }
}
