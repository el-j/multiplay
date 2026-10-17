# TASK-165: Soulseek Sharing: Share Index, Browse/Search Responses, Upload Slots & Limits

## Plan Reference
- Plan: [PLAN-045](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-045-real-soulseek-client.md)
- Role: `distributed_systems`
- Priority: **P1**
- Audit findings: `SRC-16` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-163, TASK-149

## Objectives
1. Build share index from `shared_directories` (from library DB); respond to search and browse requests; upload queue with `max_upload_slots` and bandwidth caps.
2. Ignore/ban list; share-count reporting; privacy: never share outside configured folders (path canonicalization tests).

## Acceptance Criteria (user-visible / measurable)
- Another client (e.g. Nicotine+) can browse and download from multiplay.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-soulseek/src/share.rs`

## Verification Command
`cargo test -p amp-adapter-soulseek share`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
