# TASK-153: Async Omnibox Search in GUI & TUI with Streaming Results

## Plan Reference
- Plan: [PLAN-042](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-042-universal-search-wired-end-to-end.md)
- Role: `frontend_gui`
- Priority: **P0**
- Audit findings: `UX-01` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. GUI: search window/omnibox (Ctrl/Cmd+K, J) with provider pills, source badges, play/enqueue/download actions.
2. TUI: execute search on Enter with debounce; render results as they stream.
3. Cancel in-flight searches when the query changes.

## Acceptance Criteria (user-visible / measurable)
- Typing a query shows local results < 50 ms and network results as each provider returns.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-gui/src/search_window.rs`
- `crates/amp-adapter-tui/src/search_view.rs`
- `crates/amp-app/src/`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
