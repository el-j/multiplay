# TASK-127: Transport Correctness: Epoch-Flushed Ring, Real Pause, No Lost Chunks

## Plan Reference
- Plan: [PLAN-037](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-037-unified-core-playback-engine.md)
- Role: `audio_engine`
- Priority: **P1**
- Audit findings: `AUD-01`, `AUD-02` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125

## Objectives
1. Add an epoch counter shared with the RT callback; on stop/seek/track-change the callback discards stale-epoch samples (or the consumer drains) — zero allocations/locks in the callback.
2. Pause pauses the CPAL stream (and fades 5 ms to avoid clicks); resume restarts it.
3. Reduce ring to ~150 ms (configurable `buffer_ms`), with a separate decode-ahead buffer off the RT path.
4. Pending processed samples survive pause (no mid-chunk drop).

## Acceptance Criteria (user-visible / measurable)
- Pause/stop latency < 30 ms measured via CaptureSink; seek produces no audio from the old position after the command.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-cpal/src/actor.rs`
- `crates/amp-core-engine/src/player/`

## Verification Command
`cargo test -p amp-core-engine transport`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
