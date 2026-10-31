//! Golden decode matrix (TASK-226, PLAN-058): every playable format is decoded from a real file
//! made by `scripts/make_audio_fixtures.sh`. All fixtures carry the same signal (left 440 Hz,
//! right 660 Hz, amplitude 0.5), so lossless files are checked sample by sample against the
//! formula and lossy files by level and pitch.

#![forbid(unsafe_code)]

use amp_adapter_decoder::{AudioStreamDecoder, GenericMediaSource};
use amp_domain::formats::{self, FormatStatus};
use std::f64::consts::PI;
use std::path::{Path, PathBuf};
use std::time::Duration;

struct Fixture {
    file: &'static str,
    rate: u32,
    seconds: f64,
    kind: Kind,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Bit-exact up to quantization: compare with the formula.
    Lossless,
    /// Compare level and pitch in the middle of the file.
    Lossy,
    /// 5.1 downmixed to stereo: check length and that both sides carry sound.
    Surround,
}

const FIXTURES: &[Fixture] = &[
    Fixture {
        file: "flac_16_44k.flac",
        rate: 44_100,
        seconds: 1.0,
        kind: Kind::Lossless,
    },
    Fixture {
        file: "flac_24_96k.flac",
        rate: 96_000,
        seconds: 0.5,
        kind: Kind::Lossless,
    },
    Fixture {
        file: "flac_24_192k.flac",
        rate: 192_000,
        seconds: 0.25,
        kind: Kind::Lossless,
    },
    Fixture {
        file: "flac_no_seektable.flac",
        rate: 44_100,
        seconds: 1.0,
        kind: Kind::Lossless,
    },
    Fixture {
        file: "flac_5_1_48k.flac",
        rate: 48_000,
        seconds: 0.5,
        kind: Kind::Surround,
    },
    Fixture {
        file: "wav_16.wav",
        rate: 44_100,
        seconds: 1.0,
        kind: Kind::Lossless,
    },
    Fixture {
        file: "wav_24.wav",
        rate: 48_000,
        seconds: 0.5,
        kind: Kind::Lossless,
    },
    Fixture {
        file: "wav_f32.wav",
        rate: 48_000,
        seconds: 0.5,
        kind: Kind::Lossless,
    },
    Fixture {
        file: "mp3_cbr_128.mp3",
        rate: 44_100,
        seconds: 1.0,
        kind: Kind::Lossy,
    },
    Fixture {
        file: "mp3_vbr.mp3",
        rate: 44_100,
        seconds: 1.0,
        kind: Kind::Lossy,
    },
    Fixture {
        file: "aac.m4a",
        rate: 44_100,
        seconds: 1.0,
        kind: Kind::Lossy,
    },
    Fixture {
        file: "vorbis.ogg",
        rate: 44_100,
        seconds: 1.0,
        kind: Kind::Lossy,
    }, // TASK-227: Symphonia formats switched on.
    Fixture {
        file: "alac.m4a",
        rate: 44_100,
        seconds: 1.0,
        kind: Kind::Lossless,
    },
    Fixture {
        file: "aiff_16.aiff",
        rate: 44_100,
        seconds: 1.0,
        kind: Kind::Lossless,
    },
    Fixture {
        file: "caf_16.caf",
        rate: 44_100,
        seconds: 1.0,
        kind: Kind::Lossless,
    },
    Fixture {
        file: "flac.mka",
        rate: 44_100,
        seconds: 1.0,
        kind: Kind::Lossless,
    },
    Fixture {
        file: "vorbis.webm",
        rate: 44_100,
        seconds: 1.0,
        kind: Kind::Lossy,
    },
    Fixture {
        file: "mp2_192.mp2",
        rate: 44_100,
        seconds: 1.0,
        kind: Kind::Lossy,
    },
    Fixture {
        file: "adpcm_ms.wav",
        rate: 44_100,
        seconds: 1.0,
        kind: Kind::Lossy,
    },
];

/// Runs `check` on every fixture and reports all failing ones together.
fn each_fixture(check: impl Fn(&Fixture) + std::panic::RefUnwindSafe) {
    let failures: Vec<String> = FIXTURES
        .iter()
        .filter_map(|f| {
            let err = std::panic::catch_unwind(|| check(f)).err()?;
            let msg = err
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| err.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            Some(format!("  {}: {msg}", f.file))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} fixtures failed:\n{}",
        failures.len(),
        FIXTURES.len(),
        failures.join("\n")
    );
}

fn path(f: &Fixture) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(f.file)
}

/// Decodes everything left in `dec` into interleaved stereo samples.
fn decode_all(dec: &mut AudioStreamDecoder) -> Vec<f32> {
    let mut out = Vec::new();
    let mut buf = vec![0.0f32; 8192];
    loop {
        let n = dec
            .read_interleaved_samples(&mut buf)
            .expect("decodes to the end without errors");
        if n == 0 {
            return out;
        }
        out.extend_from_slice(&buf[..n]);
    }
}

fn expected(channel: usize, frame: usize, rate: u32) -> f32 {
    let freq = if channel == 0 { 440.0 } else { 660.0 };
    (0.5 * (2.0 * PI * freq * frame as f64 / rate as f64).sin()) as f32
}

/// Largest difference between decoded samples (starting at `first_frame`) and the formula.
fn max_error(samples: &[f32], first_frame: usize, rate: u32) -> f32 {
    samples
        .as_chunks::<2>()
        .0
        .iter()
        .enumerate()
        .flat_map(|(i, lr)| {
            let f = first_frame + i;
            [
                (lr[0] - expected(0, f, rate)).abs(),
                (lr[1] - expected(1, f, rate)).abs(),
            ]
        })
        .fold(0.0, f32::max)
}

fn rms(xs: impl Iterator<Item = f32>) -> f32 {
    let (sum, n) = xs.fold((0.0f64, 0usize), |(s, n), x| {
        (s + (x as f64).powi(2), n + 1)
    });
    (sum / n.max(1) as f64).sqrt() as f32
}

/// Frequency from upward zero crossings of one channel.
fn pitch(samples: &[f32], channel: usize, rate: u32) -> f32 {
    let ch: Vec<f32> = samples.iter().skip(channel).step_by(2).copied().collect();
    let ups = ch.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
    ups as f32 * rate as f32 / ch.len() as f32
}

fn check_signal(f: &Fixture, samples: &[f32]) {
    let frames = samples.len() / 2;
    let want = (f.seconds * f.rate as f64).round() as usize;
    match f.kind {
        Kind::Lossless => {
            assert_eq!(frames, want, "{}: frame count", f.file);
            let err = max_error(samples, 0, f.rate);
            assert!(err < 1e-3, "{}: off by {err}", f.file);
        }
        Kind::Lossy => {
            // Encoders pad a little; gapless trimming keeps it within ~50 ms.
            let slack = f.rate as usize / 20;
            assert!(
                frames.abs_diff(want) <= slack,
                "{}: {frames} frames, want ~{want}",
                f.file
            );
            let mid = &samples[frames / 2..frames / 2 + (f.rate as usize / 4) * 2];
            for (ch, freq) in [(0usize, 440.0f32), (1, 660.0)] {
                let level = rms(mid.iter().skip(ch).step_by(2).copied());
                assert!(
                    (level - 0.3536).abs() < 0.035,
                    "{}: ch{ch} rms {level}",
                    f.file
                );
                let p = pitch(mid, ch, f.rate);
                assert!(
                    (p - freq).abs() / freq < 0.03,
                    "{}: ch{ch} pitch {p}",
                    f.file
                );
            }
        }
        Kind::Surround => {
            assert_eq!(frames, want, "{}: frame count", f.file);
            for ch in 0..2 {
                let level = rms(samples.iter().skip(ch).step_by(2).copied());
                assert!(
                    level > 0.1,
                    "{}: ch{ch} silent after downmix ({level})",
                    f.file
                );
            }
        }
    }
}

#[test]
fn every_fixture_decodes_from_its_path() {
    each_fixture(|f| {
        let mut dec = AudioStreamDecoder::open_file(&path(f).to_string_lossy())
            .unwrap_or_else(|e| panic!("{}: {e}", f.file));
        assert_eq!(dec.sample_rate(), f.rate, "{}", f.file);
        let duration = dec
            .duration()
            .unwrap_or_else(|| panic!("{}: no duration", f.file));
        assert!(
            (duration.as_secs_f64() - f.seconds).abs() < 0.06,
            "{}: duration {duration:?}",
            f.file
        );
        check_signal(f, &decode_all(&mut dec));
    });
}

/// Streams (network, phone uploads) arrive without a file name: content alone must suffice.
#[test]
fn every_fixture_decodes_from_a_stream_without_hints() {
    each_fixture(|f| {
        let file = std::fs::File::open(path(f)).unwrap();
        // Symphonia's CAF reader needs the stream length (known for files and for downloads
        // with a Content-Length); every other format must work without it.
        let len = f
            .file
            .ends_with(".caf")
            .then(|| file.metadata().unwrap().len());
        let source = GenericMediaSource::with_byte_len(file, len);
        let mut dec = AudioStreamDecoder::open_media_source(source)
            .unwrap_or_else(|e| panic!("{}: {e}", f.file));
        assert_eq!(dec.sample_rate(), f.rate, "{}", f.file);
        check_signal(f, &decode_all(&mut dec));
    });
}

/// Seeking lands on the exact frame for lossless files, and near it for lossy ones.
#[test]
fn every_fixture_seeks_to_the_middle() {
    each_fixture(|f| {
        let mut dec = AudioStreamDecoder::open_file(&path(f).to_string_lossy()).unwrap();
        let target = Duration::from_secs_f64(f.seconds / 2.0);
        let landed = dec
            .seek(target)
            .unwrap_or_else(|e| panic!("{}: {e}", f.file));
        assert!(
            (landed.as_secs_f64() - target.as_secs_f64()).abs() < 0.03,
            "{}: landed at {landed:?}",
            f.file
        );
        let mut buf = vec![0.0f32; 512];
        let n = dec.read_interleaved_samples(&mut buf).unwrap();
        assert!(n > 0, "{}: nothing after seek", f.file);
        if f.kind == Kind::Lossless {
            // Matroska stores timestamps in whole milliseconds, so a seek there can only be
            // exact to the millisecond; every other lossless container must be sample-exact.
            let slack = if f.file.ends_with(".mka") {
                (f.rate / 1000 + 1) as i64
            } else {
                0
            };
            let first = (target.as_secs_f64() * f.rate as f64).round() as i64;
            let err = (-slack..=slack)
                .map(|d| max_error(&buf[..n], (first + d).max(0) as usize, f.rate))
                .fold(f32::MAX, f32::min);
            assert!(err < 1e-3, "{}: off by {err} after seek", f.file);
        }
    });
}

/// Every format the registry calls playable has a fixture here.
#[test]
fn every_playable_format_has_a_fixture() {
    for format in formats::AUDIO_FORMATS {
        if format.status != FormatStatus::Playable {
            continue;
        }
        let covered = FIXTURES
            .iter()
            .any(|f| formats::by_path(Path::new(f.file)).is_some_and(|x| x.name == format.name));
        assert!(
            covered,
            "{} is playable but has no golden fixture",
            format.name
        );
    }
}
