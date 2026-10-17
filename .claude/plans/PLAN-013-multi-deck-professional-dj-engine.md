---
id: PLAN-013
title: Multi-Deck Professional DJ Engine & Universal Source Mixer
status: experimental
note: "Marked experimental per TASK-112 / ORC-06 pending M2 foundations"
role: audio_engine
tasks:
  - TASK-039
  - TASK-040
  - TASK-041
  - TASK-042
---

# PLAN-013: Multi-Deck Professional DJ Engine & Universal Source Mixer

## Objective
Implement professional multi-deck DJ mixing across arbitrary audio sources (FLAC, YouTube, Soulseek, P2P) as specified in `docs/vision/multi_deck_professional_dj_engine.md`:
1. 4-deck independent channel pipeline with 3-band isolator kill EQs and headphone PFL cue bus.
2. Real-time spectral energy BPM and beat-grid onset detector.
3. Allocation-free WSOLA time-stretching kernel for tempo shifting without pitch variation (Key Lock).
4. Professional DJ console interface with waveform beat grids, sync toggles, and crossfader transform curve.

## Architecture Context
- Crate Targets:
  - `crates/amp-core-engine/src/dj/deck.rs`: `DeckId`, `DeckChannelState`, 3-band kill filters, loop roll engine.
  - `crates/amp-core-engine/src/dj/beat_detector.rs`: `RealtimeBpmDetector`, spectral flux energy variance.
  - `crates/amp-core-engine/src/dj/wsola.rs`: WSOLA safe time-stretching kernel.
  - `crates/amp-adapter-tui` & `crates/amp-adapter-gui`: Multi-deck interactive visual console with live beat grid markers.
- Real-Time Constraints: DJ audio mixing, kill EQ filtering, and beat detection execute within the real-time processing loop without dynamic heap allocation.

## Tasks
1. [TASK-039](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-039-multi-deck-channel-architecture-and-kill-eqs.md): Multi-Deck Channel Architecture & 3-Band Kill Isolator EQs
2. [TASK-040](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-040-pure-rust-realtime-bpm-beat-detector.md): Pure-Rust Real-Time Transient BPM & Beat Grid Detector
3. [TASK-041](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-041-safe-wsola-time-stretcher-and-crossfader-mixer.md): Safe WSOLA Time-Stretcher & Crossfader Transform Mixer
4. [TASK-042](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-042-multi-deck-dj-console-tui-and-gui-interface.md): Multi-Deck Hardware-Style DJ Console Interface for TUI and GUI

## Definition of Done
- Multi-deck channels independently mix streams from diverse source adapters simultaneously.
- 3-band kill switches achieve complete band attenuation ($<-60\text{ dB}$).
- BPM detection correctly estimates standard musical tempos ($60-200\text{ BPM}$) within $\pm 1.5\text{ BPM}$.
- WSOLA adjusts playback speed $\pm 12\%$ with preserved pitch and zero allocation in audio callbacks.
- 100% pure Rust verified with zero C dependencies.
