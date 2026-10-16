---
id: TASK-075
title: Full Mutation Testing Execution & Mutation Gap Closure
planId: PLAN-024
status: completed
role: testing
verificationCommand: make mutation-test
---

# TASK-075: Full Mutation Testing Execution & Mutation Gap Closure

## Description
Execute and validate mutation testing across core business logic and DSP components (`docs/orchestration-audit.md` section 4 & 5):
1. Target Scope: Run `cargo mutants` on `amp-domain`, `amp-ports`, and `amp-core-engine` using `.cargo/mutants.toml`.
2. Mutant Gap Analysis: Review any caught mutants and missed mutants in:
   - DSP biquad filter calculations and soft-knee limiter functions
   - Shuffle algorithms and playlist queue transitions
   - Pre-caching buffer watermarks and boundary checks
   - Filtered search mask evaluations
3. Hardening Test Coverage: Add targeted test assertions to eliminate surviving mutants, reaching 100% mutant kill rate for covered components.
4. CI Gate Integration: Ensure mutation test execution runs cleanly and reports status without timing out.

## Verification
```bash
make mutation-test
```

## Acceptance Criteria
- Mutation testing runs to completion using `.cargo/mutants.toml`.
- All generated mutants in core engine algorithms are caught by the test suite.
- Comprehensive regression protection verified.
