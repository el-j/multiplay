# TASK-180: Single `multiplay` Binary, Versioning & CHANGELOG

## Plan Reference
- Plan: [PLAN-049](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-049-release-engineering-and-distribution.md)
- Role: `devops`
- Priority: **P2**
- Audit findings: `BLD-07`, `ARC-08` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-126

## Objectives
1. One `multiplay` binary in `amp-app` (`--tui`, `--headless --remote`), GUI default; move GUI main out of the adapter crate.
2. SemVer from workspace version; `--version` shows git hash; CHANGELOG via conventional commits (git-cliff / release-plz).

## Acceptance Criteria (user-visible / measurable)
- `multiplay --version` and `multiplay --tui` work from release artifacts.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-app/src/main.rs`
- `CHANGELOG.md`
- `cliff.toml`

## Verification Command
`cargo run -p amp-app -- --version`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
