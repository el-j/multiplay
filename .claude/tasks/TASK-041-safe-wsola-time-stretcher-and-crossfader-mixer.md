---
id: TASK-041
title: Safe WSOLA Time-Stretcher & Crossfader Transform Mixer
planId: PLAN-013
status: completed
role: dsp_engineering
verificationCommand: cargo test -p amp-core-engine test_wsola_time_stretcher
---

# TASK-041: Safe WSOLA Time-Stretcher & Crossfader Transform Mixer

## Description
Implement the allocation-free Waveform Similarity Overlap-Add (WSOLA) pitch-preserving time-stretching engine in `crates/amp-core-engine/src/dj/wsola.rs`:
1. WSOLA Time-Stretching Kernel:
   - Adjusts playback speed from $0.5\times$ to $2.0\times$ without altering musical pitch (Key Lock / Master Tempo).
   - Zero dynamic heap allocation during active processing loop; preallocated circular analysis and synthesis buffers.
2. Crossfader Transform Mixer:
   - Multi-deck crossfader routing assignable channels to Deck Left or Deck Right buses.
   - Selectable crossfader curves: Linear fade, Smooth Dip, and Sharp Scratch Cut.

## Verification
```bash
cargo test -p amp-core-engine test_wsola_time_stretcher
```
