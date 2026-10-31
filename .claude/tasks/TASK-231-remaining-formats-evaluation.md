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
