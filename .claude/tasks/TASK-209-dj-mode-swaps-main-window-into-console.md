# TASK-209: DJ Mode Toggle: Main Window Becomes the Deck Console Without a Gap

## Plan Reference
- Plan: [PLAN-055](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-055-dj-mode-experience-console-assignment-waveforms.md)
- Design: [Signature features design §6](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P0**
- Audit findings: `UX-08` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-202, TASK-195

## Objectives
1. DJ button / Ctrl+J swaps the main window into the 550x232 console (two deck strips + mixer column); it keeps anchoring the dock group.
2. The playing track continues seamlessly as Deck A; leaving DJ mode keeps the on-air deck playing as normal playback.
3. Replace `dj_panel.rs` / `dj_playback.rs` / `dj_audio.rs` with views over the engine.

## Acceptance Criteria (user-visible / measurable)
- Toggling DJ mode during playback produces no audible gap (capture-sink test across the toggle).
- Docked EQ/playlist reflow to the console width and back.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/dj_panel.rs`
- `crates/amp-adapter-gui/src/main_window.rs`

## Verification Command
`cargo test -p amp-adapter-gui dj_mode`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.
- 2026-10-19: DJ mode now runs the new engine inside the existing audio worker (`dj_host.rs`), which owns the sound card queue, and is driven from an egui-free `DjSession` (`dj_session.rs`) via `DjLink`.
  - **Entering** (DJ button or Ctrl/Cmd+J): the playing track starts decoding as deck A. Normal playback continues until deck A is decoded 1 s past the playing position. Deck A is then primed (so the limiter lookahead and fade-in have settled), compensated for the normal path's resampler delay, and crossfaded in over 20 ms.
  - **Leaving:** the normal decoder opens on a helper thread, seeks to the on-air deck's position plus the resampler delay, and crossfades back over 20 ms.
  - The main window becomes the DJ console (`dj_console.rs`, 550x232, minimum 520x232) and back; docked windows follow via `resize_window`, and resize sessions honour the console minimum (`app.min_size`).
  - `dj_audio.rs`, `dj_panel.rs` and `dj_playback.rs`, the DualDeckMixer path, and the separate DJ satellite window are removed.
- Root causes found on the way, each now fixed:
  - **Engine start:** the engine's first 64 frames are the limiter lookahead, which gave a 1.5 ms gap at the splice. Fixed by priming.
  - **Decoder seeks:** seeks were Coarse (they landed on the packet before the target while reporting the target). Seeks are now sample-accurate (`SeekMode::Accurate` plus dropping the lead-in frames, rounding the target). New test `seek_accuracy.rs`; this improves every seek in the player.
  - **Isolator EQ phase:** the isolator EQ's Linkwitz-Riley all-pass shifts phase by frequency (about 2 ms at 37 Hz, about 180 degrees at the 250 Hz crossover), so no splice could line up. A flat channel strip is now bypassed bit-exactly and blends in when touched (test `dj_mixer_flat_strip_passes_audio_through_untouched`).
  - **Measured alignment:** with the above, a 48 kHz splice is off by -1.4 samples and a 44.1 kHz one by 5.6 samples after resampler-delay compensation. The crossfade absorbs the rest.
- Evidence: `tests/dj_toggle.rs`. A 440 Hz WAV plays through a capture sink while DJ mode is entered mid-track and left again. The recorded audio has no near-silent run longer than 3 samples, no curvature step at or above 0.01 (no click), 440 Hz ±20 in every 100 ms window, and level within ±12 %. It passed 3 runs in a row. `responsive_dj_console_fits_from_its_minimum_up` checks the console at 520x232 up to 1000x500.
- Not yet verified by hand: the console's look and feel in the running app. The computer-use helper has no Accessibility or Screen Recording permission, so I did not drive the GUI.

## Status
- **Status:** Pending
