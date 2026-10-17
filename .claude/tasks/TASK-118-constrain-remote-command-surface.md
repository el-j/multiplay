# TASK-118: Constrain Remote Command Surface & Fix Transport Semantics

## Plan Reference
- Plan: [PLAN-035](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-035-remote-control-security-hardening.md)
- Role: `security`
- Priority: **P0**
- Audit findings: `SEC-04`, `ARC-05`, `UX-09` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-117

## Objectives
1. Replace `EnqueueTrack(Box<Track>)` with `EnqueueById(TrackId)` / `EnqueueSearchResult(token)` where tokens are host-issued; arbitrary `LocalFile`/`DirectHttp` never accepted from remotes.
2. Map `Next`/`Previous`/`VoteSkip` to real queue commands (new `PlayerCommand::Next/Previous/Skip`).
3. Version the protocol (`{"v":1,...}`) and reject unknown versions.

## Acceptance Criteria (user-visible / measurable)
- Remote cannot cause the host to open a path or URL not already in the library/search results.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-remote/src/protocol.rs`
- `crates/amp-adapter-remote/src/server.rs`
- `crates/amp-core-engine/src/events.rs`

## Verification Command
`cargo test -p amp-adapter-remote`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
