# TASK-160: New `amp-adapter-soundcloud`: client_id Discovery, Search, Progressive/HLS Streams

## Plan Reference
- Plan: [PLAN-044](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-044-bandcamp-and-soundcloud-resolvers.md)
- Role: `integrations`
- Priority: **P1**
- Audit findings: `SRC-12` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Discover/cache `client_id`; api-v2 search; resolve transcodings preferring progressive MP3, falling back to HLS via a segment-concatenating reader.
2. Handle Go+ snippets (mark as preview), rate limiting, and client_id rotation; Cargo feature `soundcloud`.

## Acceptance Criteria (user-visible / measurable)
- Recorded-fixture tests; manual: search and play 10 tracks incl. an HLS-only one.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-soundcloud/`

## Verification Command
`cargo test -p amp-adapter-soundcloud`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
