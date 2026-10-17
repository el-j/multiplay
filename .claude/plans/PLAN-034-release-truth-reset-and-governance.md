# PLAN-034: Release Truth Reset, Honest Status & Governance

## Plan Overview
- **ID:** `PLAN-034`
- **Title:** Release Truth Reset, Honest Status & Governance
- **Role:** `architecture, qa_and_hardening`
- **Milestone:** M0 — Stop the bleeding
- **Priority:** P0
- **Status:** Pending
- **Dependencies:** —
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Re-baseline the orchestration system on verified reality, fix the unsound pure-Rust policy/verifier, and publish the minimum product docs (README, LICENSE, honest feature matrix).

## Objectives
1. **TASK-112** (P0): Reconcile Orchestrator State with Verified Reality
2. **TASK-113** (P1): Redefine the Pure-Rust Policy & Rewrite a Sound Verifier
3. **TASK-114** (P0): README, LICENSE, CONTRIBUTING & Honest Feature Matrix
4. **TASK-115** (P1): Evidence-Based Definition of Done v2

## Tasks
- [ ] [TASK-112: Reconcile Orchestrator State with Verified Reality](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-112-reconcile-orchestrator-state-with-verified-reality.md)
- [ ] [TASK-113: Redefine the Pure-Rust Policy & Rewrite a Sound Verifier](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-113-redefine-pure-rust-policy-and-sound-verifier.md)
- [ ] [TASK-114: README, LICENSE, CONTRIBUTING & Honest Feature Matrix](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-114-readme-license-and-honest-feature-matrix.md)
- [ ] [TASK-115: Evidence-Based Definition of Done v2](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-115-evidence-based-definition-of-done.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
