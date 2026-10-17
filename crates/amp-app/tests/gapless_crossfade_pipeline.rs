//! Integration test suite for audiophile gapless transitions, automix crossfading,
//! EBU R128 loudness normalization, and binaural headphone crossfeed.

#![forbid(unsafe_code)]

use amp_adapter_decoder::AudioStreamDecoder;
use amp_core_engine::dsp::{
    CrossfadeCurve, Crossfader, CrossfeedProfile, HeadphoneCrossfeed, PlaybackDspChain,
};
use std::fs;
use std::io::Cursor;

fn generate_test_wav_bytes(num_samples: usize, freq: f32, sample_rate: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let data_size = (num_samples * 2 * 2) as u32; // stereo, 16-bit
    let file_size = 36 + data_size;

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&file_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&2u16.to_le_bytes()); // Stereo
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 4).to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let s = (t * freq * 2.0 * std::f32::consts::PI).sin();
        let val = (s * 30000.0) as i16;
        bytes.extend_from_slice(&val.to_le_bytes());
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

#[test]
fn test_seamless_gapless_pipeline_stitching() {
    let wav1_bytes = generate_test_wav_bytes(256, 440.0, 48000);
    let wav2_bytes = generate_test_wav_bytes(256, 880.0, 48000);

    let mut dec1 = AudioStreamDecoder::open_media_source(Cursor::new(wav1_bytes))
        .expect("Open decoder 1 failed");
    let mut dec2 = AudioStreamDecoder::open_media_source(Cursor::new(wav2_bytes))
        .expect("Open decoder 2 failed");

    let mut chunk = vec![0.0f32; 512]; // 256 stereo frames
    let n1 = dec1.read_interleaved_samples(&mut chunk).unwrap();
    assert_eq!(n1, 512);

    // Track 1 reaches EOF; immediately stitch Track 2
    let mut stitch_chunk = vec![0.0f32; 512];
    let n_eof = dec1.read_interleaved_samples(&mut stitch_chunk).unwrap();
    assert_eq!(n_eof, 0);

    // Seamless handoff: Track 2 fills the chunk directly with zero gap
    let n2 = dec2.read_interleaved_samples(&mut stitch_chunk).unwrap();
    assert_eq!(n2, 512);
    assert!(stitch_chunk.iter().any(|&s| s.abs() > 0.05));
}

#[test]
fn test_automix_equal_power_crossfade_energy() {
    let frames = 500;
    let mut xf = Crossfader::new(CrossfadeCurve::EqualPower, frames);
    xf.trigger();

    let outgoing = vec![0.7f32; frames * 2];
    let incoming = vec![0.7f32; frames * 2];
    let mut dest = vec![0.0f32; frames * 2];

    while xf.is_active() {
        let (g_out, g_in) = xf.step_gains();
        // Equal-power invariant: g_out^2 + g_in^2 == 1.0
        let energy_sum = g_out * g_out + g_in * g_in;
        assert!(
            (energy_sum - 1.0).abs() < 1e-4,
            "Equal power energy deviation: {energy_sum}"
        );
    }

    let mut xf_blend = Crossfader::new(CrossfadeCurve::EqualPower, frames);
    xf_blend.trigger();
    xf_blend.process_interleaved_blend(&outgoing, &incoming, &mut dest);
    assert_eq!(dest.len(), frames * 2);
    assert!(dest[0] > 0.0);
}

#[test]
fn test_ebu_r128_loudness_slewing_and_limiting() {
    let mut chain = PlaybackDspChain::new(48000, 512);
    let hot_input = vec![1.2f32; 1024];

    // High level signal with -8 LUFS track metadata (loud master)
    let (attenuated, _) = chain.process_interleaved_with_lufs(
        &hot_input,
        1.0,
        0.0,
        &[0.0; 10],
        false,
        Some(-8.0),
    );

    for s in &attenuated {
        assert!(
            s.abs() <= 1.0,
            "Sample exceeded digital full scale: {s}"
        );
    }

    // Dynamic quiet signal with -24 LUFS track metadata
    let quiet_input = vec![0.1f32; 1024];
    let (boosted, _) = chain.process_interleaved_with_lufs(
        &quiet_input,
        1.0,
        0.0,
        &[0.0; 10],
        false,
        Some(-24.0),
    );

    assert!(
        boosted[500].abs() > quiet_input[500].abs(),
        "Quiet signal was not normalized upwards"
    );
}

#[test]
fn test_binaural_headphone_crossfeed_imaging() {
    let mut xfeed = HeadphoneCrossfeed::new(48000.0);
    xfeed.set_profile(CrossfeedProfile::Standard);

    let mut hard_panned = vec![0.0f32; 256];
    for f in 0..128 {
        hard_panned[f * 2] = (f as f32 * 0.1).sin(); // Left only
        hard_panned[f * 2 + 1] = 0.0;                 // Right silent
    }

    xfeed.process_interleaved(&mut hard_panned);

    // Verify acoustic bleed into right ear after interaural delay (~14 samples)
    let mut right_bleed_detected = false;
    for f in 20..100 {
        if hard_panned[f * 2 + 1].abs() > 0.01 {
            right_bleed_detected = true;
            break;
        }
    }
    assert!(
        right_bleed_detected,
        "Headphone crossfeed failed to generate acoustic binaural bleed"
    );
}

#[test]
fn test_file_metadata_replaygain_extraction() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("multiplay_test_{nonce}"));
    let _ = fs::create_dir_all(&dir);
    let wav_file = dir.join("track.wav");
    let wav_bytes = generate_test_wav_bytes(100, 440.0, 44100);
    fs::write(&wav_file, wav_bytes).unwrap();

    let track = amp_adapter_storage::extract_track_from_file(&wav_file);
    let _ = fs::remove_dir_all(&dir);
    assert!(track.is_some());
    let t = track.unwrap();
    assert_eq!(t.metadata.sample_rate, Some(44100));
    assert_eq!(t.metadata.channels, Some(2));
}
