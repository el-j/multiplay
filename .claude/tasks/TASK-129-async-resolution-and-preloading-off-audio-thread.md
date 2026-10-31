# TASK-129: Async Stream Resolution & Preloading Off the Audio Worker

## Plan Reference
- Plan: [PLAN-037](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-037-unified-core-playback-engine.md)
- Role: `audio_engine`
- Priority: **P0**
- Audit findings: `AUD-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125

## Objectives
1. Resolve + open decoders on Tokio tasks; hand ready decoders to the worker via channel.
2. Preload next track N seconds ahead asynchronously; failures retried with exponential backoff (max 3) and reported as events.
3. Per-provider resolution timeout (default 10 s) with cancellation when the user skips.

## Acceptance Criteria (user-visible / measurable)
- Playing a local track followed by a slow network track (2 s resolve) produces zero underruns (CaptureSink gap detector).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/player/loader.rs`

## Verification Command
`cargo test -p amp-core-engine preload`

## Manual Verification
- Implemented non-blocking Tokio stream resolution (`spawn_track_load`, `PendingLoad`) with cancellation on skip, per-provider resolution timeout, and lookahead preloading with exponential backoff in `crates/amp-core-engine/src/player/loader.rs`.
- Verified with `cargo test -p amp-core-engine preload` (`preload_non_blocking_resolution_cancellation_and_retry` passed).

## Status
- **Status:** Pending
