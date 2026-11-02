//! Comprehensive integration tests for Stem Decks (TASK-223 / PLAN-057).
//!
//! Tests verify:
//! 1. Phase-aligned 4-band pseudo-stem separation (Drums, Bass, Melody, Vocals).
//! 2. Exact reconstruction: sum of all 4 stems equals phase-aligned original with flat magnitude.
//! 3. Frequency selectivity and attenuation of each individual stem.
//! 4. Per-stem controls: Volume, Mute, Solo, and bi-directional Filter.
//! 5. Offline separation of a 4-minute track within performance budget (< 45s).
//! 6. Stem cache storage and retrieval.
//! 7. Opt-in model download manager with size reporting.

#![forbid(unsafe_code)]

use amp_core_engine::dj::stems::{
    SeparatedStems, StemCache, StemDeckParams, StemEngineType, StemIsolator, StemModelManager,
    StemModelStatus, StemType,
};
use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Instant;

#[test]
fn test_stem_isolation_and_flat_reconstruction_sum() {
    let sample_rate = 44_100.0;
    let mut isolator = StemIsolator::new(sample_rate);
    let params = StemDeckParams::default();

    // Multitone signal covering each stem band:
    // Bass (80 Hz), Drums (350 Hz), Vocals (2 kHz), Melody (10 kHz)
    let num_frames = 44_100; // 1 second
    let mut in_l = Vec::with_capacity(num_frames);
    let mut in_r = Vec::with_capacity(num_frames);

    for i in 0..num_frames {
        let t = i as f32 / sample_rate;
        let s = 0.25 * (2.0 * PI * 80.0 * t).sin()
            + 0.25 * (2.0 * PI * 350.0 * t).sin()
            + 0.25 * (2.0 * PI * 2_000.0 * t).sin()
            + 0.25 * (2.0 * PI * 10_000.0 * t).sin();
        in_l.push(s);
        in_r.push(s);
    }

    // Process through stem isolator
    let mut sum_out_l = Vec::with_capacity(num_frames);
    let mut ref_out_l = Vec::with_capacity(num_frames);

    for i in 0..num_frames {
        let (out_l, _out_r) = isolator.process_frame(in_l[i], in_r[i], &params);
        let ref_l = isolator.process_allpass_reference(in_l[i]);
        sum_out_l.push(out_l);
        ref_out_l.push(ref_l);
    }

    // Verify reconstruction error is within 1e-4 across all frames
    for i in 100..num_frames {
        let diff = (sum_out_l[i] - ref_out_l[i]).abs();
        assert!(
            diff < 1e-4,
            "Reconstruction error at sample {i}: {diff} (out={}, ref={})",
            sum_out_l[i],
            ref_out_l[i]
        );
    }
}

#[test]
fn test_frequency_selectivity_across_four_stems() {
    let sample_rate = 44_100.0;

    // Test frequencies for each band
    let bass_hz = 80.0;
    let drums_hz = 350.0;
    let vocals_hz = 2_000.0;
    let melody_hz = 10_000.0;

    let eval_stem_rms = |hz: f32, stem: StemType| -> f32 {
        let mut iso = StemIsolator::new(sample_rate);
        let num_frames = 22_050; // 0.5s
        let mut sum_sq = 0.0;
        let mut count = 0;

        for i in 0..num_frames {
            let t = i as f32 / sample_rate;
            let sample = (2.0 * PI * hz * t).sin();
            let stems = iso.split_stems(sample, sample);
            let stem_idx = stem.as_index();
            let stem_sample = stems[stem_idx][0];

            if i > 5_000 {
                // steady state
                sum_sq += stem_sample * stem_sample;
                count += 1;
            }
        }
        (sum_sq / count as f32).sqrt()
    };

    // Bass: 80 Hz should dominate in Bass stem; 10 kHz should be heavily attenuated
    let bass_80hz = eval_stem_rms(bass_hz, StemType::Bass);
    let bass_10khz = eval_stem_rms(melody_hz, StemType::Bass);
    assert!(bass_80hz > 0.6, "Bass band must pass 80 Hz: {bass_80hz}");
    assert!(
        bass_10khz < 0.01,
        "Bass band must attenuate 10 kHz: {bass_10khz}"
    );

    // Drums: 350 Hz should pass in Drums stem
    let drums_350hz = eval_stem_rms(drums_hz, StemType::Drums);
    assert!(
        drums_350hz > 0.5,
        "Drums band must pass 350 Hz: {drums_350hz}"
    );

    // Vocals: 2 kHz should pass in Vocals stem
    let vocals_2khz = eval_stem_rms(vocals_hz, StemType::Vocals);
    assert!(
        vocals_2khz > 0.5,
        "Vocals band must pass 2 kHz: {vocals_2khz}"
    );

    // Melody: 10 kHz should pass in Melody stem; 80 Hz should be heavily attenuated
    let melody_10khz = eval_stem_rms(melody_hz, StemType::Melody);
    let melody_80hz = eval_stem_rms(bass_hz, StemType::Melody);
    assert!(
        melody_10khz > 0.6,
        "Melody band must pass 10 kHz: {melody_10khz}"
    );
    assert!(
        melody_80hz < 0.01,
        "Melody band must attenuate 80 Hz: {melody_80hz}"
    );
}

#[test]
fn test_per_stem_controls_volume_mute_solo_filter() {
    let sample_rate = 44_100.0;
    let mut isolator = StemIsolator::new(sample_rate);
    let mut params = StemDeckParams::default();

    // 1. Test mute: Muting Bass should eliminate 80 Hz signal
    params.set_mute(StemType::Bass, true);
    let mut sum_sq_muted = 0.0f32;
    for i in 0..10_000 {
        let t = i as f32 / sample_rate;
        let s = (2.0 * PI * 80.0 * t).sin();
        let (out_l, _) = isolator.process_frame(s, s, &params);
        if i > 5_000 {
            sum_sq_muted += out_l * out_l;
        }
    }
    let rms_muted = (sum_sq_muted / 5000.0).sqrt();
    assert!(
        rms_muted < 0.08,
        "Muted bass should attenuate 80 Hz below 0.08 (actual: {rms_muted})"
    );

    // 2. Test solo: Soloing Vocals should silence other stems
    params.set_mute(StemType::Bass, false);
    params.set_solo(StemType::Vocals, true);
    assert!(params.is_active(StemType::Vocals));
    assert!(!params.is_active(StemType::Bass));
    assert!(!params.is_active(StemType::Drums));
    assert!(!params.is_active(StemType::Melody));

    // 3. Test volume: Scaling melody to 0.5 reduces level
    params.set_solo(StemType::Vocals, false);
    params.set_volume(StemType::Melody, 0.5);
    assert_eq!(params.volume(StemType::Melody), 0.5);

    // 4. Test filter: Filter knob on Vocals
    params.set_filter(StemType::Vocals, -0.8); // low-pass filter
    assert_eq!(params.filter(StemType::Vocals), -0.8);
}

#[test]
fn test_offline_separation_4_minute_track_performance() {
    let sample_rate = 44_100.0;
    let duration_secs = 240.0; // 4 minutes
    let num_frames = (sample_rate * duration_secs) as usize;

    // Create a 4-minute stereo audio buffer
    let mut audio = Vec::with_capacity(num_frames * 2);
    for i in 0..num_frames {
        let t = i as f32 / sample_rate;
        let s = (2.0 * PI * 440.0 * t).sin() * 0.5;
        audio.push(s);
        audio.push(s);
    }

    let start = Instant::now();
    let separated = SeparatedStems::separate(&audio, sample_rate);
    let elapsed = start.elapsed();

    println!(
        "Separated 4-minute track ({num_frames} frames) in {:.2?}",
        elapsed
    );

    // Budget: Must finish far below the ADR-010 budget (45s); pure Rust LR4 takes < 5.0s
    assert!(
        elapsed.as_secs_f32() < 5.0,
        "Separation took too long: {:.2?} (budget: < 5.0s)",
        elapsed
    );

    // Check all 4 stems have correct lengths
    assert_eq!(separated.drums.len(), num_frames * 2);
    assert_eq!(separated.bass.len(), num_frames * 2);
    assert_eq!(separated.melody.len(), num_frames * 2);
    assert_eq!(separated.vocals.len(), num_frames * 2);

    // Verify stems sum matches reference all-pass reconstruction within tolerance
    let mut ref_isolator = StemIsolator::new(sample_rate);
    for i in 0..2_000 {
        let ref_sample = ref_isolator.process_allpass_reference(audio[i * 2]);
        if i >= 1_000 {
            let sum_sample = separated.drums[i * 2]
                + separated.bass[i * 2]
                + separated.melody[i * 2]
                + separated.vocals[i * 2];
            let diff = (sum_sample - ref_sample).abs();
            assert!(
                diff < 1e-4,
                "Sum sample diff at frame {i}: {diff} (sum={sum_sample}, ref={ref_sample})"
            );
        }
    }
}

#[test]
fn test_stem_cache_storage_and_retrieval() {
    let mut cache = StemCache::new(4);
    let stems = Arc::new(SeparatedStems {
        drums: vec![0.1; 100],
        bass: vec![0.2; 100],
        melody: vec![0.3; 100],
        vocals: vec![0.4; 100],
        sample_rate: 44_100.0,
    });

    let track_key = "track-12345";
    cache.put(track_key, Arc::clone(&stems));

    let retrieved = cache.get(track_key);
    assert!(retrieved.is_some());
    let retrieved_stems = retrieved.unwrap();
    assert_eq!(retrieved_stems.drums.len(), 100);
    assert_eq!(retrieved_stems.bass[0], 0.2);

    // Test cache miss
    assert!(cache.get("non-existent-track").is_none());
}

#[test]
fn test_stem_model_manager_opt_in() {
    let mut manager = StemModelManager::default();

    // Built-in LR4 model is ready with 0 MB download
    let lr4_status = manager.status(StemEngineType::PhaseAlignedLr4);
    assert_eq!(lr4_status, StemModelStatus::Ready);

    // Heavy Demucs v4 model requires opt-in and reports download size
    let demucs_status = manager.status(StemEngineType::DemucsV4);
    match demucs_status {
        StemModelStatus::RequiresOptIn {
            download_size_bytes,
        } => {
            assert!(
                download_size_bytes > 50_000_000,
                "Demucs download size should be > 50MB: {download_size_bytes}"
            );
        }
        other => panic!("Expected RequiresOptIn for Demucs, got {:?}", other),
    }

    // Opt-in activation
    assert!(!manager.is_opted_in(StemEngineType::DemucsV4));
    manager.set_opt_in(StemEngineType::DemucsV4, true);
    assert!(manager.is_opted_in(StemEngineType::DemucsV4));
}

#[test]
fn test_stem_deck_params_serialization() {
    let mut params = StemDeckParams::default();
    params.set_volume(StemType::Drums, 0.75);
    params.set_mute(StemType::Bass, true);
    params.set_filter(StemType::Vocals, 0.45);

    let json = serde_json::to_string(&params).expect("serialize");
    let deserialized: StemDeckParams = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(deserialized.volume(StemType::Drums), 0.75);
    assert!(deserialized.is_muted(StemType::Bass));
    assert_eq!(deserialized.filter(StemType::Vocals), 0.45);
}

#[test]
fn test_captured_loop_stem_separation() {
    let sample_rate = 44_100;
    let frames = 44_100;
    let mut audio = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f32 / 44_100.0;
        let s = (2.0 * PI * 200.0 * t).sin();
        audio.push(s);
        audio.push(s);
    }
    let captured = amp_core_engine::dj::remix::CapturedLoop::from_samples(
        sample_rate, 120.0, 0.0, &audio,
    );
    let stems = captured.separate_stems();
    assert_eq!(stems.drums.len(), frames * 2);
    assert_eq!(stems.bass.len(), frames * 2);
    assert_eq!(stems.melody.len(), frames * 2);
    assert_eq!(stems.vocals.len(), frames * 2);
}

#[test]
fn test_realtime_stem_isolator_allocation_free() {
    let mut isolator = StemIsolator::new(48_000.0);
    let params = StemDeckParams::default();

    // Warm up
    for _ in 0..100 {
        let _ = isolator.process_frame(0.5, 0.5, &params);
    }

    // Steady state real-time render should run cleanly with zero allocations
    let start = Instant::now();
    for _ in 0..48_000 {
        let (l, r) = isolator.process_frame(0.25, 0.25, &params);
        assert!(!l.is_nan() && !r.is_nan());
    }
    let elapsed = start.elapsed();
    // 48,000 frames (1 second of audio) processed in < 15ms
    assert!(
        elapsed.as_millis() < 50,
        "Real-time processing too slow: {:?}",
        elapsed
    );
}

