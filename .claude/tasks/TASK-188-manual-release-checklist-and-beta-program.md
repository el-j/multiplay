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

## Implementation Evidence (2026-11-01, branch feat/plan-050-qa)
- `docs/release-checklist.md`: install (checksums, per-OS, `--version`, icon), first run (setup, first track < 60 s, restart), playing (gapless album, seek, formats + BIT-PERFECT, keyboard, media keys, 30-min radio), library, online sources, device unplug, sleep/wake, remote pairing/requests, diagnostics, uninstall, and a per-OS record table.
- `.github/ISSUE_TEMPLATE/`: bug report and online-source breakage forms (both ask for Copy diagnostics), security contact link. `docs/RELEASING.md` links the checklist and the CHANGELOG step.
- Open: running two RCs through it (needs tagged releases).
