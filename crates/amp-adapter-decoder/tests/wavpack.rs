//! WavPack playback (TASK-231) through `wavicle`, block by block: lossless 16/24-bit files with
//! the APEv2 tag real files carry, opened by name and as a hint-less stream, with exact seeking.

#![forbid(unsafe_code)]

use amp_adapter_decoder::SymphoniaDecoderFactory;
use amp_ports::{DecoderFactoryPort, PcmStreamPort};
use std::f64::consts::PI;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// (file, rate, seconds)
const FILES: [(&str, u32, f64); 2] = [
    ("wavpack_16.wv", 44_100, 1.0),
    ("wavpack_24.wv", 48_000, 0.5),
];

fn path(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(file)
}

fn decode_all(dec: &mut dyn PcmStreamPort) -> Vec<f32> {
    let mut out = Vec::new();
    let mut buf = vec![0.0f32; 3000];
    loop {
        let n = dec.read_interleaved(&mut buf).expect("decodes");
        if n == 0 {
            return out;
        }
        out.extend_from_slice(&buf[..n]);
    }
}

/// Largest difference from the known signal (left 440 Hz, right 660 Hz, 0.5) from `first`.
fn max_error(samples: &[f32], first: usize, rate: u32) -> f32 {
    samples
        .as_chunks::<2>()
        .0
        .iter()
        .enumerate()
        .flat_map(|(i, lr)| {
            let t = (first + i) as f64 / rate as f64;
            [
                (lr[0] as f64 - 0.5 * (2.0 * PI * 440.0 * t).sin()).abs(),
                (lr[1] as f64 - 0.5 * (2.0 * PI * 660.0 * t).sin()).abs(),
            ]
        })
        .fold(0.0, f64::max) as f32
}

#[test]
fn wavpack_decodes_losslessly_by_name_and_as_a_stream() {
    for (file, rate, seconds) in FILES {
        let by_name = SymphoniaDecoderFactory
            .open_file(&path(file).to_string_lossy())
            .unwrap_or_else(|e| panic!("{file}: {e}"));
        let by_stream = SymphoniaDecoderFactory
            .open_stream(Box::new(std::fs::File::open(path(file)).unwrap()))
            .unwrap_or_else(|e| panic!("{file} as stream: {e}"));
        for mut dec in [by_name, by_stream] {
            assert_eq!(dec.sample_rate(), rate, "{file}");
            let d = dec.duration().expect("duration");
            assert!((d.as_secs_f64() - seconds).abs() < 0.001, "{file}: {d:?}");
            let samples = decode_all(dec.as_mut());
            assert_eq!(
                samples.len() / 2,
                (rate as f64 * seconds) as usize,
                "{file}"
            );
            let err = max_error(&samples, 0, rate);
            assert!(err < 1e-3, "{file}: off by {err}");
        }
    }
}

#[test]
fn wavpack_seeks_to_the_exact_frame() {
    for (file, rate, seconds) in FILES {
        let mut dec = SymphoniaDecoderFactory
            .open_file(&path(file).to_string_lossy())
            .unwrap();
        let target = Duration::from_secs_f64(seconds * 0.6);
        let landed = dec.seek(target).expect("seek");
        assert!(
            (landed.as_secs_f64() - target.as_secs_f64()).abs() < 1e-4,
            "{file}"
        );
        let mut buf = vec![0.0f32; 512];
        let n = dec.read_interleaved(&mut buf).unwrap();
        let first = (target.as_secs_f64() * rate as f64).round() as usize;
        assert!(
            max_error(&buf[..n], first, rate) < 1e-3,
            "{file}: wrong frame after seek"
        );
    }
}
