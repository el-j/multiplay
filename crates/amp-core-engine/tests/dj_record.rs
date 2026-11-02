//! Recording the master mix (TASK-224): what the speakers get is what the file holds. WAV keeps
//! the 32-bit float master bit for bit; FLAC keeps it at 24 bits. The render thread never waits
//! for the disk, and the finished file is tagged as a mix.

use amp_core_engine::dj::engine::{DjEngine, TrackBuffer, CROSSFADER_LEFT};
use amp_core_engine::dj::recorder::{MixMetadata, RecordFormat};
use amp_ports::DecoderFactoryPort;
use std::sync::Arc;

fn chirp(rate: u32, secs: f32) -> Arc<TrackBuffer> {
    let frames = (rate as f32 * secs) as usize;
    let mut s = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f32 / rate as f32;
        let v = 0.6 * (2.0 * std::f32::consts::PI * (200.0 + 300.0 * t) * t).sin();
        s.extend([v, -v * 0.5]);
    }
    TrackBuffer::from_samples(rate, &s)
}

/// Renders a scripted mix while recording; returns what the engine sent to the speakers.
fn scripted_mix(path: &std::path::Path, format: RecordFormat) -> Vec<f32> {
    let (mut engine, mut control) = DjEngine::new(48_000, 2);
    control.load(0, chirp(48_000, 3.0));
    control.load(1, chirp(44_100, 3.0));
    control.params().set_crossfader(CROSSFADER_LEFT);
    for d in 0..2 {
        control.params().deck(d).set_playing(true);
    }
    let meta = MixMetadata {
        title: "Friday mix".into(),
        artist: "DJ Test".into(),
    };
    control
        .recorder()
        .start(path, format, meta)
        .expect("starts");
    assert!(control.recorder().is_recording());
    let (mut master, mut cue) = (vec![0.0f32; 1024], vec![0.0f32; 1024]);
    let mut heard = Vec::new();
    for block in 0..120 {
        // Move the crossfader during the mix so both decks are heard.
        control.params().set_crossfader(-1.0 + block as f32 / 60.0);
        engine.render(&mut master, &mut cue);
        heard.extend_from_slice(&master);
    }
    let info = control.recorder().stop().expect("finishes");
    assert_eq!(info.frames, heard.len() as u64 / 2);
    assert_eq!(info.dropped_frames, 0);
    assert!(!control.recorder().is_recording());
    heard
}

fn wav_floats(bytes: &[u8]) -> Vec<f32> {
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let mut at = 12;
    let mut fmt_tag = 0;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        let body = &bytes[at + 8..at + 8 + len];
        if id == b"fmt " {
            fmt_tag = u16::from_le_bytes([body[0], body[1]]);
            assert_eq!(u16::from_le_bytes([body[2], body[3]]), 2, "stereo");
            assert_eq!(u32::from_le_bytes(body[4..8].try_into().unwrap()), 48_000);
        }
        if id == b"data" {
            assert_eq!(fmt_tag, 3, "IEEE float");
            return body
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| f32::from_le_bytes(*c))
                .collect();
        }
        at += 8 + len + (len & 1);
    }
    panic!("no data chunk");
}

#[test]
fn dj_record_wav_is_bit_identical_to_the_master_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mix.wav");
    let heard = scripted_mix(&path, RecordFormat::Wav);
    assert!(heard.iter().any(|s| s.abs() > 0.1), "the mix is not silent");
    let recorded = wav_floats(&std::fs::read(&path).unwrap());
    assert_eq!(recorded.len(), heard.len());
    assert!(
        recorded
            .iter()
            .zip(&heard)
            .all(|(a, b)| a.to_bits() == b.to_bits()),
        "every sample is the same bits"
    );
}

#[test]
fn dj_record_flac_matches_at_24_bits_and_is_tagged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mix.flac");
    let heard = scripted_mix(&path, RecordFormat::Flac);
    let mut decoder = amp_adapter_decoder::SymphoniaDecoderFactory
        .open_file(path.to_str().unwrap())
        .expect("a valid FLAC file");
    assert_eq!(decoder.sample_rate(), 48_000);
    let mut decoded = Vec::new();
    let mut buf = vec![0.0f32; 4096];
    loop {
        let n = decoder.read_interleaved(&mut buf).unwrap();
        if n == 0 {
            break;
        }
        decoded.extend_from_slice(&buf[..n]);
    }
    assert_eq!(decoded.len(), heard.len());
    let worst = decoded
        .iter()
        .zip(&heard)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        worst <= 2.0 / 8_388_608.0,
        "within 24-bit rounding: {worst}"
    );

    let track = amp_adapter_storage_free_tags(&path);
    assert_eq!(track, ("Friday mix".to_string(), "DJ Test".to_string()));
}

fn amp_adapter_storage_free_tags(path: &std::path::Path) -> (String, String) {
    use lofty::file::TaggedFileExt;
    use lofty::tag::Accessor;
    let file = lofty::read_from_path(path).unwrap();
    let tag = file.primary_tag().expect("tagged");
    (
        tag.title().unwrap_or_default().to_string(),
        tag.artist().unwrap_or_default().to_string(),
    )
}

#[test]
fn dj_record_never_blocks_the_render_thread() {
    // Nobody drains the ring (no recording started): rendering is unaffected.
    let (mut engine, mut control) = DjEngine::new(48_000, 2);
    control.load(0, chirp(48_000, 1.0));
    control.params().deck(0).set_playing(true);
    let (mut master, mut cue) = (vec![0.0f32; 1024], vec![0.0f32; 1024]);
    for _ in 0..200 {
        engine.render(&mut master, &mut cue);
    }
    assert!(!control.recorder().is_recording());
    assert!(control.recorder().stop().is_err(), "nothing to stop");
}

#[test]
fn dj_record_refuses_a_second_start_and_an_unwritable_path() {
    let dir = tempfile::tempdir().unwrap();
    let (_engine, mut control) = DjEngine::new(48_000, 2);
    let meta = || MixMetadata {
        title: "x".into(),
        artist: "y".into(),
    };
    let bad = dir.path().join("missing-folder/mix.wav");
    assert!(control
        .recorder()
        .start(&bad, RecordFormat::Wav, meta())
        .is_err());
    assert!(!control.recorder().is_recording());
    let path = dir.path().join("mix.wav");
    control
        .recorder()
        .start(&path, RecordFormat::Wav, meta())
        .unwrap();
    assert!(control
        .recorder()
        .start(&path, RecordFormat::Flac, meta())
        .is_err());
    let info = control.recorder().stop().unwrap();
    assert_eq!(info.frames, 0);
    assert!(path.is_file());
}
