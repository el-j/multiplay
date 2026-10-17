# TASK-183: Opt-In Update Checker & Provider Hotfix Channel

## Plan Reference
- Plan: [PLAN-049](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-049-release-engineering-and-distribution.md)
- Role: `devops`
- Priority: **P3**
- Audit findings: `SRC-09` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-181

## Objectives
1. Opt-in check against GitHub Releases API (no telemetry); notify about provider-breakage hotfix releases.

## Acceptance Criteria (user-visible / measurable)
- Disabled by default; one HTTPS request per day max when enabled.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-app/src/update_check.rs`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
