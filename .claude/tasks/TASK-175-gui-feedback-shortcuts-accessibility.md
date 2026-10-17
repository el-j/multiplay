# TASK-175: GUI Feedback, Shortcuts, Seek-on-Release & Accessibility

## Plan Reference
- Plan: [PLAN-047](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-047-ux-first-run-to-daily-driver.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: `UX-07`, `ARC-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-130

## Objectives
1. Toasts/status bar for `PlayerEvent::Error` and buffering; seek on drag release only; 'Add URL' field.
2. Winamp-style shortcuts (Z X C V B, J, Ctrl+L) + platform shortcuts; font scaling, contrast theme, AccessKit labels.
3. Non-blocking file dialogs (spawn off UI thread or `rfd` with xdg-portal on Linux); cancelling never triggers a scan.

## Acceptance Criteria (user-visible / measurable)
- Keyboard-only usage possible; errors are visible in a .app build.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-gui/src/`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
