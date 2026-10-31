# TASK-227: Enable Symphonia's Remaining Codecs & Containers

## Plan Reference
- Plan: [PLAN-058](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-058-universal-audio-format-support.md)
- Role: `codec_engineering`
- Priority: **P0**
- Depends on: TASK-226

## Objectives
1. Turn on the pure-Rust Symphonia 0.6 features not yet enabled in `amp-adapter-decoder`:
   `alac`, `aiff`, `caf`, `mkv` (MKA and WebM containers), `mp1`, `mp2`, `adpcm`.
2. Mark them `Playable` in the TASK-225 registry (`.alac` inside `.m4a`, `.aif/.aiff/.aifc`,
   `.caf`, `.mka`, `.webm` audio, `.mp1/.mp2`, ADPCM WAV) with fixtures from TASK-226.
3. Re-run `./scripts/verify_pure_rust.sh` and cargo-deny: no new native code.

## Acceptance Criteria (user-visible / measurable)
- ALAC, AIFF, CAF, MKA (Vorbis/FLAC), MP2 and ADPCM fixtures decode and play in the engine.
- Binary size growth of the release GUI measured and noted in the evidence.
- Pure-Rust verification passed (`./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test`).
- File length within `.nexus/policy.json` `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-decoder/Cargo.toml`, `src/stream.rs`
- `crates/amp-domain/src/formats.rs`

## Automated Verification Command
`./scripts/verify_pure_rust.sh && cargo test -p amp-adapter-decoder --test format_matrix`

## Manual Verification & Evidence
- **Build / Run Command:** `cargo run --release -p amp-adapter-gui --bin multiplay-gui`
- **Observed Behavior:** An ALAC album from iTunes and an AIFF file play with tags.
- **Evidence Links:** Screenshot, CI run URL.

## Status
- **Status:** Pending

## Implementation Evidence (2026-10-31)
- `amp-adapter-decoder` Symphonia features now: `mp1, mp2, mp3, flac, pcm, vorbis, aac, alac, adpcm, isomp4, ogg, wav, aiff, caf, mkv`. Registry marks AIFF, CAF, Matroska/WebM audio, MPEG Layer I/II playable; ALAC plays inside `.m4a` ("AAC / ALAC (M4A)").
- New fixtures: `alac.m4a`, `aiff_16.aiff`, `caf_16.caf`, `flac.mka`, `vorbis.webm`, `mp2_192.mp2`, `adpcm_ms.wav` — all 19 fixtures pass the decode matrix and play to the end through the engine.
- Decoder fixes found by the new fixtures:
  - Durations and seeks now use the track's time base (Matroska/WebM tick in milliseconds, not samples): duration falls back from frame count to track duration to file duration; seeks convert with rounding so sample-based formats stay exact (`seek_accuracy` still lands on the exact frame).
  - Symphonia 0.6.1 bug: seeking in ADPCM WAV reports success but leaves the reader at EOF. The decoder now rewinds and decodes forward to the target when a seek is followed by an immediate EOF (generic fallback). Worth reporting upstream.
- Known limits: Matroska seeks are exact to 1 ms (container precision). Symphonia's CAF reader needs the stream length, so CAF works from files and length-known streams but not from length-less streams.
- `cargo deny check` and `verify_pure_rust.sh` pass with the new crates; release `multiplay-gui` (macOS arm64) 39,469,008 → 41,245,024 bytes (+1.8 MB).
- Gate: clippy `-D warnings`, `cargo test --workspace --no-fail-fast` 481 passed / 0 failed.
