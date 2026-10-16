---
id: TASK-009
title: Real-Time Zero-Allocation 16-Band FFT Spectrum Analyzer
planId: PLAN-003
status: pending
role: dsp_engineering
verificationCommand: cargo test -p amp-core-engine test_visualizer
---

# TASK-009: Real-Time Zero-Allocation 16-Band FFT Spectrum Analyzer

## Description
Implement `SpectrumVisualizer` in `crates/amp-core-engine/src/dsp/visualizer.rs`:
- Precomputes Hann window coefficients for 512-point FFT.
- Implements `push_samples(&[f32])` to maintain circular sample buffer.
- `compute_spectrum_bands() -> [f32; 16]` executes pure-Rust FFT transformation.
- Aggregates logarithmic frequency bands into 16 normalized values (0.0 to 1.0) for driving UI spectrum meters.
- Zero heap allocations during compute calls to ensure smooth 60 FPS visual rendering.

## Verification
```bash
cargo test -p amp-core-engine test_visualizer
```
