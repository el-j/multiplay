//! Comprehensive DJ Engine Integration Test Harness.
//! Validates: Dual-deck concurrent decoding, WSOLA time stretching,
//! 3-band kill isolator EQ, auto-BPM sync, crossfade profiles, and soft saturation.

#![forbid(unsafe_code)]

use amp_adapter_decoder::AudioStreamDecoder;
use amp_core_engine::dj::{CrossfaderCurveProfile, DualDeckMixer};
use std::io::Cursor;

fn generate_test_wav_stream(duration_secs: f32, freq_hz: f32) -> Vec<u8> {
    let sample_rate = 44100u32;
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut pcm_samples = Vec::with_capacity(num_samples * 2);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let val = (2.0 * std::f32::consts::PI * freq_hz * t).sin();
        let sample_i16 = (val * 24000.0) as i16;
        pcm_samples.push(sample_i16);
        pcm_samples.push(sample_i16);
    }

    let data_bytes_len = pcm_samples.len() * 2;
    let file_size_minus_8 = (36 + data_bytes_len) as u32;

    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size_minus_8.to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&(16u32).to_le_bytes());
    wav.extend_from_slice(&(1u16).to_le_bytes());
    wav.extend_from_slice(&(2u16).to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 4).to_le_bytes());
    wav.extend_from_slice(&(4u16).to_le_bytes());
    wav.extend_from_slice(&(16u16).to_le_bytes());

    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_bytes_len as u32).to_le_bytes());
    for s in pcm_samples {
        wav.extend_from_slice(&s.to_le_bytes());
    }

    wav
}

#[test]
fn test_dj_engine_dual_stream_and_mixing_harness() {
    let wav_a = generate_test_wav_stream(1.0, 440.0);
    let wav_b = generate_test_wav_stream(1.0, 880.0);

    let mut dec_a = AudioStreamDecoder::open_stream(Cursor::new(wav_a))
        .expect("Decoder A should initialize");
    let mut dec_b = AudioStreamDecoder::open_stream(Cursor::new(wav_b))
        .expect("Decoder B should initialize");

    let chunk_size = 512;
    let mut mixer = DualDeckMixer::new(44100.0, chunk_size);
    mixer.deck_a.set_playing(true);
    mixer.deck_b.set_playing(true);

    let mut buf_a = vec![0.0f32; chunk_size * 2];
    let mut buf_b = vec![0.0f32; chunk_size * 2];
    let mut mix_out = vec![0.0f32; chunk_size * 2];

    // 1. Initial simultaneous read & center crossfade
    let n_a = dec_a.read_interleaved_samples(&mut buf_a).expect("Read deck A");
    let n_b = dec_b.read_interleaved_samples(&mut buf_b).expect("Read deck B");
    assert!(n_a > 0 && n_b > 0);

    mixer.set_crossfader(0.0, CrossfaderCurveProfile::SmoothDip);
    let mixed = mixer.process_and_mix(&buf_a[..n_a], &buf_b[..n_b], &mut mix_out);
    assert_eq!(mixed, n_a.max(n_b));
    assert!(mix_out.iter().take(mixed).any(|&s| s.abs() > 0.05));

    // 2. Full Deck A transition
    mixer.set_crossfader(-1.0, CrossfaderCurveProfile::Linear);
    let _ = mixer.process_and_mix(&buf_a[..n_a], &buf_b[..n_b], &mut mix_out);
    let sum_a: f32 = mix_out.iter().take(64).map(|s| s.abs()).sum();
    assert!(sum_a > 0.1);

    // 3. 3-Band Kill EQ on Deck A
    mixer.deck_a.isolator_eq.kill_low = true;
    mixer.deck_a.isolator_eq.kill_mid = true;
    mixer.deck_a.isolator_eq.kill_high = true;
    let _ = mixer.process_and_mix(&buf_a[..n_a], &buf_b[..n_b], &mut mix_out);
    assert_eq!(mix_out[0], 0.0, "Triple kill should completely silence Deck A");

    // Reset kills
    mixer.deck_a.isolator_eq.kill_low = false;
    mixer.deck_a.isolator_eq.kill_mid = false;
    mixer.deck_a.isolator_eq.kill_high = false;

    // 4. Full Deck B transition
    mixer.set_crossfader(1.0, CrossfaderCurveProfile::SharpCut);
    let _ = mixer.process_and_mix(&buf_a[..n_a], &buf_b[..n_b], &mut mix_out);
    let sum_b: f32 = mix_out.iter().take(64).map(|s| s.abs()).sum();
    assert!(sum_b > 0.1);

    // 5. WSOLA Pitch Stretcher & Tempo Sync
    mixer.deck_a.bpm = Some(128.0);
    mixer.deck_b.bpm = Some(120.0);
    let pitch = mixer.sync_deck_b_to_a().expect("Sync calculation");
    // 128 / 120 = 1.0667 -> ~+6.67%
    assert!((pitch - 6.6667).abs() < 0.1);
    assert!((mixer.deck_b.pitch_percent - 6.6667).abs() < 0.1);

    // 6. Test soft saturation prevents harsh clipping
    let loud_a = vec![0.95f32; 128];
    let loud_b = vec![0.95f32; 128];
    mixer.set_crossfader(0.0, CrossfaderCurveProfile::SharpCut);
    let n = mixer.process_and_mix(&loud_a, &loud_b, &mut mix_out[..128]);
    assert_eq!(n, 128);
    for &s in &mix_out[..128] {
        assert!((-1.0..=1.0).contains(&s), "Output must never exceed +/-1.0");
    }
}
