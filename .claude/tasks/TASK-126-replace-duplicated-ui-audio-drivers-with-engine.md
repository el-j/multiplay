# TASK-126: Replace Both UI Audio Drivers with PlayerEngine; Delete Duplicates

## Plan Reference
- Plan: [PLAN-037](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-037-unified-core-playback-engine.md)
- Role: `architecture`
- Priority: **P1**
- Audit findings: `ARC-03`, `ARC-08` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125

## Objectives
1. Delete `amp-app/src/{audio_driver,track_loader}.rs` and `amp-adapter-gui/src/{audio_driver,playback_worker,audio_pusher,track_loader}.rs`; both UIs talk to the engine via ports/events.
2. Single shared async runtime owned by the composition root.

## Acceptance Criteria (user-visible / measurable)
- No playback logic lives in UI crates (`grep -r AudioStreamDecoder crates/amp-adapter-gui crates/amp-adapter-tui` is empty).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-app/src/`
- `crates/amp-adapter-gui/src/`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
