# TASK-112: Reconcile Orchestrator State with Verified Reality

## Plan Reference
- Plan: [PLAN-034](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-034-release-truth-reset-and-governance.md)
- Role: `architecture`
- Priority: **P0**
- Audit findings: `ORC-01`, `ORC-02`, `ORC-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Using the Nexus tools (never by hand), re-open every task whose user-visible outcome is not demonstrable: TASK-015, 019, 023, 024, 030, 035, 036–038, 044, 064, 065, 109 and any other task listed in `.claude/audits/AUDIT-05` / `AUDIT-01`.
2. Annotate each re-opened task with the audit finding ID that disproves completion.
3. Make plan/task counts derived (discover mode) and fix the `orchestrator-index.md` header and `orchestrator.json` metrics to match files on disk (33 → 52 plans, 111 → 194 tasks).
4. Mark DJ (PLAN-013/031) and mobile jam (PLAN-012/032) features as `Experimental` and freeze new feature plans until M2 completes.

## Acceptance Criteria (user-visible / measurable)
- Index header, JSON metrics and plan files agree.
- No task is `Completed` whose acceptance cannot be demonstrated on a release build.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `.claude/orchestrator-index.md`
- `.claude/orchestrator.json`
- `.claude/plans/*`
- `.claude/tasks/*`

## Verification Command
`Manual review against `.claude/audits/README.md` scorecard`

## Manual Verification
Reconciliation executed against AUDIT-2026-10-17:
- All 14 mock/unwired tasks (TASK-015, 019, 023, 024, 030, 035, 036, 037, 038, 044, 063, 064, 065, 109) re-opened with explicit audit findings and destination plan/task mappings.
- DJ (PLAN-013, PLAN-031) and Mobile Jam (PLAN-012, PLAN-032) marked Experimental.
- Orchestrator index and orchestrator.json re-baselined to derived disk metrics: 52 plans, 194 tasks, 80 verified completed tasks.
- Active plan set to PLAN-034.

## Status
- **Status:** Completed
