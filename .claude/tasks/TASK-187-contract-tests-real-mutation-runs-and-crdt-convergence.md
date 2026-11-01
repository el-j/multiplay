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

## Implementation Evidence (2026-11-01, branch feat/plan-049-release) — partial
- Contract kit: new dev-only crate `crates/amp-test-support` — `check_pcm_stream` (rate, whole stereo pairs into odd buffers, finite samples, sticky EOF, monotonic elapsed matching frames, duration within 2%, seek accuracy, replay after seeking to 0) and `check_library_storage` (store, replace, search by title/artist, remove).
- Applied: 17 fixtures through `SymphoniaDecoderFactory::open_file` and 5 through `open_stream` (incl. our Opus, DSD, WavPack) in `amp-adapter-decoder/tests/contract.rs`; redb in `amp-adapter-storage/tests/contract.rs`.
- Bugs the contract found and fixed: the Symphonia decoder split stereo pairs into odd buffers (channels would swap from then on); AAC/M4A and MKA could not play again after reaching the end and seeking back (Symphonia 0.6.1 reader states) — the decoder now reopens the stream from its start (`reopen_at_start`) when a reader refuses or fails after a seek.
- Mutation testing: `.github/workflows/mutants.yml` (weekly, domain + DSP + library index + update check). Score not recorded yet (first run pending).
- Not done: the CRDT fixes — the jam CRDT is unused; fixing or deleting it waits on the TASK-167 decision.
