# TASK-166: Soulseek UI: Account Settings, Transfer Manager, User Browse

## Plan Reference
- Plan: [PLAN-045](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-045-real-soulseek-client.md)
- Role: `frontend_gui`
- Priority: **P2**
- Audit findings: `SRC-16`, `UX-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-164, TASK-165

## Objectives
1. Account login in settings (vault-backed); transfers window (downloads/uploads, queue position, speed, cancel/retry); browse a user's shares from a search result.

## Acceptance Criteria (user-visible / measurable)
- Manual acceptance on GUI; TUI gets a transfers panel.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-gui/src/soulseek_window.rs`
- `crates/amp-adapter-tui/src/`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
