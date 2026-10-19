# TASK-204: Background Track Analysis: BPM, Beat Grid, Waveforms (Cached)

## Plan Reference
- Plan: [PLAN-054](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-054-dj-engine-multi-deck-cue-and-beat-analysis.md)
- Design: [Signature features design §5](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `dsp_engineering`
- Priority: **P0**
- Audit findings: —
- Depends on: TASK-202, TASK-148

## Objectives
1. Background analysis job: BPM, beat grid with downbeat phase, overview waveform and three-band detail waveform; optional musical key (P2).
2. Cache results in the library DB keyed by stable track ID with an analysis version; analyse on load when missing, never on the audio thread.
3. Reuse/replace `beat_detector.rs` based on measured accuracy.

## Acceptance Criteria (user-visible / measurable)
- BPM within ±0.1 on synthetic click tracks (90–175 BPM) and within ±1 % on the TASK-184 corpus; beat-grid phase error < 10 ms.
- A 5-minute track analyses in < 3 s on CI release build.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/analysis/ (new)`
- `crates/amp-adapter-storage/src/`

## Verification Command
`cargo test -p amp-core-engine dj_analysis`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
