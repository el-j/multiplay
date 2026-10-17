# TASK-171: First-Run Onboarding Wizard

## Plan Reference
- Plan: [PLAN-047](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-047-ux-first-run-to-daily-driver.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: `UX-03` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-149, TASK-150

## Objectives
1. Steps: pick music folders (background scan w/ progress), choose output device, opt into providers with honest notes, remote control off by default.
2. Re-runnable from settings.

## Acceptance Criteria (user-visible / measurable)
- Fresh-profile usability test: first track playing < 60 s.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-gui/src/onboarding.rs`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
