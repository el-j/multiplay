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
`cargo test -p amp-core-engine dj_stems`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
