---
id: TASK-059
title: Continuous Biquad Filter Coefficient Morphing & State Preservation
planId: PLAN-019
status: completed
role: dsp_engineering
verificationCommand: cargo test -p amp-core-engine test_biquad_state_preservation
---

# TASK-059: Continuous Biquad Filter Coefficient Morphing & State Preservation

## Description
Eliminate clicking, buzz, and audio distortion caused by filter state wipeouts in the 10-band equalizer:
1. State Preservation: In `BiquadFilter`, implement `update_coefficients(b0, b1, b2, a1, a2)` that updates filter coefficients in-place while keeping the internal delay line registers `x1, x2, y1, y2` intact.
2. Change Detection Guard: In `GraphicEqualizer`, only recalculate biquad filter coefficients when the user modifies gain by more than the threshold ($|\Delta| > 0.01$ dB), eliminating redundant per-block calculations.
3. Audio Driver Fix: In `AudioPlaybackDriver`, remove the loop calling `eq.set_band_gain(...)` every 10ms frame chunk; only update EQ when gains actually change.
4. Frequency Boundary Protection: Ensure center frequencies are safely bounded below the Nyquist limit ($< 0.49 \times \text{sample\_rate}$) to prevent numerical instability or infinities at arbitrary sample rates.

## Verification
```bash
cargo test -p amp-core-engine test_biquad_state_preservation
```
