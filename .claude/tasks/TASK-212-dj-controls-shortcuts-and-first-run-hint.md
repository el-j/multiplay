# TASK-212: DJ Controls, Keyboard Map, Cheat Sheet & First-Run Hint

## Plan Reference
- Plan: [PLAN-055](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-055-dj-mode-experience-console-assignment-waveforms.md)
- Design: [Signature features design §6](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-203, TASK-205, TASK-206, TASK-207, TASK-209

## Objectives
1. Per deck: play/pause, cue, sync, PFL, loop A/B, beat-loop size, hot cues 1–8, pitch fader, nudge; crossfader; cue mix/volume.
2. Full keyboard map registered via the TASK-175 shortcut map; cheat-sheet overlay (?).
3. First-time three-step hint (assign → cue → fade); defaults: sync on, quantize on, auto mix off; no single control can stop the on-air deck by accident.

## Acceptance Criteria (user-visible / measurable)
- Every control reachable by mouse and keyboard (test over the shortcut table).
- Usability check with two first-time users recorded in the evidence.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/dj_controls.rs (new)`

## Verification Command
`cargo test -p amp-adapter-gui dj_controls`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
