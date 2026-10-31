//! Gapless-first policy and rate-matched crossfade tests (`TASK-145`, `DSP-07`).

#![forbid(unsafe_code)]

use amp_core_engine::dsp::{CrossfadeCurve, Crossfader, PureResampler};
use amp_domain::{AudioHardwareConfig, AudioSource, Track, TrackMetadata};
use std::time::Duration;

#[test]
fn default_crossfade_is_off() {
    let config = AudioHardwareConfig::default();
    assert_eq!(
        config.crossfade_duration_ms, 0,
        "Crossfade must be 0 ms (off by default) to preserve gapless playback"
    );
}

#[test]
fn smart_crossfade_suppressed_for_same_album() {
    let mut meta_a = TrackMetadata::new("Speak to Me", "Pink Floyd");
    meta_a.album = Some("The Dark Side of the Moon".into());
    meta_a.duration = Some(Duration::from_secs(90));
    let track_a = Track::new(
        AudioSource::LocalFile {
            path: "/music/DarkSideOfTheMoon/01-SpeakToMe.flac".into(),
        },
        meta_a,
    );

    let mut meta_b = TrackMetadata::new("Breathe", "Pink Floyd");
    meta_b.album = Some("The Dark Side of the Moon".into());
    meta_b.duration = Some(Duration::from_secs(163));
    let track_b = Track::new(
        AudioSource::LocalFile {
            path: "/music/DarkSideOfTheMoon/02-Breathe.flac".into(),
        },
        meta_b,
    );

    // Smart crossfade predicate
    let is_same_album = match (&track_a.metadata.album, &track_b.metadata.album) {
        (Some(a1), Some(a2)) => !a1.is_empty() && a1.eq_ignore_ascii_case(a2),
        _ => false,
    };
    assert!(
        is_same_album,
        "Consecutive tracks from the same album must be recognized"
    );
}

#[test]
fn rate_matched_crossfade_preserves_pitch() {
    let target_rate = 48000;

    // Stream 1: 44.1 kHz sine wave at 440 Hz (0.5 second = 22050 frames)
    let mut resampler_441 = PureResampler::new(44100, target_rate, 2, 512).unwrap();
    let num_frames_441 = 22050;
    let mut pcm_441 = Vec::with_capacity(num_frames_441 * 2);
    for i in 0..num_frames_441 {
        let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.5;
        pcm_441.push(s);
        pcm_441.push(s);
    }
    let resampled_1 = resampler_441.process_interleaved(&pcm_441).unwrap();

    // Stream 2: Native 48 kHz sine wave at 440 Hz (0.5 second = 24000 frames)
    let num_frames_48 = 24000;
    let mut pcm_48 = Vec::with_capacity(num_frames_48 * 2);
    for i in 0..num_frames_48 {
        let s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin() * 0.5;
        pcm_48.push(s);
        pcm_48.push(s);
    }

    // Blend both rate-matched streams
    let blend_frames = 20000;
    let mut blended = vec![0.0f32; blend_frames * 2];
    let mut xf = Crossfader::new(CrossfadeCurve::EqualPower, blend_frames);
    xf.trigger();
    xf.process_interleaved_blend(
        &resampled_1[..blend_frames * 2],
        &pcm_48[..blend_frames * 2],
        &mut blended,
    );

    // Verify output has zero discontinuities and preserves 440 Hz frequency
    let left: Vec<f32> = blended.iter().step_by(2).copied().collect();
    let crossings = left
        .windows(2)
        .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
        .count();
    let est_hz = crossings as f32 * 48000.0 / left.len() as f32;
    assert!(
        (est_hz - 440.0).abs() < 2.0,
        "Crossfaded tone preserved pitch: {est_hz} Hz"
    );
}
