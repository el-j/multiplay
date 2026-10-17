# TASK-164: Soulseek Transfers: Queue, F-Connection, Resume & Progressive Play

## Plan Reference
- Plan: [PLAN-045](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-045-real-soulseek-client.md)
- Role: `integrations`
- Priority: **P1**
- Audit findings: `SRC-15` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-163, TASK-135

## Objectives
1. QueueUpload, TransferRequest/Response, PlaceInQueue updates, F connection with offset; resume partial files from the cache dir.
2. `resolve_stream` returns a progressive precache stream fed by the transfer; completed files optionally kept in downloads/library.

## Acceptance Criteria (user-visible / measurable)
- Play-while-downloading of a FLAC from a real peer works; interrupted transfer resumes.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-soulseek/src/transfer.rs`

## Verification Command
`cargo test -p amp-adapter-soulseek transfer`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
