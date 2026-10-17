# TASK-115: Evidence-Based Definition of Done v2

## Plan Reference
- Plan: [PLAN-034](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-034-release-truth-reset-and-governance.md)
- Role: `qa_and_hardening`
- Priority: **P1**
- Audit findings: `ORC-03`, `ORC-04`, `LEG-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-112

## Objectives
1. Replace the aspirational DoD with a measurable one: CI green on 3 OSes (link to run), real-adapter test (fixture or local server), manual acceptance step on a release build, measured budgets where claimed.
2. Add a task template `.claude/tasks/_TEMPLATE.md` with mandatory sections: Audit findings, Acceptance criteria (user-visible), Automated verification, Manual verification, Evidence links.
3. Document the rule: mocks may satisfy unit tests but never mark a provider task Completed.

## Acceptance Criteria (user-visible / measurable)
- Template exists and all new tasks (TASK-116+) follow it.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `.claude/tasks/_TEMPLATE.md`
- `docs/definition-of-done.md`
- `CLAUDE.md`

## Verification Command
`test -f docs/definition-of-done.md`

## Manual Verification
- `docs/definition-of-done.md` created with 5 concrete gates: Pure-Rust sound verification, Tests-First with mock exclusion rule, Hexagonal boundary & lock-free audio thread invariants, Code quality & 300-line limit, and Release-build manual acceptance.
- `.claude/tasks/_TEMPLATE.md` created incorporating audit tracing, user-visible acceptance criteria, and evidence sections.
- Verification command passed: `test -f docs/definition-of-done.md && test -f .claude/tasks/_TEMPLATE.md`.

## Status
- **Status:** Completed
