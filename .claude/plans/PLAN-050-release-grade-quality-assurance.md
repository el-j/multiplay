# PLAN-050: Release-Grade Quality Assurance

## Plan Overview
- **ID:** `PLAN-050`
- **Title:** Release-Grade Quality Assurance
- **Role:** `qa_and_hardening`
- **Milestone:** M5 — Ship
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-037`, `PLAN-039`, `PLAN-040`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Tests that prove the product works on real audio, under stress, against hostile input — plus a human release checklist.

## Objectives
1. **TASK-184** (P1): Real-Audio Fixture Corpus & Golden Decode Tests
2. **TASK-185** (P2): Fuzz All Untrusted-Input Parsers & Add Size Limits
3. **TASK-186** (P2): Soak, Stress & Failure-Injection Tests
4. **TASK-187** (P2): Port Contract Tests, Real Mutation Runs & CRDT Convergence Fixes
5. **TASK-188** (P1): Manual Release Checklist & Beta Program

## Tasks
- [ ] [TASK-184: Real-Audio Fixture Corpus & Golden Decode Tests](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-184-real-audio-fixture-corpus-and-golden-tests.md)
- [ ] [TASK-185: Fuzz All Untrusted-Input Parsers & Add Size Limits](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-185-fuzz-untrusted-parsers.md)
- [ ] [TASK-186: Soak, Stress & Failure-Injection Tests](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-186-soak-stress-and-failure-injection.md)
- [ ] [TASK-187: Port Contract Tests, Real Mutation Runs & CRDT Convergence Fixes](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-187-contract-tests-real-mutation-runs-and-crdt-convergence.md)
- [ ] [TASK-188: Manual Release Checklist & Beta Program](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-188-manual-release-checklist-and-beta-program.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
