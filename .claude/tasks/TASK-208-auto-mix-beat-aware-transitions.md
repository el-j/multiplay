# TASK-208: Auto Mix: Automatic Beat-Aware Transitions Between Decks

## Plan Reference
- Plan: [PLAN-054](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-054-dj-engine-multi-deck-cue-and-beat-analysis.md)
- Design: [Signature features design §5](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `audio_engine`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-205, TASK-207, TASK-108

## Objectives
1. When on, transition before track end at a configurable point (beats, seconds, or detected outro).
2. Compatible BPMs: sync and beat-aligned transition; otherwise equal-power crossfade.
3. Then load the next assigned track for the freed deck; with none assigned, take the next unplayed playlist track; with nothing left, let the last track end.

## Acceptance Criteria (user-visible / measurable)
- Engine test: a 4-track assigned playlist auto-mixes end to end with no silence and no beat-phase error > 10 ms in synced transitions.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/automix.rs (new)`
- `crates/amp-core-engine/src/dsp/crossfade.rs`

## Verification Command
`cargo test -p amp-core-engine dj_automix`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.
- 2026-10-19: `dj::automix`, a control-side state machine (never on the audio thread) for decks A/B:
  - **Transition start.** Configurable: `BeatsBeforeEnd`, `SecondsBeforeEnd`, or `Outro` (the last point after which the detail waveform stays below 60 % of the median). On a grid it starts on a downbeat.
  - **Compatible tempos** (within 8 %, half or double allowed): the incoming deck syncs and starts on its first downbeat, so bars line up.
  - **Otherwise** an equal-power crossfade.
  - **After the mix** the new track keeps the tempo it came in at.
  - **Freed deck.** Loaded with the next track assigned to it, else the next unplayed track (`PlaylistSource` with `Assignment::Deck/Any`). With nothing left, the last track plays out and the state becomes `Finished`.
  - Tracks are levelled to a target loudness through auto-gain.
- Evidence (`cargo test -p amp-core-engine dj_automix`):
  - A four-track playlist (120/124/122/126 BPM, deck-assigned and Any) plays in order with 3 synced transitions. The longest quiet stretch is 0.011 s (only the very first block). Beat phase between the decks during transitions is 0.000 ms (limit 10 ms). The mix finished at 134.8 s.
  - With sync disabled in transitions (mutation) the phase check reports 241.89 ms, so the measurement is real.
  - 90 to 140 BPM uses an unsynced crossfade with no gap.
  - A playlist with nothing assigned to deck B falls back to the next unplayed track.
  - Outro detection is unit-tested.
- Open: UI controls and playlist wiring come with PLAN-055 (TASK-209/210).

## Status
- **Status:** Pending
