//! Every golden fixture plays to the end through the real player engine (TASK-226): decoding,
//! resampling 44.1/96/192 kHz to the 48 kHz output, 5.1 downmix and the output stage, captured
//! instead of sent to a sound card.

#![forbid(unsafe_code)]

use amp_adapter_gui::create_capture_engine;
use amp_domain::{AudioSource, Track, TrackMetadata};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

const OUTPUT_RATE: f64 = 48_000.0;

const FIXTURES: &[(&str, f64)] = &[
    ("flac_16_44k.flac", 1.0),
    ("flac_24_96k.flac", 0.5),
    ("flac_24_192k.flac", 0.25),
    ("flac_no_seektable.flac", 1.0),
    ("flac_5_1_48k.flac", 0.5),
    ("wav_16.wav", 1.0),
    ("wav_24.wav", 0.5),
    ("wav_f32.wav", 0.5),
    ("mp3_cbr_128.mp3", 1.0),
    ("mp3_vbr.mp3", 1.0),
    ("aac.m4a", 1.0),
    ("vorbis.ogg", 1.0),
    ("alac.m4a", 1.0),
    ("aiff_16.aiff", 1.0),
    ("caf_16.caf", 1.0),
    ("flac.mka", 1.0),
    ("vorbis.webm", 1.0),
    ("mp2_192.mp2", 1.0),
    ("adpcm_ms.wav", 1.0),
    ("opus_stereo.opus", 1.0),
    ("opus_mono.opus", 1.0),
    ("opus_5_1.opus", 0.5),
    ("opus.webm", 1.0),
];

fn play_to_end(file: &str, seconds: f64) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../amp-adapter-decoder/tests/fixtures")
        .join(file);
    let (engine, output) = create_capture_engine().expect("capture engine");
    engine.play(Track::new(
        AudioSource::LocalFile {
            path: path.to_string_lossy().into_owned(),
        },
        TrackMetadata::new("Fixture", "multiplay"),
    ));
    let deadline = Instant::now() + Duration::from_secs_f64(seconds + 8.0);
    while !engine.track_finished.load(Ordering::SeqCst) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        engine.track_finished.load(Ordering::SeqCst),
        "{file}: never finished"
    );
    // "Finished" means decoding is done; the output buffer still drains in real time.
    let mut len = output.len();
    loop {
        std::thread::sleep(Duration::from_millis(150));
        let now = output.len();
        if now == len {
            break;
        }
        len = now;
    }

    let samples = output.samples();
    assert!(
        samples.iter().all(|s| s.is_finite()),
        "{file}: NaN or infinity"
    );
    let first = samples.iter().position(|s| s.abs() > 0.05).unwrap_or(0) / 2;
    let last = samples.iter().rposition(|s| s.abs() > 0.05).unwrap_or(0) / 2;
    let span = (last - first) as f64;
    let want = seconds * OUTPUT_RATE;
    assert!(
        (span - want).abs() < want * 0.03 + 0.03 * OUTPUT_RATE,
        "{file}: {span} frames of sound at 48 kHz, want ~{want}"
    );
    for ch in 0..2 {
        let peak = samples
            .iter()
            .skip(ch)
            .step_by(2)
            .fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(
            peak > 0.2 && peak <= 1.0,
            "{file}: channel {ch} peak {peak}"
        );
    }
}

#[test]
fn every_fixture_plays_to_the_end_through_the_engine() {
    std::thread::scope(|s| {
        for (file, seconds) in FIXTURES {
            s.spawn(move || play_to_end(file, *seconds));
        }
    });
}
