# TASK-152: Lossless Playlist I/O: M3U/M3U8 (relative paths), PLS, XSPF

## Plan Reference
- Plan: [PLAN-041](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-041-persistent-library-config-and-secrets.md)
- Role: `storage_and_state`
- Priority: **P2**
- Audit findings: `DAT-06`, `SRC-02`, `SRC-08` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-156

## Objectives
1. Resolve relative entries against the playlist file; write relative paths when inside the same tree.
2. Canonical source URIs round-trip (`multiplay:` scheme for non-URL sources).
3. PLS and XSPF import/export; dedupe on import.

## Acceptance Criteria (user-visible / measurable)
- Round-trip property test: export → import yields identical sources for all `AudioSource` variants.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-storage/src/playlist.rs`

## Verification Command
`cargo test -p amp-adapter-storage playlist`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
