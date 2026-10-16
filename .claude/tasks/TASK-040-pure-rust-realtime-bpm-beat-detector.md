---
id: TASK-040
title: Pure-Rust Real-Time Transient BPM & Beat Grid Detector
planId: PLAN-013
status: planned
role: dsp_engineering
verificationCommand: cargo test -p amp-core-engine test_bpm_detector
---

# TASK-040: Pure-Rust Real-Time Transient BPM & Beat Grid Detector

## Description
Implement the zero-C continuous BPM estimation and onset detection kernel in `crates/amp-core-engine/src/dj/beat_detector.rs`:
1. `RealtimeBpmDetector`:
   - Spectral flux and energetic transient onset detection across rolling audio blocks.
   - Dynamic thresholding constant ($C$) calculated from rolling historical energy variance.
   - Refractory period filtering to prevent spurious multi-triggering on sustained bass transients.
2. Low-pass smoothed BPM estimator:
   - Calculates instantaneous interval between beats and stabilizes BPM tracking across $60-200\text{ BPM}$.
   - Exposes deterministic phase offset for visual beat-grid synchronization.

## Verification
```bash
cargo test -p amp-core-engine test_bpm_detector
```
