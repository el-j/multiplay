# TASK-114: README, LICENSE, CONTRIBUTING & Honest Feature Matrix

## Plan Reference
- Plan: [PLAN-034](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-034-release-truth-reset-and-governance.md)
- Role: `architecture`
- Priority: **P0**
- Audit findings: `LEG-01`, `LEG-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Decide the project license (recommend GPL-3.0-or-later or MPL-2.0) and add `LICENSE` plus `license` fields in every `Cargo.toml`.
2. Write `README.md`: what multiplay is, screenshots, install, build, a feature matrix with Stable / Experimental / Planned per source and feature, and known limitations.
3. Add `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md` (private disclosure address).

## Acceptance Criteria (user-visible / measurable)
- Every public claim in README is backed by a test or measurement, or labelled Planned.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `README.md`
- `LICENSE`
- `CONTRIBUTING.md`
- `SECURITY.md`
- `crates/*/Cargo.toml`

## Verification Command
`test -f README.md && test -f LICENSE && cargo metadata --format-version 1 | grep -q '"license"'`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
