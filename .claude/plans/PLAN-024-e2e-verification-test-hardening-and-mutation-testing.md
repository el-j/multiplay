---
id: PLAN-024
title: End-to-End Verification, Test Suite Hardening & CI Test Resilience
status: in_progress
role: testing
tasks:
  - TASK-073
  - TASK-074
  - TASK-075
---

# PLAN-024: End-to-End Verification, Test Suite Hardening & CI Test Resilience

## Objective
Establish uncompromised test reliability, eliminate test process OOM crashes (exit code 137), build multi-format E2E audio pipeline tests, and prove algorithmic correctness via comprehensive mutation testing.

## Architecture Context
- **Audit Grounding (`docs/orchestration-audit.md`)**:
  - Section 1 & 4: `cargo test --workspace --quiet` crashed with exit code 137 due to unbounded concurrency and memory pressure.
  - Section 4 & 9: Real E2E tests covering the complete decode -> DSP -> output pipeline were missing.
  - Section 4 & 5: Mutation testing was configured in `.cargo/mutants.toml` but had no executed validation.
- **Remediation Strategy (`docs/orchestration-remediation-plan.md` Phase 4 & 5)**:
  - Build memory-conscious runner script `scripts/run_all_tests.sh`.
  - Implement genuine multi-format audio playback integration tests with `CaptureAudioSink`.
  - Execute `cargo mutants` and close all surviving mutation gaps.

## Tasks
1. [TASK-073](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-073-multi-format-e2e-audio-pipeline-test-harness.md): Multi-Format E2E Audio Pipeline Test Harness
2. [TASK-074](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-074-memory-efficient-test-harness-and-ci-resilience.md): Memory-Efficient Test Execution Harness & CI Resilience
3. [TASK-075](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-075-full-mutation-testing-pass-with-cargo-mutants.md): Full Mutation Testing Execution & Mutation Gap Closure

## Definition of Done
- Complete test suite passes with exit code 0 on all development machines and CI runners.
- End-to-end tests prove authentic audio playback across FLAC, MP3, WAV, OGG, AAC.
- Mutation testing catches 100% of mutations in core DSP and domain algorithms.
