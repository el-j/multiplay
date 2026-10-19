# ADR-003: N-Deck DJ Engine with In-Memory Decks and a Low-Latency Render Thread

- **Status:** Accepted (TASK-202)
- **Date:** 2026-10-19
- **Design:** [Signature features design §5](../superpowers/specs/2026-10-19-signature-features-design.md)
- **Supersedes:** the `DualDeckMixer` path (TASK-099/100/101) and TASK-176

## Context

The first DJ path (`dj::mixer::DualDeckMixer`, driven from the GUI's playback worker) decoded
two streams into the normal player's output queue. That design cannot carry real DJ features:

- The pitch slider did not change playback speed unless key lock was on; it only fed WSOLA.
- Decks had no playhead over their track, so seek, loops, hot cues and phase sync had nothing to
  act on, and 44.1 kHz tracks played at the wrong speed on a 48 kHz device.
- Controls travelled through a `Mutex` that was cloned every tick (UX-08).
- The output queue holds up to 65,536 frames (about 1.5 s), so faders and cue reacted late.

## Decision

1. **Engine in the core, adapters through ports.** `amp_core_engine::dj::engine` holds the
   engine. Tracks arrive through the new driven port `amp_ports::PcmStreamPort` (stereo `f32` at
   the stream's own rate); `amp-adapter-decoder` implements it for the Symphonia decoder.
2. **Decks: N in the model, two shown.** `MAX_DECKS = 4`; the UI shows A and B (PLAN-055),
   C and D serve the remix work of PLAN-057. Each deck has a crossfader side (A left, B right,
   C/D thru by default).
3. **Tracks decode into memory while they play.** `spawn_loader` opens the stream on a loader
   thread and appends decoded audio to a `TrackBuffer` in 16,384-frame chunks, each published
   once through a `OnceLock`. The deck can start at once; any loaded frame is readable without
   locks, so seek, loops and cues are instant. Cost: about 21 MB per minute of stereo audio at
   44.1 kHz (f32), so roughly 100 MB for a 5-minute track on each loaded deck. This is
   DJ-mode-only memory and does not touch the idle footprint. A bounded window cache is the
   known alternative if this proves too heavy.
4. **Variable-rate playback per deck.** Each deck advances its playhead by
   `source rate / engine rate × (1 + pitch %)` per output frame and reads with 4-point Hermite
   interpolation. This does sample-rate conversion and tempo change in one step; pitch moves
   with tempo like a turntable. Key lock (TASK-205) will stretch time before this step.
5. **Two buses.** `DjEngine::render` produces a master bus (trim × fader × equal-power
   crossfader gain) and a cue bus (trim, pre-fader, decks with PFL on). Routing the cue bus to
   a second device or to one side of the main device is TASK-203.
6. **Real-time rules for `render`.** No locks, no allocation, no freeing, no I/O:
   - continuous controls are atomics in `DjParams`, read every block; gains ramp across each
     256-frame block, so changes never click;
   - discrete actions (`Load`, `Seek`, `Eject`) arrive through a lock-free SPSC queue;
   - a replaced track is sent back on a second queue and freed by `DjControl::collect_garbage`
     on the control side;
   - status (playhead, loaded seconds, ended, underruns) is published through atomics in
     `DjStatus`.
   A counting global allocator in `tests/dj_engine.rs` proves that `render` performs zero
   allocations and frees.
7. **Low-latency render thread.** `DjRuntime` runs the engine on its own thread and keeps each
   output queue topped up to a small lead (default about 50 ms) instead of filling it. The sound
   card callback (`amp-adapter-cpal`) still only drains a ring buffer, so the audio callback
   rules are unchanged. Rendering straight inside the callback is a later option once latency
   below about 20 ms is needed.
8. **Hidden until the DJ experience is accepted.** `experimental.dj_mode` in the config
   (default off) gates the new DJ mode until PLAN-055 is accepted.

## Consequences

- The old `DualDeckMixer`, `DeckChannelState` and the GUI's `dj_playback` worker stay only until
  PLAN-055 moves the DJ UI onto this engine, and are then removed. The kill EQ (TASK-039/100),
  WSOLA (TASK-041/100) and beat detector (TASK-101) are reused by TASK-205/207 only where their
  tests prove them correct.
- Loops, hot cues, sync, analysis and the full mixer (TASK-204–208) build on the deck playhead
  and the atomics introduced here rather than on new threads.
- Verified by `cargo test -p amp-core-engine dj`:
  - 44.1 kHz and 48 kHz decks at their true pitch on a 48 kHz bus;
  - pitch changes tempo and pitch together;
  - PFL is on the cue bus only;
  - clean stop at the end of a track;
  - 10 minutes of mixing with no underruns;
  - no allocation in `render`;
  - a deck plays while its track is still loading;
  - the runtime feeds a simulated sound card for 2 s without starving it, with latency near the lead.
