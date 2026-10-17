# TASK-131: Queue Semantics: Next/Previous/Repeat/Shuffle Owned by the Engine

## Plan Reference
- Plan: [PLAN-037](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-037-unified-core-playback-engine.md)
- Role: `audio_engine`
- Priority: **P1**
- Audit findings: `ARC-05`, `UX-09` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125

## Objectives
1. Engine-owned queue with `Next`, `Previous` (restart if > 3 s in), `RepeatMode {Off, One, All}`, shuffle via existing `SmartQueueManager`.
2. Gapless advancement owned by the engine (removes `check_gapless_advancement` from GUI).
3. Consistent semantics across GUI, TUI and remote.

## Acceptance Criteria (user-visible / measurable)
- Contract tests for all queue transitions incl. end-of-queue and repeat-one.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/player/queue.rs`
- `crates/amp-core-engine/src/playback/shuffle.rs`

## Verification Command
`cargo test -p amp-core-engine queue`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
