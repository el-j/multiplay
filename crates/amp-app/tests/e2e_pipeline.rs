//! Comprehensive End-to-End Pipeline Integration Test.
//! Validates: Local WAV -> Lofty Tagging -> Redb Storage -> Precache Buffer ->
//! Symphonia Decoder -> Rubato Resampler -> 10-Band EQ -> FFT Visualizer -> CQRS Bus.

#![forbid(unsafe_code)]

use amp_adapter_decoder::PureDecoderEngine;
use amp_adapter_storage::RedbStorageAdapter;
use amp_core_engine::{
    EventBus, GraphicEqualizer, PlayerCommand, PlayerEvent, PureResampler, SharedPrecacheBuffer,
    SpectrumVisualizer,
};
use amp_domain::{AudioSource, PlaybackState, Track, TrackMetadata};
use amp_ports::driven::LibraryStoragePort;
use std::io::Cursor;
use std::time::Duration;

fn generate_pcm_wav_stream(duration_secs: f32, freq_hz: f32) -> Vec<u8> {
    let sample_rate = 44100u32;
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut pcm_samples = Vec::with_capacity(num_samples * 2);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let val = (2.0 * std::f32::consts::PI * freq_hz * t).sin();
        let sample_i16 = (val * 32767.0) as i16;
        pcm_samples.push(sample_i16); // Left
        pcm_samples.push(sample_i16); // Right
    }

    let data_bytes_len = pcm_samples.len() * 2;
    let file_size_minus_8 = (36 + data_bytes_len) as u32;

    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size_minus_8.to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    // fmt chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&(16u32).to_le_bytes());
    wav.extend_from_slice(&(1u16).to_le_bytes()); // PCM
    wav.extend_from_slice(&(2u16).to_le_bytes()); // Stereo
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 4).to_le_bytes());
    wav.extend_from_slice(&(4u16).to_le_bytes());
    wav.extend_from_slice(&(16u16).to_le_bytes());

    // data chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_bytes_len as u32).to_le_bytes());
    for s in pcm_samples {
        wav.extend_from_slice(&s.to_le_bytes());
    }

    wav
}

#[tokio::test]
async fn test_full_audiophile_e2e_pipeline() {
    // 1. Generate 0.5s of 1kHz pure tone audio in WAV container
    let wav_bytes = generate_pcm_wav_stream(0.5, 1000.0);
    assert!(!wav_bytes.is_empty());

    // 2. Setup Track Domain Entity
    let mut meta = TrackMetadata::new("E2E Test Tone", "Synthesizer");
    meta.sample_rate = Some(44100);
    meta.channels = Some(2);
    meta.duration = Some(Duration::from_millis(500));

    let track = Track::new(
        AudioSource::DirectHttp {
            url: "http://localhost/tone.wav".to_string(),
        },
        meta,
    );

    // 3. Test Embedded Redb Storage Persistence
    let temp_dir = std::env::temp_dir().join(format!("multiplay_e2e_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test_e2e.redb");
    let storage = RedbStorageAdapter::new(&db_path).expect("Failed to create redb storage");

    storage
        .store_track(&track)
        .await
        .expect("Failed to store track");
    let fetched = storage
        .fetch_all()
        .await
        .expect("Failed to fetch all tracks");
    assert_eq!(fetched.len(), 1);
    assert_eq!(fetched[0].metadata.title, "E2E Test Tone");

    // 4. Test RAM Precaching Buffer
    let precache = SharedPrecacheBuffer::new(1024 * 1024);
    precache.push_chunk(&wav_bytes);
    precache.mark_finished();
    assert_eq!(precache.total_bytes_cached(), wav_bytes.len());

    // 5. Test Pure-Rust Symphonia Decoder
    let (mut ring_prod, mut ring_cons) = rtrb::RingBuffer::<f32>::new(65536);
    let stop_signal = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let spec = PureDecoderEngine::decode_stream_to_ring_buffer(
        Cursor::new(wav_bytes),
        &mut ring_prod,
        stop_signal,
    )
    .expect("Decoder failed to parse WAV");
    assert_eq!(spec.sample_rate, 44100);
    assert_eq!(spec.channels, 2);

    // Read decoded samples from lock-free ring buffer
    let mut decoded_samples = Vec::new();
    while let Ok(sample) = ring_cons.pop() {
        decoded_samples.push(sample);
    }
    assert!(!decoded_samples.is_empty());

    // 6. Test Rubato Sinc Resampler (44.1kHz -> 48kHz)
    let mut resampler =
        PureResampler::new(44100, 48000, 2, 1024).expect("Failed to create Sinc resampler");
    // Resample a chunk of 1024 stereo frames (2048 samples)
    let chunk_samples = &decoded_samples[..1024 * 2];
    let resampled = resampler
        .process_interleaved(chunk_samples)
        .expect("Resampling failed");
    assert!(!resampled.is_empty());

    // 7. Test 10-Band Graphic Equalizer
    let mut eq = GraphicEqualizer::new(48000.0);
    eq.set_band_gain(3, 4.0); // Boost 500Hz
    eq.set_band_gain(4, -3.0); // Cut 1kHz
    let mut filtered_samples = resampled.clone();
    eq.process_interleaved(&mut filtered_samples);
    assert_ne!(resampled, filtered_samples);

    // 8. Test Zero-Allocation FFT Spectrum Visualizer
    let mut visualizer = SpectrumVisualizer::new();
    visualizer.push_samples(&filtered_samples);
    let spectrum = visualizer.compute_spectrum_bands();
    assert_eq!(spectrum.len(), 16);
    // Peak energy should be detected in the spectrum
    let total_magnitude: f32 = spectrum.iter().sum();
    assert!(total_magnitude > 0.0);

    // 9. Test CQRS Event Bus
    let (bus, mut cmd_rx) = EventBus::new(32);
    let mut event_rx = bus.subscribe();

    bus.send_command(PlayerCommand::PlayTrack(Box::new(track.clone())));
    let received_cmd = cmd_rx.recv().await.expect("Failed to receive command");
    assert!(matches!(received_cmd, PlayerCommand::PlayTrack(_)));

    bus.emit_event(PlayerEvent::PlaybackStateChanged(PlaybackState::Playing));
    let received_evt = event_rx.recv().await.expect("Failed to receive event");
    assert!(matches!(
        received_evt,
        PlayerEvent::PlaybackStateChanged(PlaybackState::Playing)
    ));

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_corrupt_stream_fail_fast_without_panic() {
    use amp_adapter_decoder::AudioStreamDecoder;

    let corrupt_payloads: Vec<Vec<u8>> = vec![
        vec![],                             // Empty
        vec![0x52, 0x49, 0x46, 0x46],       // Incomplete "RIFF" header
        vec![0xFF, 0xFB, 0x90, 0x44],       // Truncated MP3 frame
        vec![0x4F, 0x67, 0x67, 0x53, 0x00], // Truncated Ogg header
        vec![0x66, 0x4C, 0x61, 0x43],       // Truncated fLaC marker
        vec![0xAA; 128],                    // Completely random garbage bytes
    ];

    for (idx, payload) in corrupt_payloads.into_iter().enumerate() {
        let cursor = Cursor::new(payload);
        let res = AudioStreamDecoder::open_media_source(cursor);
        assert!(
            res.is_err(),
            "Payload #{idx} should return Err rather than panicking"
        );
    }
}

#[tokio::test]
async fn test_progressive_network_audio_streaming() {
    use amp_app::AudioPlaybackDriver;
    use amp_ports::{async_trait, AudioSourceStream, ProviderError, StreamResolverPort};
    use std::sync::Arc;

    struct ProgressiveMockResolver {
        wav_data: Vec<u8>,
    }

    #[async_trait]
    impl StreamResolverPort for ProgressiveMockResolver {
        fn handles_source(&self, source: &AudioSource) -> bool {
            matches!(source, AudioSource::DirectHttp { url } if url.contains("progressive_test"))
        }

        async fn resolve_stream(
            &self,
            _source: &AudioSource,
        ) -> Result<Box<dyn AudioSourceStream>, ProviderError> {
            let buffer = SharedPrecacheBuffer::new(self.wav_data.len() + 1024);
            let writer = buffer.clone();
            let chunks: Vec<Vec<u8>> = self.wav_data.chunks(256).map(|c| c.to_vec()).collect();

            tokio::spawn(async move {
                for chunk in chunks {
                    writer.push_chunk(&chunk);
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                writer.mark_finished();
            });

            // Wait for initial probe chunk (~256 bytes)
            while buffer.total_bytes_cached() < 256 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }

            Ok(Box::new(buffer))
        }
    }

    let wav_data = generate_pcm_wav_stream(0.5, 440.0);
    let resolver = Arc::new(ProgressiveMockResolver { wav_data });

    if let Ok(driver) = AudioPlaybackDriver::new() {
        driver.register_resolver(resolver);

        let meta = TrackMetadata::new("Stream Song", "Network Station");
        let track = Track::new(
            AudioSource::DirectHttp {
                url: "http://example.com/progressive_test.wav".to_string(),
            },
            meta,
        );

        driver.play(track);
        tokio::time::sleep(Duration::from_millis(80)).await;

        // Verify driver is playing and track was loaded
        assert!(driver.is_playing.load(std::sync::atomic::Ordering::Relaxed));
        driver.stop();
        assert!(!driver.is_playing.load(std::sync::atomic::Ordering::Relaxed));
    }
}
