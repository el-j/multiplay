//! The faceplate shows what is really playing (TASK-170): the source format from the decoder and
//! tags, and whether the output is bit-perfect or goes through DSP/resampling.

#![forbid(unsafe_code)]

use amp_adapter_gui::create_capture_engine;
use amp_adapter_gui::main_window::stream_label;
use amp_core_engine::player::StreamInfo;
use amp_domain::{AudioSource, Track, TrackMetadata};
use std::path::Path;
use std::time::{Duration, Instant};

fn fixture(file: &str) -> Track {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../amp-adapter-decoder/tests/fixtures")
        .join(file);
    Track::new(
        AudioSource::LocalFile {
            path: path.to_string_lossy().into_owned(),
        },
        TrackMetadata::new("Fixture", "multiplay"),
    )
}

fn info_while_playing(file: &str, volume: f32) -> StreamInfo {
    let (engine, _out) = create_capture_engine().expect("capture engine");
    engine.set_volume(volume);
    engine.play(fixture(file));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(info) = engine.stream_info() {
            return info;
        }
        assert!(Instant::now() < deadline, "no stream info for {file}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn a_44k_file_on_a_48k_output_is_resampled() {
    let info = info_while_playing("flac_16_44k.flac", 1.0);
    assert_eq!((info.source_rate, info.output_rate), (44_100, 48_000));
    assert!(!info.bit_perfect);
}

#[test]
fn a_48k_file_at_unity_volume_is_bit_perfect() {
    let info = info_while_playing("wav_24.wav", 1.0);
    assert_eq!((info.source_rate, info.output_rate), (48_000, 48_000));
    assert!(info.bit_perfect);
}

#[test]
fn turning_the_volume_down_is_dsp_too() {
    assert!(!info_while_playing("wav_24.wav", 0.85).bit_perfect);
}

#[test]
fn the_faceplate_label_reads_like_a_spec_sheet() {
    let mut meta = TrackMetadata::new("t", "a");
    meta.bit_depth = Some(24);
    meta.channels = Some(2);
    let perfect = StreamInfo {
        source_rate: 96_000,
        output_rate: 96_000,
        bit_perfect: true,
    };
    assert_eq!(
        stream_label(Some(&perfect), &meta),
        "96 kHz · 24-bit · ST · BIT-PERFECT"
    );

    let mut mp3 = TrackMetadata::new("t", "a");
    mp3.bitrate_kbps = Some(320);
    mp3.channels = Some(1);
    let resampled = StreamInfo {
        source_rate: 44_100,
        output_rate: 48_000,
        bit_perfect: false,
    };
    assert_eq!(
        stream_label(Some(&resampled), &mp3),
        "44.1 kHz · 320 kbps · MONO · DSP · 48 kHz OUT"
    );

    // Nothing playing: no made-up format.
    assert_eq!(stream_label(None, &TrackMetadata::new("t", "a")), "");
}
