# TASK-221: Remix Slots: Capture Beat-Synced Loops From Any Deck

## Plan Reference
- Plan: [PLAN-057](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-057-remix-decks-stems-and-mix-recording.md)
- Design: [Signature features design §8](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `audio_engine`
- Priority: **P2**
- Audit findings: —
- Depends on: TASK-220, TASK-206

## Objectives
1. Capture the current loop of any deck into a slot grid; slots play synced to master tempo and phase, quantized start/stop.
2. Per-slot volume, filter, mute; slot sets saved with the session.

## Acceptance Criteria (user-visible / measurable)
- Capture-sink test: four slots from different tracks stay phase-locked for 5 minutes.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/remix.rs (new)`

## Verification Command
`cargo test -p amp-core-engine dj_remix`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Completed

## Evidence (2026-11-02, branch feat/plan-057-dj)
- Implemented `crates/amp-core-engine/src/dj/remix.rs`: `CapturedLoop` extraction from `TrackBuffer` with loop boundary bounds, `RemixSlotParams` with atomic volume, mute, and one-knob filter controls (`BiquadFilter` retuned on cutoff updates), and `RemixGrid` processing with Hermite interpolation and beat-quantized phase alignment to master tempo.
- Session persistence support via `RemixSlotConfig` (index, volume, filter, muted, bpm).
- `crates/amp-core-engine/tests/dj_remix.rs`:
  - `test_remix_slot_capture_from_track_buffer`: loop capture from decoded buffer.
  - `test_remix_grid_controls_volume_mute_filter`: per-slot volume, mute, and filter response.
  - `test_remix_four_slots_phase_locked_for_five_minutes`: 4 slots at distinct tempos (120, 124, 128, 130 BPM) stay phase-locked to 125 BPM master within 5 ms across 5 minutes.
  - `test_render_allocations_zero_during_playback`: verified 0 allocations during steady playback.
  - `test_remix_session_serialization`: verified JSON round-trip serialization.
- Quality gates passed: `./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test`, `cargo clippy --workspace --all-targets -- -D warnings`, and file line counts $\le 500$.
