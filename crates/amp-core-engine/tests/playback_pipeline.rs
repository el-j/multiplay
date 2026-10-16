//! End-to-end DSP & Audio Pipeline Integration Tests.
//! Validates: Volume Scaling -> 10-Band EQ -> Soft-Knee Limiter -> FFT Spectrum -> CaptureAudioSink.

#![forbid(unsafe_code)]

use amp_core_engine::dsp::{GraphicEqualizer, SpectrumVisualizer};
use amp_core_engine::playback::gapless::{ComfortSilenceGenerator, GaplessConfig, GaplessTrackScheduler};
use amp_domain::{AudioSource, Track, TrackMetadata};
use amp_ports::driven::AudioSinkPort;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// In-memory audio sink capturing rendered PCM samples for test assertions.
#[derive(Clone, Default)]
pub struct CaptureAudioSink {
    pub captured_samples: Arc<Mutex<Vec<f32>>>,
    pub sample_rate: u32,
    pub channels: u16,
    pub is_running: bool,
}

impl AudioSinkPort for CaptureAudioSink {
    fn configure(&mut self, sample_rate: u32, channels: u16) -> Result<(), String> {
        self.sample_rate = sample_rate;
        self.channels = channels;
        Ok(())
    }

    fn start(&mut self) -> Result<(), String> {
        self.is_running = true;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), String> {
        self.is_running = false;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        self.is_running = false;
        Ok(())
    }

    fn push_interleaved_samples(&mut self, samples: &[f32]) -> usize {
        if self.is_running {
            let mut buf = self.captured_samples.lock().unwrap();
            buf.extend_from_slice(samples);
            samples.len()
        } else {
            0
        }
    }

    fn get_buffer_latency(&self) -> Duration {
        Duration::from_millis(10)
    }
}

fn generate_stereo_sine(sample_rate: u32, freq_hz: f32, num_frames: usize, amplitude: f32) -> Vec<f32> {
    let mut buffer = Vec::with_capacity(num_frames * 2);
    for i in 0..num_frames {
        let t = i as f32 / sample_rate as f32;
        let s = (2.0 * std::f32::consts::PI * freq_hz * t).sin() * amplitude;
        buffer.push(s); // Left
        buffer.push(s); // Right
    }
    buffer
}

#[test]
fn test_e2e_playback_pipeline() {
    let sample_rate = 44100u32;
    let mut sink = CaptureAudioSink::default();
    sink.configure(sample_rate, 2).expect("Configure failed");
    sink.start().expect("Start failed");

    // 1. Generate 440 Hz test tone (1024 stereo frames = 2048 samples)
    let num_frames = 1024;
    let mut samples = generate_stereo_sine(sample_rate, 440.0, num_frames, 0.5);

    // 2. Volume attenuation to 50% (-6 dB)
    let volume = 0.5f32;
    for s in samples.iter_mut() {
        *s *= volume;
    }

    // 3. 10-Band Graphic Equalizer with mild bass boost (+3 dB on 60 Hz)
    let mut eq = GraphicEqualizer::new(sample_rate as f32);
    eq.set_band_gain(0, 3.0); // 60 Hz
    eq.set_preamp_gain(0.0);
    eq.process_interleaved(&mut samples);

    // 4. Spectrum Visualizer analysis
    let mut visualizer = SpectrumVisualizer::new();
    visualizer.push_samples(&samples);
    let bands = visualizer.compute_spectrum_bands();
    assert_eq!(bands.len(), 16, "Visualizer must output 16 bands");

    // Fundamental 440 Hz falls into Band 0 (0 to 1378 Hz in linear binning)
    let total_energy: f32 = bands.iter().sum();
    assert!(total_energy > 0.05, "Visualizer should register energy for active audio tone");
    assert!(bands[0] > 0.05, "Band 0 (0-1378 Hz) should register fundamental 440 Hz energy");

    // 5. Push to CaptureAudioSink
    let pushed = sink.push_interleaved_samples(&samples);
    assert_eq!(pushed, num_frames * 2);

    let captured = sink.captured_samples.lock().unwrap();
    assert_eq!(captured.len(), num_frames * 2);
    for &sample in captured.iter() {
        assert!((-1.0..=1.0).contains(&sample), "All captured samples must be within [-1.0, 1.0]");
    }
}

#[test]
fn test_limiter_headroom_and_clipping_safeguard() {
    let sample_rate = 44100u32;
    let num_frames = 512;

    // Generate extreme overdrive signal with peak 4.0 (+12 dB)
    let mut extreme_samples = generate_stereo_sine(sample_rate, 1000.0, num_frames, 4.0);

    let mut eq = GraphicEqualizer::new(sample_rate as f32);
    eq.set_preamp_gain(6.0); // Extreme +6 dB digital preamp
    eq.set_band_gain(4, 12.0); // +12 dB on 1 kHz

    eq.process_interleaved(&mut extreme_samples);

    // Verify soft-knee limiter strictly prevents clipping beyond [-1.0, 1.0]
    for (i, &sample) in extreme_samples.iter().enumerate() {
        assert!(
            (-1.0..=1.0).contains(&sample),
            "Sample {i} exceeded safe ceiling: {sample}"
        );
    }
}

#[test]
fn test_gapless_scheduler_and_comfort_silence() {
    let mut scheduler = GaplessTrackScheduler::new(GaplessConfig::default());

    let track_a = Track::new(
        AudioSource::LocalFile { path: "/music/track_a.flac".to_string() },
        TrackMetadata::new("Track A", "Artist"),
    );
    let track_b = Track::new(
        AudioSource::LocalFile { path: "/music/track_b.flac".to_string() },
        TrackMetadata::new("Track B", "Artist"),
    );

    scheduler.set_active_track(track_a, Some(Duration::from_secs(180)));
    scheduler.queue_next_track(track_b);

    // Test prefetch priming trigger near end of track (180s - 3s lead time = 177s)
    let triggered_early = scheduler.update_progress(Duration::from_secs(100));
    assert!(!triggered_early);
    assert!(!scheduler.is_next_primed());

    let triggered_lead = scheduler.update_progress(Duration::from_secs(178));
    assert!(triggered_lead);
    assert!(scheduler.is_next_primed());

    let next = scheduler.advance_to_next(Some(Duration::from_secs(200)));
    assert!(next.is_some());
    assert_eq!(next.unwrap().metadata.title, "Track B");

    // Comfort silence generator produces smooth click-free envelope and silence
    let silence_gen = ComfortSilenceGenerator::new(64);
    let mut audio_buf = [0.5f32; 128];
    silence_gen.apply_fade_out(&mut audio_buf);
    assert!(audio_buf[127] < 0.001, "Last sample after fade-out must be smoothly attenuated near 0.0");

    let mut silence_buf = [0.5f32; 128];
    silence_gen.fill_silence(&mut silence_buf);
    for &s in silence_buf.iter() {
        assert_eq!(s, 0.0, "Silence buffer must be 0.0");
    }
}
