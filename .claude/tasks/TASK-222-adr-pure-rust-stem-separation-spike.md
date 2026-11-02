# TASK-222: ADR Spike: Pure-Rust Stem Separation (Go/No-Go)

## Plan Reference
- Plan: [PLAN-057](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-057-remix-decks-stems-and-mix-recording.md)
- Design: [Signature features design §8](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `dsp_engineering`
- Priority: **P2**
- Audit findings: —
- Depends on: TASK-202

## Objectives
1. Evaluate candle / tract / burn with an openly licensed 4-stem model: licence, download size, CPU time per track, quality on the corpus.
2. Decide: ship real stems, ship band-split pseudo-stems on the kill EQ, or drop.

## Acceptance Criteria (user-visible / measurable)
- ADR merged with measurements and the decision; pure-Rust verifier passes for the chosen stack.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `docs/adr/`

## Verification Command
`./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test`

## Manual Verification
Evaluated candle, tract, and burn runtimes against Demucs v4 architecture. Documented results in `docs/adr/ADR-010-stem-separation-spike.md`.
Decision reached:
- NO-GO on bundled deep learning neural network inference models in v1 (prevents 150+ heavy dependencies, 100MB+ models, and 45s-180s CPU latency during live DJ mixing).
- GO on Linkwitz-Riley 4th-order (LR4) phase-aligned 4-band pseudo-stems (Drums, Bass, Melody, Vocals) providing zero latency, allocation-free real-time rendering, and flat magnitude reconstruction at unity gain.
Verification script `./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test` executed successfully with 0 errors.

## Status
- **Status:** Completed
