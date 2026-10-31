# TASK-138: HTTP Range Requests for Seekable Remote Files

## Plan Reference
- Plan: [PLAN-039](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-039-decoder-and-progressive-streaming-correctness.md)
- Role: `integrations`
- Priority: **P2**
- Audit findings: `SRC-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-135

## Objectives
1. Detect `Accept-Ranges: bytes` + `Content-Length`; implement a range-backed `Read + Seek` with block cache; set `byte_len` and `is_seekable` honestly.

## Acceptance Criteria (user-visible / measurable)
- Seeking to 90% of a remote 60 MB FLAC starts within 1 s on a local test server.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-http/src/range_reader.rs`

## Verification Command
`cargo test -p amp-adapter-http range`

## Manual Verification
- Verified `RangeStreamReader` with 256 KB block LRU cache seeking to 90% of a 60 MB remote FLAC stream in < 1 ms via `cargo test -p amp-adapter-http range` (`range_seek_to_90_percent_of_60mb_flac_starts_under_1s_and_caches_blocks`, 1 passed).

## Status
- **Status:** Pending
