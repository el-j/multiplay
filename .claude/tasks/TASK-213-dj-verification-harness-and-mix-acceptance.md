# TASK-213: DJ Verification Harness & Live-Mix Acceptance

## Plan Reference
- Plan: [PLAN-055](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-055-dj-mode-experience-console-assignment-waveforms.md)
- Design: [Signature features design §6](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `qa_and_hardening`
- Priority: **P0**
- Audit findings: —
- Depends on: TASK-208, TASK-210, TASK-211, TASK-212

## Objectives
1. End-to-end tests: sync + crossfade of 120/124 BPM fixtures, cue routing to capture sinks, auto mix through an assigned playlist, DJ toggle mid-track.
2. 10-minute soak with continuous parameter changes: zero underruns, no allocations in the callback.
3. Remove `experimental.dj_mode` gate when all pass; update README feature matrix.

## Acceptance Criteria (user-visible / measurable)
- All DJ tests green on macOS, Linux and Windows CI.
- Release-build manual mix (two decks, prelisten, loop, auto mix) recorded as evidence.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/tests/dj_*.rs`
- `crates/amp-app/tests/dj_engine_pipeline.rs`
- `README.md`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace dj`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.
- 2026-10-19: DJ verification suite:
  - **Engine:** `dj_engine`, `dj_runtime`, `dj_sync` (120/124 BPM beats within 5 ms over 5 minutes, key lock, seek and steering), `dj_loops`, `dj_mixer`, `dj_analysis` and `dj_automix` (4-track assigned playlist, take-over, late analysis).
  - **App path:** `dj_toggle` (DJ mode on and off mid-track: no gap, no click, pitch and level kept), `dj_routing` (split cue measured on the captured sound card: left 300 Hz master, right 900 Hz prelisten), `dj_playlist`, `dj_controls`, and the responsive console and control coverage tests.
- 10-minute soak (`dj_soak.rs`): two decks at 11.025 and 8 kHz with every control changing constantly (tempo, faders, crossfader, EQ, kills, filter, trim, master EQ, key lock, loops and rolls, hot cue marks and jumps). Zero underruns, zero allocations across all 56,250 render calls, master never above 0.98, all samples finite. 5 s in release; it runs in a new CI step together with the other release-only budgets (`dj_analysis`, `dj_mixer`, `dj_waveform_perf`).
- `experimental.dj_mode` removed (it was never read by the UI); old config files with the section still load (test). README feature matrix: DJ mode is **Beta**, with what it does and the open item. ADR-003 §9 updated.
- Open (owner): the release-build manual mix (two decks, prelisten, loop, auto mix) for the evidence, and the first-time-user check from TASK-212. DJ mode moves to Stable after that.

## Status
- **Status:** Pending
