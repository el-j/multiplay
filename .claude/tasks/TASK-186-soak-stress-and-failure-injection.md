# TASK-186: Soak, Stress & Failure-Injection Tests

## Plan Reference
- Plan: [PLAN-050](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-050-release-grade-quality-assurance.md)
- Role: `qa_and_hardening`
- Priority: **P2**
- Audit findings: `QA-08`, `AUD-10` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-136, TASK-133

## Objectives
1. 8 h headless soak (memory growth < 5 MB/h), 1,000 rapid skips, network drop/slow server (1 KB/s), resolver timeouts, disk-full during download, device loss.

## Acceptance Criteria (user-visible / measurable)
- Weekly scheduled workflow green; results in `docs/performance.md`.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/tests/soak.rs`
- `.github/workflows/soak.yml`

## Verification Command
`cargo test --release -p amp-core-engine --test soak -- --ignored`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
