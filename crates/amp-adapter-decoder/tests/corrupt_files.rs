//! Damaged audio fails gracefully (TASK-184, TASK-185): every fixture truncated, bit-flipped or
//! replaced by garbage either refuses to open or decodes a bounded amount, without panicking or
//! hanging. Covers our own DSD, WavPack and Opus code as well as Symphonia's formats.

#![forbid(unsafe_code)]

use amp_adapter_decoder::SymphoniaDecoderFactory;
use amp_ports::DecoderFactoryPort;
use proptest::prelude::*;

/// A damaged header once asked for 64 GiB: on Linux that aborts the process, which no error
/// handling can catch, while macOS hands out the memory lazily and passed. A 1 GiB ceiling makes
/// such a request fail the same way on every system.
#[global_allocator]
static ALLOC: cap::Cap<std::alloc::System> = cap::Cap::new(std::alloc::System, 1 << 30);
use std::io::Cursor;
use std::path::Path;
use std::time::{Duration, Instant};

/// Reads at most this much per damaged file; a fixture is a second or less.
const MAX_READS: usize = 4_000;
const TIME_LIMIT: Duration = Duration::from_secs(5);

fn fixtures() -> Vec<(String, Vec<u8>)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| {
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

/// Opens and drains `bytes`; panics only if decoding panics, loops or runs away.
fn survive(label: &str, bytes: Vec<u8>) {
    let start = Instant::now();
    let Ok(mut dec) = SymphoniaDecoderFactory.open_stream(Box::new(Cursor::new(bytes))) else {
        return;
    };
    let mut buf = vec![0.0f32; 4096];
    let mut seeked = false;
    for _ in 0..MAX_READS {
        match dec.read_interleaved(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => assert!(
                buf[..n].iter().all(|s| s.is_finite()),
                "{label}: non-finite samples"
            ),
        }
        if !seeked {
            seeked = true;
            let _ = dec.seek(Duration::from_millis(300));
        }
        assert!(
            start.elapsed() < TIME_LIMIT,
            "{label}: still decoding after {TIME_LIMIT:?}"
        );
    }
    let _ = dec.duration();
}

/// A small deterministic generator so failures name a reproducible case.
fn lcg(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *seed >> 33
}

#[test]
fn truncated_files_fail_gracefully() {
    for (name, bytes) in fixtures() {
        for keep in [
            0,
            4,
            12,
            44,
            bytes.len() / 10,
            bytes.len() / 2,
            bytes.len() * 9 / 10,
        ] {
            survive(
                &format!("{name} cut to {keep}"),
                bytes[..keep.min(bytes.len())].to_vec(),
            );
        }
    }
}

#[test]
fn bit_flipped_files_fail_gracefully() {
    for (name, bytes) in fixtures() {
        if bytes.is_empty() {
            continue;
        }
        for case in 0..12u64 {
            let mut seed = case + 1;
            let mut damaged = bytes.clone();
            for _ in 0..(1 + case * 3) {
                let i = lcg(&mut seed) as usize % damaged.len();
                damaged[i] ^= 1 << (lcg(&mut seed) % 8);
            }
            survive(&format!("{name} flip case {case}"), damaged);
        }
    }
}

#[test]
fn a_garbage_middle_fails_gracefully() {
    for (name, bytes) in fixtures() {
        let mut damaged = bytes.clone();
        let (from, to) = (bytes.len() / 3, bytes.len() * 2 / 3);
        let mut seed = 7;
        for b in &mut damaged[from..to] {
            *b = lcg(&mut seed) as u8;
        }
        survive(&format!("{name} garbage middle"), damaged);
    }
}

/// Cases per property: the default in CI, many more in the nightly fuzz job (`FUZZ_CASES`).
fn cases(default: u32) -> u32 {
    std::env::var("FUZZ_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(cases(256)))]

    /// Garbage behind each magic number our own parsers (DSD, DSDIFF, WavPack, Ogg Opus) and
    /// Symphonia's probes look for.
    #[test]
    fn garbage_behind_a_magic_number(
        magic in prop::sample::select(vec![
            b"DSD ".to_vec(), b"FRM8".to_vec(), b"wvpk".to_vec(), b"OggS".to_vec(),
            b"fLaC".to_vec(), b"RIFF".to_vec(), b"ID3\x03".to_vec(), b"FORM".to_vec(), b"caff".to_vec(),
            vec![0x1a, 0x45, 0xdf, 0xa3],
        ]),
        rest in proptest::collection::vec(any::<u8>(), 0..2048),
    ) {
        let mut bytes = magic;
        bytes.extend(rest);
        survive("magic+garbage", bytes);
    }
}
