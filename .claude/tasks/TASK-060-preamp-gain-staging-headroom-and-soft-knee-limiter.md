---
id: TASK-060
title: Preamp Gain Staging, Headroom Management & Soft-Knee Limiter DSP
planId: PLAN-019
status: completed
role: dsp_engineering
verificationCommand: cargo test -p amp-core-engine test_eq_headroom_and_limiter
---

# TASK-060: Preamp Gain Staging, Headroom Management & Soft-Knee Limiter DSP

## Description
Protect audio fidelity against digital clipping, wrap-around distortion, and volume blowout when boosting EQ frequency bands:
1. Preamp Gain Control: Add a Preamp slider (-12.0 dB to +12.0 dB) to `GraphicEqualizer`, allowing attenuation before filter processing to provide clean headroom.
2. Dynamic Headroom Calculation: Track cumulative positive gain boosts across the 10 bands to advise optimal preamp setting.
3. Pure-Rust Audiophile Soft-Knee Limiter:
   - Implement transparent polynomial / hyperbolic tangent saturation curve at the output of the equalizer cascade.
   - For signals $|x| \le 0.9$, pass through 100% linear and uncolored.
   - For signals exceeding $0.9$, compress smoothly towards $1.0$, guaranteeing output samples never exceed $[-1.0, 1.0]$.
4. Zero-distortion verification tests: Confirm that simultaneous +12 dB boosts on multiple bands produce smooth, harmonic saturation without harsh digital wrapping or NaNs.

## Verification
```bash
cargo test -p amp-core-engine test_eq_headroom_and_limiter
```
