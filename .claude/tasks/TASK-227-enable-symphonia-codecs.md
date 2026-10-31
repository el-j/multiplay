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
