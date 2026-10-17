# TASK-188: Manual Release Checklist & Beta Program

## Plan Reference
- Plan: [PLAN-050](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-050-release-grade-quality-assurance.md)
- Role: `qa_and_hardening`
- Priority: **P1**
- Audit findings: `ORC-03`, `LEG-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-181

## Objectives
1. `docs/release-checklist.md`: per-OS install, first run, each provider, gapless album, radio 30 min, device unplug, sleep/wake, remote pairing, uninstall.
2. Public beta (GitHub pre-releases + issue templates for bug/provider breakage).

## Acceptance Criteria (user-visible / measurable)
- Two consecutive RCs pass the checklist on all 3 OSes before v1.0.0.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `docs/release-checklist.md`
- `.github/ISSUE_TEMPLATE/`

## Verification Command
`Manual`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
