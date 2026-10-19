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
- **Status:** Pending
