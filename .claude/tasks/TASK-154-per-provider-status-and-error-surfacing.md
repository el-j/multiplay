# TASK-154: Per-Provider Status, Timeouts & Error Surfacing

## Plan Reference
- Plan: [PLAN-042](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-042-universal-search-wired-end-to-end.md)
- Role: `integrations`
- Priority: **P1**
- Audit findings: `SRC-07` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-130

## Objectives
1. `FilteredSearchPort` returns a stream of `ProviderOutcome { kind, result: Result<Vec<Track>, ProviderError>, latency }`.
2. UI chips: ✓ results, ⏳ loading, 🔒 login needed, ⚠ rate-limited/offline.

## Acceptance Criteria (user-visible / measurable)
- A failing provider never hides other providers' results and is visibly flagged.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/filtered_search.rs`
- `crates/amp-ports/src/driving.rs`

## Verification Command
`cargo test -p amp-core-engine search`

## Manual Verification
Verified that `FilteredSearchCoordinator::search_outcomes` streams `ProviderOutcome` for each registered provider containing latency, error status, and UI status chips (`✓`, `⏳`, `🔒`, `⚠`, `∅`), and that failing/timed-out providers never conceal results from successful providers. Verified via `cargo test -p amp-core-engine search`.

## Status
- **Status:** Completed
