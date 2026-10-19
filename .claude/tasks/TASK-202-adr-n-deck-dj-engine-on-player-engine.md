# TASK-202: ADR & N-Deck DJ Engine on PlayerEngine (Per-Deck Decode, Resample, Stretch)

## Plan Reference
- Plan: [PLAN-054](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-054-dj-engine-multi-deck-cue-and-beat-analysis.md)
- Design: [Signature features design §5](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `audio_engine`
- Priority: **P0**
- Audit findings: `UX-08`, `ARC-07`, `DSP-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125, TASK-127, TASK-142

## Objectives
1. ADR: deck model (2 active, 4 supported), threads, command/parameter channels, buses, real-time rules; reconcile TASK-039/041/099/100/101/102 and supersede TASK-176.
2. Deck = async loader + own decoder + own resampler + own time-stretcher, fed through the PlayerEngine transport (no decoding on the audio thread).
3. Parameters (fader, EQ, pitch, crossfader) via lock-free atomics/triple buffers; no deck state cloning per tick (UX-08).
4. Hidden behind `experimental.dj_mode` until PLAN-055 acceptance.

## Acceptance Criteria (user-visible / measurable)
- Two decks at 44.1 and 48 kHz mix at correct pitch with zero underruns for 10 minutes (capture-sink test + soak).
- Audio callback verified allocation- and lock-free (test harness or `assert_no_alloc`-style guard in debug).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/`
- `crates/amp-ports/src/driving.rs`
- `docs/adr/`

## Verification Command
`cargo test -p amp-core-engine dj`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.
- 2026-10-19: ADR-003 (`docs/adr/ADR-003-dj-engine.md`) records the deck model (4 in the model, 2 shown), threads, the parameter/command/garbage channels, buses and real-time rules, and reconciles TASK-039/041/099/100/101/102 and TASK-176.
- 2026-10-19: engine in `amp_core_engine::dj::engine`:
  - `TrackBuffer`: decoded in chunks on a loader thread, readable lock-free while loading.
  - `PcmStreamPort` (new driven port), implemented by the Symphonia decoder.
  - Per-deck variable-rate Hermite playhead (rate conversion and tempo in one step).
  - Master and cue buses, equal-power crossfader, click-free gain ramps.
  - `DjRuntime`: render thread with a ~50 ms lead instead of the player's 1.5 s queue.
  - `experimental.dj_mode` config flag (default off, old files parse).
- Evidence: `cargo test -p amp-core-engine dj` covers:
  - 44.1 and 48 kHz decks at true pitch on a 48 kHz bus;
  - pitch changes tempo;
  - PFL is on cue only;
  - clean end of track;
  - 10 minutes of mixing (accelerated render) with zero underruns;
  - `render` makes zero allocations and frees, proven by a counting global allocator;
  - a deck plays while its track is still loading;
  - the runtime keeps a simulated real-time sound card fed for 2 s with no starvation.
  A mutation check (pitch ignored; old track freed on the render thread) made the matching tests fail.
- Open: the GUI still uses the old `DualDeckMixer` path, until PLAN-055 moves it onto this engine; the flag is not yet read by the UI. No 10-minute real-device soak yet; that needs the GUI wiring.
- 2026-10-19: CI flake on macOS (run for e7c6274): `dj_runtime_keeps_a_real_time_output_fed` starved. The cause was a wall-clock test sharing its binary with the CPU-heavy 10-minute mixing test, on a 3-core runner with a debug build. The runtime tests now have their own binary (`tests/dj_runtime.rs`), and that test's lead is 100 ms. Open for the GUI wiring (PLAN-055): the production render thread runs at normal priority with a ~50 ms lead, so a heavily loaded machine can still underrun. Measure there and consider a larger default lead or a higher-priority thread.

## Status
- **Status:** Pending
