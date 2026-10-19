# TASK-211: Stacked Beat-Aligned Waveforms, Phase Meter & Overviews

## Plan Reference
- Plan: [PLAN-055](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-055-dj-mode-experience-console-assignment-waveforms.md)
- Design: [Signature features design §6](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-204, TASK-209

## Objectives
1. Scrolling three-band detail waveforms for both decks stacked, beat markers aligned at a fixed playhead; phase meter.
2. Overview waveform per deck with playhead, hot cues and loop region; click to seek.
3. BPM, pitch %, key and time readouts; 60 fps without allocating per frame.

## Acceptance Criteria (user-visible / measurable)
- Waveform render reads the cached analysis (no recompute); frame time < 4 ms for both decks on CI-class hardware.
- Manual: two synced tracks show aligned beat markers.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/dj_waveform.rs (new)`

## Verification Command
`cargo test -p amp-adapter-gui waveform`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
