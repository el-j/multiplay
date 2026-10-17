# TASK-137: ICY Metadata Demux & PLS/M3U/XSPF Radio Playlist Resolution

## Plan Reference
- Plan: [PLAN-039](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-039-decoder-and-progressive-streaming-correctness.md)
- Role: `integrations`
- Priority: **P0**
- Audit findings: `SRC-04`, `SRC-05` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-135

## Objectives
1. Parse `icy-metaint`; strip metadata blocks from the audio byte stream; emit `StreamTitle` as `PlayerEvent::NowPlayingMetadata`.
2. Detect playlist bodies by content-type/extension; resolve entries with failover; follow redirects with limit.

## Acceptance Criteria (user-visible / measurable)
- SomaFM PLS URL plays continuously for 10 min with titles updating (recorded-fixture test + manual).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-http/src/icy.rs`
- `crates/amp-adapter-http/src/playlist.rs`

## Verification Command
`cargo test -p amp-adapter-http`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
