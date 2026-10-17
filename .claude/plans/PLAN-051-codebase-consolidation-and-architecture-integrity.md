# PLAN-051: Codebase Consolidation & Architecture Integrity

## Plan Overview
- **ID:** `PLAN-051`
- **Title:** Codebase Consolidation & Architecture Integrity
- **Role:** `architecture`
- **Milestone:** M5 — Ship (parallelizable from M1)
- **Priority:** P2
- **Status:** Pending
- **Dependencies:** `PLAN-037`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Remove dead code, restore hexagonal boundaries and replace shared-mutex state with message passing.

## Objectives
1. **TASK-189** (P2): Wire or Delete Dead Engine Modules (with ADR)
2. **TASK-190** (P2): Restore Hexagonal Boundaries & Single Composition Root
3. **TASK-191** (P2): Replace Arc<Mutex> State Soup with Commands, Atomics & Triple Buffers

## Tasks
- [ ] [TASK-189: Wire or Delete Dead Engine Modules (with ADR)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-189-wire-or-delete-dead-engine-modules.md)
- [ ] [TASK-190: Restore Hexagonal Boundaries & Single Composition Root](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-190-restore-hexagonal-boundaries-single-composition-root.md)
- [ ] [TASK-191: Replace Arc<Mutex> State Soup with Commands, Atomics & Triple Buffers](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-191-message-passing-and-lock-free-parameters.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
