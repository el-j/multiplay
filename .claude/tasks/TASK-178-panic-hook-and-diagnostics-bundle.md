# TASK-178: Panic Hook, Crash Reports & 'Copy Diagnostics'

## Plan Reference
- Plan: [PLAN-048](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-048-observability-and-diagnostics.md)
- Role: `qa_and_hardening`
- Priority: **P1**
- Audit findings: `QA-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-177

## Objectives
1. Panic hook writes crash report (backtrace, versions, OS, audio device, redacted config) to the log dir and restores the terminal in TUI mode.
2. Settings → About → 'Copy diagnostics' zips logs + report.

## Acceptance Criteria (user-visible / measurable)
- Forced panic produces a report and a clean terminal.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-app/src/crash.rs`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
