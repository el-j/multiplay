---
id: PLAN-003
title: Audiophile DSP Pipeline, Sinc Resampling & FFT Visualizer
status: planned
role: dsp_engineering
tasks:
  - TASK-007
  - TASK-008
  - TASK-009
---

# PLAN-003: Audiophile DSP Pipeline, Sinc Resampling & FFT Visualizer

## Objective
Construct the audiophile DSP chain placed between the audio decoder and hardware output: polyphase Sinc resampler using `rubato`, 10-band IIR graphic equalizer with classic Winamp center frequencies, and real-time zero-allocation Hann-windowed FFT spectrum analyzer.

## Architecture Context
- Vision Docs: `docs/vision/audiophile_dsp_resampling_visualizer.md`.
- Target Modules:
  - `crates/amp-core-engine/src/dsp/resampler.rs`: Bit-perfect polyphase Sinc interpolation via `rubato`.
  - `crates/amp-core-engine/src/dsp/biquad.rs` & `equalizer.rs`: Peaking IIR biquad filters (60Hz, 170Hz, 310Hz, 600Hz, 1kHz, 3kHz, 6kHz, 12kHz, 14kHz, 16kHz) with $\pm 12\text{ dB}$ range.
  - `crates/amp-core-engine/src/dsp/visualizer.rs`: Hann-windowed FFT spectrum computation generating 16 log-spaced normalized frequency bands (0.0 to 1.0).

## Tasks
1. [TASK-007](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-007-rubato-sinc-resampler.md): Bit-Perfect Polyphase Sinc Resampler (`rubato`)
2. [TASK-008](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-008-10-band-graphic-equalizer.md): 10-Band Graphic Equalizer with IIR Peaking Biquad Filterbank
3. [TASK-009](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-009-fft-spectrum-visualizer.md): Real-Time Zero-Allocation 16-Band FFT Spectrum Analyzer

## Definition of Done
- Resampler successfully processes multi-rate streams (44.1 kHz, 48 kHz, 96 kHz, 192 kHz) into hardware DAC clock rate.
- Equalizer allows dynamic band gain updates without clicks, pops, or audio dropouts.
- Visualizer computes 16 bands at 60 FPS with zero heap allocations during steady-state loop.
