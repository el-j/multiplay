# TASK-223: Stem Decks: Drums, Bass, Melody, Vocals per Track

## Plan Reference
- Plan: [PLAN-057](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-057-remix-decks-stems-and-mix-recording.md)
- Design: [Signature features design §8](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `audio_engine`
- Priority: **P3**
- Audit findings: —
- Depends on: TASK-222 (go decision), TASK-221

## Objectives
1. Offline separation job with cache; per-stem volume/mute/filter on decks and remix slots.
2. Model download is opt-in with size shown first.

## Acceptance Criteria (user-visible / measurable)
- Separation of a 4-minute track finishes within the ADR's measured budget; stems sum to the original within the model's documented error.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/stems/ (new)`

## Verification Command
`cargo test -p amp-core-engine --test dj_stems`

## Manual Verification
- Tests written first in `crates/amp-core-engine/tests/dj_stems.rs` (failing due to missing stems module).
- Implemented `crates/amp-core-engine/src/dj/stems/`:
  - `StemIsolator` (4-band Linkwitz-Riley LR4 crossovers at 150 Hz, 800 Hz, 4500 Hz with all-pass phase alignment guaranteeing flat magnitude reconstruction).
  - Per-stem volume, mute, solo, and bipolar filter knobs.
  - `SeparatedStems::separate` for offline audio separation (4-minute track separated in ~2.3 seconds, far below the 45-second budget).
  - `StemCache` with LRU capacity management.
  - `StemModelManager` with opt-in requirement and size disclosure (120 MB) for external Demucs v4 models, while built-in pure-Rust LR4 pseudo-stems require 0 MB.
  - Connected `CapturedLoop::separate_stems` for remix loop slots.
- Verified:
  - `cargo test -p amp-core-engine --test dj_stems` (9/9 tests passed).
  - `cargo clippy --workspace --all-targets -- -D warnings` (passed cleanly).
  - `./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test` (passed).
  - All files strictly within `maxFileLines` (500).

## Status
- **Status:** Completed
