---
id: TASK-074
title: Memory-Efficient Test Execution Harness & CI Resilience
planId: PLAN-024
status: completed
role: testing
verificationCommand: ./scripts/run_all_tests.sh
---

# TASK-074: Memory-Efficient Test Execution Harness & CI Resilience

## Description
Eliminate the test suite process crash (OS exit code 137 / SIGKILL) identified in `docs/orchestration-audit.md` section 1 & 4:
1. Root Cause Resolution: Running `cargo test --workspace` concurrently spawns dozens of compilation and test binaries that exhaust RAM/swap on development machines and CI runners.
2. Controlled Runner Script: Create `scripts/run_all_tests.sh` that executes tests crate-by-crate or with bounded parallelism (`-- --test-threads=2`) and low memory footprints.
3. Makefile Integration: Update `Makefile` target `test` to invoke `scripts/run_all_tests.sh`, guaranteeing zero crashes and clear output per crate.
4. Clean Workspace Pass: Validate that every unit test, integration test, and doc test across all 12 workspace crates passes cleanly.

## Verification
```bash
./scripts/run_all_tests.sh
```

## Acceptance Criteria
- Full workspace test pass completes with exit code 0 without being killed by the OS.
- Clear per-crate test summary output with test counts and durations.
- 100% reproducible locally and in GitHub Actions CI.
