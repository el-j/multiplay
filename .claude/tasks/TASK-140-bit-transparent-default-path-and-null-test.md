# TASK-140: Bit-Transparent Default Path & CI Null Test

## Plan Reference
- Plan: [PLAN-040](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-040-audiophile-dsp-transparency.md)
- Role: `dsp_engineering`
- Priority: **P0**
- Audit findings: `DSP-01`, `DSP-02` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Remove `soft_limit` from the EQ; bypass EQ when all gains+preamp are 0; bypass crossfeed when Off; limiter only when net gain > 0 dB.
2. `Bit-perfect` mode: unity volume, DSP off, device opened at source rate; UI badge `BIT-PERFECT` when the path is identity.
3. CI null test: decode fixture → chain (neutral) → compare == input (exact).

## Acceptance Criteria (user-visible / measurable)
- Null test passes; enabling a +3 dB band changes output; disabling returns to identity.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/dsp/equalizer.rs`
- `crates/amp-core-engine/src/dsp/chain.rs`

## Verification Command
`cargo test -p amp-core-engine null_test`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
