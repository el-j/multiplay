---
id: TASK-008
title: 10-Band Graphic Equalizer with IIR Peaking Biquad Filterbank
planId: PLAN-003
status: pending
role: dsp_engineering
verificationCommand: cargo test -p amp-core-engine test_equalizer
---

# TASK-008: 10-Band Graphic Equalizer with IIR Peaking Biquad Filterbank

## Description
Implement the 10-band stereo graphic equalizer in `crates/amp-core-engine/src/dsp/`:
- `biquad.rs`: `BiquadFilter` implementing digital direct form II transposed peaking EQ filter.
- `equalizer.rs`: `GraphicEqualizer` with standard Winamp center frequencies: 60 Hz, 170 Hz, 310 Hz, 600 Hz, 1 kHz, 3 kHz, 6 kHz, 12 kHz, 14 kHz, 16 kHz.
- Supports $\pm 12\text{ dB}$ gain per band with `set_band_gain(band_idx, gain_db)`.
- In-line sample processing for interleaved stereo audio frames `process_interleaved(&mut [f32])`.

## Verification
```bash
cargo test -p amp-core-engine test_equalizer
```
