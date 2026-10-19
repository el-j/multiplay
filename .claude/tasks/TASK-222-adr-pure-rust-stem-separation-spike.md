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
`./scripts/verify_pure_rust.sh`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
