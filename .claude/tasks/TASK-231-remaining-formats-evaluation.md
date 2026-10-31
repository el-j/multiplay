# TASK-231: Remaining Formats: DSD, WavPack, APE, Musepack, TTA, WMA

## Plan Reference
- Plan: [PLAN-058](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-058-universal-audio-format-support.md)
- Role: `codec_engineering`
- Priority: **P2**
- Depends on: TASK-225, TASK-226

## Objectives
1. **DSD (DSF/DFF):** implement in pure Rust: both containers are simple; convert 1-bit DSD64/128
   to PCM (88.2/176.4 kHz) with a decimating low-pass filter. Native DSD/DoP output is out of scope.
2. **WavPack, APE (Monkey's Audio), Musepack, TTA:** look for audited pure-Rust decoders
   (Symphonia upstream or crates.io); adopt behind a fixture only if they pass `verify_pure_rust.sh`,
   cargo-deny and a bit-exact check against reference output. Otherwise record `Unsupported { reason }`.
3. **WMA / other proprietary codecs:** record `Unsupported` with a clear reason (no pure-Rust
   decoder, patent/licensing), shown to the user by TASK-225's skipped-file report.
4. Short ADR addendum listing each format's decision and when to re-check.

## Acceptance Criteria (user-visible / measurable)
- DSF and DFF fixtures play through the engine with correct duration and no clipping.
- Every format in this task is either `Playable` with a passing fixture or `Unsupported` with a reason the UI shows.
- Pure-Rust verification passed (`./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test`).
- File length within `.nexus/policy.json` `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-decoder/src/dsd/` (new)
- `crates/amp-domain/src/formats.rs`

## Automated Verification Command
`./scripts/verify_pure_rust.sh && cargo test -p amp-adapter-decoder`

## Manual Verification & Evidence
- **Build / Run Command:** `cargo run --release -p amp-adapter-gui --bin multiplay-gui`
- **Observed Behavior:** A DSF file plays; a WMA file is listed as skipped with its reason.
- **Evidence Links:** Screenshot.

## Status
- **Status:** Pending

## Implementation Evidence (2026-10-31)
- Decision record: [ADR-006](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/adr/ADR-006-remaining-audio-formats.md).
- **DSD** (`src/dsd/`): DSF + DSDIFF parsing, three-stage decimation to 88.2 kHz, seeking with filter warm-up; factory routes by extension and by header. CPU: 2.94 % (single 1024-tap FIR) → 0.86 % after multistage filtering and removing a per-sample buffer shift. Fixtures `dsd_stereo.dsf/.dff` (sigma-delta generator, verified with ffmpeg: 440/660 Hz, RMS 0.354). Tests `tests/dsd.rs` (3) + engine playback; GUI imports DSD with the decoder's duration (`drop_ingest::dsd_files_are_added_with_their_duration`).
- **WavPack** (`src/wavpack.rs`): `wavicle =0.1.0`, block-by-block, stops at APEv2 tags, exact seeks. Fixtures `wavpack_16.wv`, `wavpack_24.wv` (ffmpeg, tagged). Tests `tests/wavpack.rs` (2): lossless vs formula by name and as stream, frame-exact seek; engine playback; tags via lofty.
- APE, TTA, Musepack, WMA stay unplayable with updated reasons (see ADR-006).
- Gate: verify_pure_rust, cargo-deny, clippy `-D warnings`, `cargo test --workspace --no-fail-fast` 508 passed / 0 failed.
