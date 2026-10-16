---
id: TASK-007
title: Bit-Perfect Polyphase Sinc Resampler
planId: PLAN-003
status: completed
role: dsp_engineering
verificationCommand: cargo test -p amp-core-engine test_resampler
---

# TASK-007: Bit-Perfect Polyphase Sinc Resampler

## Description
Implement `PureResampler` in `crates/amp-core-engine/src/dsp/resampler.rs`:
- Uses pure-Rust `rubato` with `SincFixedIn<f32>` polyphase Sinc interpolation.
- Converts arbitrary source stream sample rates (44.1 kHz, 48 kHz, 88.2 kHz, 96 kHz, 176.4 kHz, 192 kHz) to the target DAC hardware sample rate.
- Uses high-quality Blackman-Harris 2 window function and 256 sinc length for audiophile transparency and zero audible aliasing.

## Verification
```bash
cargo test -p amp-core-engine test_resampler
```
