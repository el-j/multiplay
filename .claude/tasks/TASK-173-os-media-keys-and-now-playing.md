# TASK-173: OS Media Keys & Now Playing (MPRIS / macOS / SMTC)

## Plan Reference
- Plan: [PLAN-047](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-047-ux-first-run-to-daily-driver.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: `UX-05` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125

## Objectives
1. `MediaControlsPort` + `souvlaki` adapter: play/pause/next/prev/seek, metadata + artwork; works in GUI and TUI.

## Acceptance Criteria (user-visible / measurable)
- Hardware media keys and Control Center work on macOS; `playerctl` works on Linux; SMTC overlay on Windows.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-media-controls/`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
