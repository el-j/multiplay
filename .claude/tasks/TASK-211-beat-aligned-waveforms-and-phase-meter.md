# TASK-211: Stacked Beat-Aligned Waveforms, Phase Meter & Overviews

## Plan Reference
- Plan: [PLAN-055](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-055-dj-mode-experience-console-assignment-waveforms.md)
- Design: [Signature features design §6](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-204, TASK-209

## Objectives
1. Scrolling three-band detail waveforms for both decks stacked, beat markers aligned at a fixed playhead; phase meter.
2. Overview waveform per deck with playhead, hot cues and loop region; click to seek.
3. BPM, pitch %, key and time readouts; 60 fps without allocating per frame.

## Acceptance Criteria (user-visible / measurable)
- Waveform render reads the cached analysis (no recompute); frame time < 4 ms for both decks on CI-class hardware.
- Manual: two synced tracks show aligned beat markers.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/dj_waveform.rs (new)`

## Verification Command
`cargo test -p amp-adapter-gui waveform`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.
- 2026-10-19: `dj_waveform.rs` plus the console layout:
  - **Detail lanes:** stacked scrolling three-band waveforms (deck A above deck B) across the top of the console, with beat and downbeat markers and a fixed playhead.
  - **Shared time axis:** the lanes use heard time (track time divided by playing speed), so synced decks' beat markers coincide.
  - **Phase meter:** between the lanes, showing B against A within half a beat; green within 2 %.
  - **Overviews:** each deck strip has a whole-track overview with the played part dimmed, the playhead, coloured hot cues and the loop region (green when active). Clicking seeks.
  - **Readouts:** heard BPM, pitch % and time/total per deck. Everything is drawn from the cached `TrackAnalysis`; there is no recompute per frame.
- Evidence:
  - `dj_waveform_synced_decks_show_beats_at_the_same_pixels`: a 124 BPM deck slowed to 120 and in phase puts every beat within 0.5 px of the 120 BPM deck's;
  - playhead-centre and bar-spacing tests;
  - `tests/dj_waveform_perf.rs` (release): both lanes plus overviews at 1000 px take 0.14 ms per frame (budget 4 ms);
  - the console still fits from 520x232 up (responsive test).
- Open: musical key readout (key analysis is P2 in TASK-204). A manual look at two synced tracks in the running app.

## Status
- **Status:** Pending
