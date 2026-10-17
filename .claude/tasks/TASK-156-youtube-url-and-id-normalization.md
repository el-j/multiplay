# TASK-156: YouTube URL/ID Normalization (watch, youtu.be, shorts, music, playlists)

## Plan Reference
- Plan: [PLAN-043](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-043-youtube-resolver-and-downloader-hardening.md)
- Role: `integrations`
- Priority: **P1**
- Audit findings: `SRC-08` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Domain-level parser extracting 11-char IDs from all URL forms; playlist URLs expand to multiple tracks; timestamps preserved as start offset.
2. Fix GUI `ingest_url` and M3U import to store IDs, not URLs.

## Acceptance Criteria (user-visible / measurable)
- Table-driven tests for ≥ 15 URL shapes; export/import round-trip clean.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-domain/src/source_url.rs`
- `crates/amp-adapter-gui/src/app_state.rs`

## Verification Command
`cargo test -p amp-domain source_url`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
