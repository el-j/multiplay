//! DSD playback (TASK-231): DSF and DSDIFF files (1-bit DSD64) are converted to PCM in pure Rust.
//! The fixtures come from `scripts/make_dsd_fixtures.py` and were cross-checked with ffmpeg's
//! DSD decoder: left 440 Hz, right 660 Hz, amplitude 0.5, 0.25 s.

#![forbid(unsafe_code)]

use amp_adapter_decoder::SymphoniaDecoderFactory;
use amp_ports::{DecoderFactoryPort, PcmStreamPort};
use std::path::{Path, PathBuf};
use std::time::Duration;

const FILES: [&str; 2] = ["dsd_stereo.dsf", "dsd_stereo.dff"];
/// DSD64 decimated by 32.
const PCM_RATE: u32 = 88_200;

fn path(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(file)
}

fn decode_all(dec: &mut dyn PcmStreamPort) -> Vec<f32> {
    let mut out = Vec::new();
    let mut buf = vec![0.0f32; 4096];
    loop {
        let n = dec.read_interleaved(&mut buf).expect("decodes");
        if n == 0 {
            return out;
        }
        out.extend_from_slice(&buf[..n]);
    }
}

/// (rms, pitch in Hz) of one channel over the middle half.
fn analyse(samples: &[f32], channel: usize) -> (f32, f32) {
    let ch: Vec<f32> = samples.iter().skip(channel).step_by(2).copied().collect();
    let mid = &ch[ch.len() / 4..3 * ch.len() / 4];
    let rms = (mid.iter().map(|x| x * x).sum::<f32>() / mid.len() as f32).sqrt();
    let ups = mid.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
    (rms, ups as f32 * PCM_RATE as f32 / mid.len() as f32)
}

fn check(file: &str, dec: &mut dyn PcmStreamPort) {
    assert_eq!(dec.sample_rate(), PCM_RATE, "{file}");
    let samples = decode_all(dec);
    assert!(
        samples.iter().all(|s| s.is_finite() && s.abs() < 1.0),
        "{file}: range"
    );
    let frames = samples.len() / 2;
    assert!(frames.abs_diff(22_050) <= 300, "{file}: {frames} frames");
    for (ch, freq) in [(0, 440.0f32), (1, 660.0)] {
        let (rms, pitch) = analyse(&samples, ch);
        assert!((rms - 0.3536).abs() < 0.035, "{file} ch{ch}: rms {rms}");
        assert!(
            (pitch - freq).abs() / freq < 0.03,
            "{file} ch{ch}: pitch {pitch}"
        );
    }
}

#[test]
fn dsf_and_dff_files_play_as_pcm() {
    for file in FILES {
        let mut dec = SymphoniaDecoderFactory
            .open_file(&path(file).to_string_lossy())
            .unwrap_or_else(|e| panic!("{file}: {e}"));
        let d = dec.duration().expect("duration");
        assert!((d.as_secs_f64() - 0.25).abs() < 0.005, "{file}: {d:?}");
        check(file, dec.as_mut());
    }
}

#[test]
fn dsd_streams_are_recognised_without_a_file_name() {
    for file in FILES {
        let stream = Box::new(std::fs::File::open(path(file)).unwrap());
        let mut dec = SymphoniaDecoderFactory
            .open_stream(stream)
            .unwrap_or_else(|e| panic!("{file}: {e}"));
        check(file, dec.as_mut());
    }
}

#[test]
fn dsd_seeks_to_the_middle() {
    for file in FILES {
        let mut dec = SymphoniaDecoderFactory
            .open_file(&path(file).to_string_lossy())
            .unwrap();
        let landed = dec.seek(Duration::from_millis(125)).expect("seek");
        assert!(
            (landed.as_secs_f64() - 0.125).abs() < 0.002,
            "{file}: {landed:?}"
        );
        let rest = decode_all(dec.as_mut());
        let frames = rest.len() / 2;
        assert!(
            frames.abs_diff(11_025) <= 300,
            "{file}: {frames} frames after seek"
        );
        let (rms, pitch) = analyse(&rest, 0);
        assert!(
            (rms - 0.3536).abs() < 0.035 && (pitch - 440.0).abs() < 15.0,
            "{file}"
        );
    }
}
