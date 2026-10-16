---
id: PLAN-020
title: Orchestration Audit Remediation & Completion Re-baseline
status: in_progress
role: architecture
tasks:
  - TASK-061
  - TASK-062
---

# PLAN-020: Orchestration Audit Remediation & Completion Re-baseline

## Objective
Reconcile the project’s orchestration metadata with the actual workspace reality. Replace aspirational completion claims with a verified, evidence-driven remediation path that closes the biggest gaps in implementation, verification, and documentation.

## Architecture Context
- Nexus policy requires plan-first work, tests-first implementation, and verification before completion.
- The current orchestration metadata overstates completion for the workspace state.
- The real project must be re-baselined around a minimal, verified runtime path rather than an inflated roadmap.

## Target Areas
- `.claude/orchestrator-index.md`: status and completion claims must be re-grounded in evidence.
- `.claude/tasks/`: new backlog items must reflect the actual repair sequence.
- `docs/orchestration-audit.md`: audit evidence and honest status statements.
- `docs/orchestration-remediation-plan.md`: remediation ordering and execution gates.
- Runtime and adapter crates: provider stubs, synthetic streams, and incomplete integration paths.
- Test and verification pipeline: workspace test execution, mutation coverage, and warning/type health.

## Tasks
1. [TASK-061](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-061-orchestration-audit-remediation.md): Re-baseline the orchestration status and close the completion claim gap
2. [TASK-062](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-062-domain-contract-alignment-and-error-standardization.md): Domain Contract Alignment & Error Standardization

## Definition of Done
- The orchestration metadata and task records match the actual verified project state.
- The highest-risk placeholder provider paths are either removed or explicitly quarantined as non-production.
- A minimal end-to-end playback path is proven by tests.
- Verification commands are rerun and recorded before any task is closed.
- No task claims completion without fresh evidence.
