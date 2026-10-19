# TASK-206: Loops & Cues: A-B Loop, Beat Loops, Loop Roll, Hot Cues

## Plan Reference
- Plan: [PLAN-054](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-054-dj-engine-multi-deck-cue-and-beat-analysis.md)
- Design: [Signature features design §5](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `audio_engine`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-202, TASK-204

## Objectives
1. A-B loop: one action sets A, the next sets B; optional quantize to beats; exit/reloop.
2. Beat loops 1/4 … 32 beats with halve/double; loop roll (playback continues in the background, resumes in phase).
3. 8 hot cues per track persisted in the library; sample-accurate, click-free loop seams.

## Acceptance Criteria (user-visible / measurable)
- Capture-sink tests: loop seam has no discontinuity > -60 dBFS step; loop length matches beats at the deck tempo; hot cues survive restart.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/loops.rs (new)`
- `crates/amp-adapter-storage/src/`

## Verification Command
`cargo test -p amp-core-engine dj_loops`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
