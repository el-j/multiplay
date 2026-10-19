# TASK-206: Loops & Cues: A-B Loop, Beat Loops, Loop Roll, Hot Cues

## Plan Reference
- Plan: [PLAN-054](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-054-dj-engine-multi-deck-cue-and-beat-analysis.md)
- Design: [Signature features design §5](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `audio_engine`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-202, TASK-204

## Objectives
1. A-B loop: one action sets A, the next sets B; optional quantize to beats; exit/reloop.
2. Beat loops 1/4 … 32 beats with halve/double; loop roll (playback continues in the background, resumes in phase).
3. 8 hot cues per track persisted in the library; sample-accurate, click-free loop seams.

## Acceptance Criteria (user-visible / measurable)
- Capture-sink tests: loop seam has no discontinuity > -60 dBFS step; loop length matches beats at the deck tempo; hot cues survive restart.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/loops.rs (new)`
- `crates/amp-adapter-storage/src/`

## Verification Command
`cargo test -p amp-core-engine dj_loops`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.
- 2026-10-19: loops and cues in the engine (`dj::engine::loops`, `deck_commands`):
  - Beat loops of 1/32 to 64 beats from the quantized playhead, with halve/double.
  - Loop roll: the track runs on underneath and playback resumes there.
  - A-B loop: the first press sets A, the second sets B; both snap to the grid with quantize, which is on by default.
  - Exit and reloop (jumps to the loop start); `loop_between` for explicit ranges.
  - 8 hot cues: mark (quantized), set/clear, and jump. With quantize the jump keeps the beat phase; jumping out of a loop exits it; a synced deck re-snaps.
  - All actions run on the render thread at the exact playhead.
  - Seams: a 5 ms raised-cosine blend into the audio leading up to the loop start, just before the end, so the first hit inside the loop keeps full strength.
  - Hot cues persist through the new `HotCueStorePort` (redb table `dj_hot_cues_v1`).
- Evidence (`cargo test -p amp-core-engine dj_loops`, on rendered audio):
  - 1- and 4-beat loops repeat every 0.5/1.05 s at +5 % (within 1 ms);
  - the seam's second difference stays below 0.001 (-60 dBFS) on a 100 Hz tone cut at mismatched phases. With the blend removed (mutation) it is 0.167;
  - A-B lands on 1.0-2.5 s from presses at 1.1 and 2.3 s;
  - halve, double, exit and reloop work;
  - roll resumes exactly 1.5 s later after 1 s of roll and 0.5 s of play;
  - a hot cue marked at 3.13 s is stored as 3.0 s, and jumping keeps the sub-beat offset.
  `hot_cues_survive_a_restart` (storage). Render stays allocation-free with loops, rolls and cues.
- Open: the UI and library wiring (save cues on change, load them on deck load) comes with PLAN-055.

## Status
- **Status:** Pending
