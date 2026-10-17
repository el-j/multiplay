# PLAN-036: Green CI on macOS/Linux/Windows & Hermetic Test Suite

## Plan Overview
- **ID:** `PLAN-036`
- **Title:** Green CI on macOS/Linux/Windows & Hermetic Test Suite
- **Role:** `devops, qa_and_hardening`
- **Milestone:** M0 — Stop the bleeding
- **Priority:** P0
- **Status:** Pending
- **Dependencies:** `PLAN-034`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Make CI a trustworthy gate: formatted, building and testing on all three OSes without audio hardware, network, or touching the developer's home directory.

## Objectives
1. **TASK-120** (P0): Fix rustfmt Drift & Install Linux System Dependencies in CI
2. **TASK-121** (P0): Hardware-Free Test Mode with Null/Capture Audio Sink
3. **TASK-122** (P1): Hermetic Tests: No $HOME Writes, No Network, No 0.0.0.0 Binds
4. **TASK-123** (P0): Repair the Release Workflow (Missing Files, Silent Failures)
5. **TASK-124** (P2): Supply Chain: cargo-deny, cargo-audit, Dependabot & Workspace Pinning

## Tasks
- [ ] [TASK-120: Fix rustfmt Drift & Install Linux System Dependencies in CI](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-120-fix-fmt-and-ci-system-dependencies.md)
- [ ] [TASK-121: Hardware-Free Test Mode with Null/Capture Audio Sink](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-121-hardware-free-null-audio-sink-test-mode.md)
- [ ] [TASK-122: Hermetic Tests: No $HOME Writes, No Network, No 0.0.0.0 Binds](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-122-hermetic-tests-no-home-no-network-no-public-bind.md)
- [ ] [TASK-123: Repair the Release Workflow (Missing Files, Silent Failures)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-123-repair-release-workflow.md)
- [ ] [TASK-124: Supply Chain: cargo-deny, cargo-audit, Dependabot & Workspace Pinning](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-124-supply-chain-cargo-deny-audit-and-dependency-hygiene.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
