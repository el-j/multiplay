# TASK-155: Cross-Source Ranking & Deduplication

## Plan Reference
- Plan: [PLAN-042](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-042-universal-search-wired-end-to-end.md)
- Role: `integrations`
- Priority: **P2**
- Audit findings: `SRC-07` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-154

## Objectives
1. Normalize artist/title (case, feat., remaster tags); group duplicates; prefer library > lossless > higher bitrate; user preference order.

## Acceptance Criteria (user-visible / measurable)
- Golden ranking tests on recorded multi-provider result sets.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/search_rank.rs`

## Verification Command
`cargo test -p amp-core-engine rank`

## Manual Verification
Golden cross-source ranking and deduplication tests in `amp-core-engine::search_rank` verifying that parenthetical/remaster/official video metadata is normalized, identical tracks across multiple providers (Local, Bandcamp, YouTube) are collapsed, and higher quality/local sources are prioritized. Tested via `cargo test -p amp-core-engine rank`.

## Status
- **Status:** Completed
