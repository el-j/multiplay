---
id: TASK-035
title: Pure-Rust EBU R128 Loudness Normalizer & Peak Limiter
planId: PLAN-011
status: re-opened
role: dsp_engineering
auditFindings: ["ORC-01", "DSP-05"]
mappedTo: ["PLAN-040", "TASK-143"]
verificationCommand: cargo test -p amp-core-engine test_loudness
---

# TASK-035: Pure-Rust EBU R128 Loudness Normalizer & Peak Limiter

## Description
Implement the zero-C ITU-R BS.1770 / EBU R128 loudness normalization and limiting engine in `crates/amp-core-engine/src/dsp/loudness.rs`:
1. `LoudnessConfig`: Configurable `target_lufs` (e.g., $-14.0\text{ LUFS}$ default, $-18.0\text{ LUFS}$ audiophile mode), `max_peak_dbfs` ($-0.5\text{ dBFS}$ inter-sample headroom), and bypass toggle.
2. `PureVolumeNormalizer`:
   - Translates ReplayGain / LUFS metadata tags into linear target gain coefficients.
   - Smooth lookahead peak limiter clamping preventing clipping and DAC overflow.
   - Zero-allocation interleaved buffer processing.

## Verification
```bash
cargo test -p amp-core-engine test_loudness
```
