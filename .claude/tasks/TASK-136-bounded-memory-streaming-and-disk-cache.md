# TASK-136: Bounded-Memory Streaming: Live Ring Buffer & Disk Spill Cache

## Plan Reference
- Plan: [PLAN-039](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-039-decoder-and-progressive-streaming-correctness.md)
- Role: `audio_engine`
- Priority: **P0**
- Audit findings: `AUD-10`, `QA-05` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-135

## Objectives
1. Live (unknown length / radio) streams use a fixed-size sliding window (configurable, default 8 MB).
2. Finite streams > `precache_ram_mb` spill to a cache file in the cache dir (LRU-evicted, size-capped).
3. Background download tasks are cancelled when the stream is dropped (track change).

## Acceptance Criteria (user-visible / measurable)
- 1 h radio playback keeps RSS growth < 10 MB; skipping 50 network tracks leaves no running download tasks.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-ports/src/stream.rs`
- `crates/amp-core-engine/src/precache.rs`

## Verification Command
`cargo test -p amp-core-engine bounded_buffer`

## Manual Verification
- Verified live radio sliding window (`SharedPrecacheBuffer::new_live_ring`), disk spill (`SharedPrecacheBuffer::with_disk_spill`), LRU eviction (`DiskSpillCache`), and 50-stream cancellation on reader drop via `cargo test -p amp-core-engine bounded_buffer` (3 passed).

## Status
- **Status:** Pending
