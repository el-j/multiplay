# TASK-159: New `amp-adapter-bandcamp`: Search, Album/Track Parsing, Streams, Purchase Links

## Plan Reference
- Plan: [PLAN-044](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-044-bandcamp-and-soundcloud-resolvers.md)
- Role: `integrations`
- Priority: **P1**
- Audit findings: `SRC-11` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Search via public search endpoints; parse `data-tralbum` JSON for track list, `mp3-128` stream URLs, artwork, durations.
2. Album URL ingestion expands to tracks; every result carries the purchase/artist page link.
3. Implement `ProviderSearchPort` + `StreamResolverPort`; Cargo feature `bandcamp`.

## Acceptance Criteria (user-visible / measurable)
- Recorded-fixture tests for search and album pages; manual: search, play, open purchase page.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-bandcamp/`

## Verification Command
`cargo test -p amp-adapter-bandcamp`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
