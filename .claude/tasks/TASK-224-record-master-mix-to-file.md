# TASK-224: Record the Master Mix to WAV/FLAC

## Plan Reference
- Plan: [PLAN-057](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-057-remix-decks-stems-and-mix-recording.md)
- Design: [Signature features design §8](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `audio_engine`
- Priority: **P2**
- Audit findings: —
- Depends on: TASK-207

## Objectives
1. Record button captures the master bus to WAV or FLAC with session metadata; disk writes off the audio thread via a lock-free ring.
2. Recording added to the library when finished.

## Acceptance Criteria (user-visible / measurable)
- Test: recorded file is bit-identical to the capture-sink master output for a scripted mix.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/recorder.rs (new)`

## Verification Command
`cargo test -p amp-core-engine dj_record`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending

## Evidence (2026-11-01, branch feat/plan-057-dj)
- `crates/amp-core-engine/src/dj/recorder.rs` (tap + control, rtrb ring, writer thread) and `recorder_sinks.rs` (streaming float WAV, streaming 24-bit FLAC via pure-Rust `flacenc` with default features off, lofty tags).
- `cargo test -p amp-core-engine dj_record`: WAV bit-identical to the rendered master over a scripted two-deck mix with crossfader moves; FLAC within 24-bit rounding, decodes with our Symphonia adapter, tagged title/artist; second start and unwritable path refused; render allocates 0 times while recording (allocation guard).
- `crates/amp-adapter-gui/tests/dj_record.rs`: REC in the console saves `multiplay mix <date>.flac` beside the music folder (else the app folder) and the mix reaches the playlist.
- verify_pure_rust (+ self-test), cargo deny, clippy -D warnings, cargo test --workspace: 635 passed.
- Not yet: a release-build manual demo; format choice in the UI (FLAC only from the button).
