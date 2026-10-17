# TASK-135: Blocking Precache Reader (Condvar) with Timeouts & Cancellation

## Plan Reference
- Plan: [PLAN-039](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-039-decoder-and-progressive-streaming-correctness.md)
- Role: `audio_engine`
- Priority: **P0**
- Audit findings: `AUD-09` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. `SharedPrecacheBuffer::read` blocks on a `Condvar` until data, EOF, timeout (→ `TimedOut`) or cancellation — never `WouldBlock`.
2. Expose buffering state so the engine emits `Buffering { percent }`.
3. A `CancellationToken` stops the producer task when the stream handle drops.

## Acceptance Criteria (user-visible / measurable)
- Decoding an MP3 served at 8 KB/s with jitter completes with sample-identical output to the local file.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-ports/src/stream.rs`
- `crates/amp-adapter-http/src/lib.rs`
- `crates/amp-adapter-youtube/src/lib.rs`

## Verification Command
`cargo test -p amp-ports precache && cargo test -p amp-adapter-http slow_server`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
