# TASK-174: TUI as a Complete Player

## Plan Reference
- Plan: [PLAN-047](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-047-ux-first-run-to-daily-driver.md)
- Role: `frontend_tui`
- Priority: **P1**
- Audit findings: `UX-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-126, TASK-153

## Objectives
1. Add files/folders/URLs (path prompt with completion), seek (←/→), next/prev (b/z), EQ editing, volume/balance, repeat/shuffle, `?` help overlay, quit confirmation while playing.
2. Route all diagnostics to the log file (never stdout/stderr in raw mode).

## Acceptance Criteria (user-visible / measurable)
- Scripted TUI test (ratatui `TestBackend`) covers every keybinding.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-tui/src/`
- `crates/amp-app/src/`

## Verification Command
`cargo test -p amp-adapter-tui -p amp-app`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
