# TASK-179: Measured Resource Budgets: Benchmark Harness & CI Tracking

## Plan Reference
- Plan: [PLAN-048](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-048-observability-and-diagnostics.md)
- Role: `qa_and_hardening`
- Priority: **P2**
- Audit findings: `QA-05`, `AUD-14`, `ORC-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125

## Objectives
1. Headless benchmark: engine + null sink playing FLAC/MP3/radio; record CPU% and RSS idle/active; GUI idle/playing measured manually per release.
2. Replace policy numbers with measured, per-front-end budgets; CI fails on > 20% regression.

## Acceptance Criteria (user-visible / measurable)
- Budgets in `.nexus/policy.json` match measurements in `docs/performance.md`.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `benches/`
- `docs/performance.md`
- `.nexus/policy.json`

## Verification Command
`cargo bench -p amp-core-engine`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
