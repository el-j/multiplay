# TASK-187: Port Contract Tests, Real Mutation Runs & CRDT Convergence Fixes

## Plan Reference
- Plan: [PLAN-050](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-050-release-grade-quality-assurance.md)
- Role: `qa_and_hardening`
- Priority: **P2**
- Audit findings: `QA-03`, `ARC-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125

## Objectives
1. Port contract test kits every adapter/fake must pass.
2. Run `cargo mutants` on domain/dsp/player in a weekly job; record score; set threshold.
3. Fix CRDT: per-peer upvote sets, causal buffering of orphan inserts; proptest convergence over permutations/duplicates (or delete if TASK-167 cuts jam).

## Acceptance Criteria (user-visible / measurable)
- Mutation score recorded in docs and CI; convergence proptest passes 10k cases.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-test-support/`
- `.github/workflows/mutants.yml`
- `crates/amp-domain/src/jam_crdt.rs`

## Verification Command
`cargo mutants --in-place -p amp-core-engine --file 'src/dsp/**'`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
