# PLAN-048: Observability, Diagnostics & Measured Resource Budgets

## Plan Overview
- **ID:** `PLAN-048`
- **Title:** Observability, Diagnostics & Measured Resource Budgets
- **Role:** `qa_and_hardening`
- **Milestone:** M4 — Feels great
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-041`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Make field issues debuggable and replace claimed performance budgets with measured ones.

## Objectives
1. **TASK-177** (P1): Structured Logging with `tracing` to Rolling Log Files
2. **TASK-178** (P1): Panic Hook, Crash Reports & 'Copy Diagnostics'
3. **TASK-179** (P2): Measured Resource Budgets: Benchmark Harness & CI Tracking

## Tasks
- [ ] [TASK-177: Structured Logging with `tracing` to Rolling Log Files](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-177-structured-logging-with-tracing.md)
- [ ] [TASK-178: Panic Hook, Crash Reports & 'Copy Diagnostics'](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-178-panic-hook-and-diagnostics-bundle.md)
- [ ] [TASK-179: Measured Resource Budgets: Benchmark Harness & CI Tracking](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-179-measured-resource-budgets-benchmark-harness.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
