# TASK-158: Working Offline Downloader (Async, Correct Containers, Safe Names, Progress)

## Plan Reference
- Plan: [PLAN-043](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-043-youtube-resolver-and-downloader-hardening.md)
- Role: `integrations`
- Priority: **P1**
- Audit findings: `SRC-10` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-135

## Objectives
1. Async streaming copy (blocking read on `spawn_blocking` or async stream API); detect container (probe) for the extension.
2. Cross-platform filename sanitizer (reserved names, `..`, control chars, length); never overwrite (suffix ` (2)`).
3. Implement `TrackDownloadPort` with progress/cancel events; downloads land in the configured dir and are added to the library; UI download manager.

## Acceptance Criteria (user-visible / measurable)
- Downloading a YouTube and an HTTP MP3 produces correctly named, tagged, playable files.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/downloader.rs`

## Verification Command
`cargo test -p amp-core-engine downloader`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
