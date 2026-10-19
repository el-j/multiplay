# TASK-212: DJ Controls, Keyboard Map, Cheat Sheet & First-Run Hint

## Plan Reference
- Plan: [PLAN-055](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-055-dj-mode-experience-console-assignment-waveforms.md)
- Design: [Signature features design §6](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-203, TASK-205, TASK-206, TASK-207, TASK-209

## Objectives
1. Per deck: play/pause, cue, sync, PFL, loop A/B, beat-loop size, hot cues 1–8, pitch fader, nudge; crossfader; cue mix/volume.
2. Full keyboard map registered via the TASK-175 shortcut map; cheat-sheet overlay (?).
3. First-time three-step hint (assign → cue → fade); defaults: sync on, quantize on, auto mix off; no single control can stop the on-air deck by accident.

## Acceptance Criteria (user-visible / measurable)
- Every control reachable by mouse and keyboard (test over the shortcut table).
- Usability check with two first-time users recorded in the evidence.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/dj_controls.rs (new)`

## Verification Command
`cargo test -p amp-adapter-gui dj_controls`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.
- 2026-10-19: `dj_controls.rs` (actions, keymap, rules), `dj_strip.rs` (deck strips and mixer column) and `dj_console.rs` (layout, keys, overlays):
  - **Per deck:** play/pause, CDJ-style CUE (set when paused, hold to preview, release to return), SYNC, PFL, A-B loop, beat loop with a ½/×2 size stepper (1/4 to 32), exit loop, hot cues 1-8 (right-click clears), a pitch fader with ±0.1 % key steps (Shift ±1 %), and hold-to-nudge.
  - **Mixer:** channel faders, a crossfader with centre, cue mix and cue volume, and headphone routing (off / split L-R / second device). AUTO and `?` cheat sheet buttons.
  - **Headphone device:** "DJ headphones (cue)" in Settings > Audio opens a second device. One at a different sample rate is refused with an explanation, because the cue would play at the wrong pitch.
  - **Keyboard map:** deck A on the left hand (A S D F / E R Q W T / Z X C V), deck B on the right (J K L H / U I Y O P / N M , .). Hot cues on 1-8 (Shift for B, Alt clears), arrows for the crossfader, - and = for cue mix (Shift for cue volume), B for auto mix, ? for the cheat sheet. Keys are ignored while a text field has focus, and Ctrl/Cmd shortcuts and the playlist's [ ] \ stay free.
  - **First-run hint:** three steps (assign, cue, fade), dismissed once and saved with the session.
  - **Defaults:** sync on, quantize on, auto mix off.
  - **Safety:** pausing the deck on air needs a second press within 1 s (with a message), and CUE is ignored on the deck on air.
- Evidence:
  - `dj_every_control_has_a_key`, `dj_no_key_does_two_things` and `dj_keys_leave_app_shortcuts_alone`;
  - `responsive_every_dj_control_is_on_screen`: every action is drawn as a control at the console's minimum size;
  - `tests/dj_controls.rs`: the on-air deck needs two presses, a late second press starts over, an off-air deck pauses at once, CUE never stops the deck on air, nudge and cue preview end on release;
  - the console fits at 520x232. A probe confirmed the check catches overflow: 33 px at 180 px tall;
  - `dj_headphones_must_match_the_main_rate`.
- Open: the usability check with two first-time users needs the owner. I could not drive the GUI (no Accessibility or Screen Recording permission for the computer-use helper).

## Status
- **Status:** Pending
