---
id: TASK-034
title: Dual-Pipeline Gapless Decoder Mixer & Real-Time Crossfader DSP
planId: PLAN-011
status: completed
role: dsp_engineering
verificationCommand: cargo test -p amp-core-engine test_crossfade
---

# TASK-034: Dual-Pipeline Gapless Decoder Mixer & Real-Time Crossfader DSP

## Description
Implement the real-time crossfade and gapless transition engine in `crates/amp-core-engine/src/dsp/crossfade.rs`:
1. `CrossfadeCurve`:
   - `Linear`: $(1 - t, t)$ linear gain slope.
   - `EqualPower`: Constant acoustic energy transition $(\cos(t \cdot \pi/2), \sin(t \cdot \pi/2))$ preserving total root-mean-square loudness across the blend.
2. `Crossfader`: Allocation-free frame stepper computing smooth gain curves over configurable frame durations.
3. Dual-Pipeline Mixer integration: Precache and initiate decoding of next track $N+1$ before track $N$ terminates, executing seamless overlap.

## Verification
```bash
cargo test -p amp-core-engine test_crossfade
```
