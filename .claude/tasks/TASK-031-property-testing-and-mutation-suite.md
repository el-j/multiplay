---
id: TASK-031
title: Comprehensive Property-Based Testing (proptest) & Mutation Test Suite
planId: PLAN-010
status: completed
role: qa_and_hardening
verificationCommand: cargo test --workspace
---

# TASK-031: Comprehensive Property-Based Testing (proptest) & Mutation Test Suite

## Description
Harden DSP algorithms and lock-free concurrency against regressions and edge cases:
1. Property testing with `proptest`:
   - Biquad filter numerical stability across extreme cutoffs ($0.1 \text{ Hz} \le f \le 0.499 \cdot f_s$) and Q factors ($0.1 \le Q \le 100$).
   - Polyphase Sinc resampler stability across dynamic sample rate ratios ($8 \text{ kHz}$ to $384 \text{ kHz}$).
   - `SharedPrecacheBuffer` concurrent read/write/seek invariants under random chunk sizes.
2. Mutation Testing setup:
   - Configure `cargo-mutants` configuration `.cargo/mutants.toml` targeting `amp-core-engine`, `amp-domain`, and `amp-ports`.

## Verification
```bash
cargo test --workspace
```
