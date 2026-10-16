---
id: TASK-072
title: Decompose CPAL Sink and Symphonia Decoder into Modular Units
planId: PLAN-023
status: completed
role: audio_engine
verificationCommand: cargo check -p amp-adapter-cpal -p amp-adapter-decoder
---

# TASK-072: Decompose CPAL Sink and Symphonia Decoder into Modular Units

## Description
Deconstruct monolithic files in the audio adapters (`amp-adapter-cpal/src/lib.rs` at 407 lines and `amp-adapter-decoder/src/lib.rs` at 388 lines):
1. `amp-adapter-cpal`:
   - `device.rs`: CoreAudio / ALSA / WASAPI host device enumeration, default sink query, and config negotiation.
   - `stream.rs`: Audio stream lifecycle (build, play, pause, error callback).
   - `ring_buffer.rs`: Pure-Rust `rtrb` lock-free ring-buffer integration and audio callback handler.
2. `amp-adapter-decoder`:
   - `demuxer.rs`: Container probing, format detection, and packet extraction.
   - `decoder_core.rs`: Packet decoding into audio sample buffers across FLAC, MP3, WAV, Vorbis, AAC, Opus.
   - `sample_converter.rs`: Interleaved/planar conversion and float32 normalization.
   - `seek.rs`: Precise stream seeking and time offset mapping.

## Verification
```bash
cargo check -p amp-adapter-cpal -p amp-adapter-decoder
```

## Acceptance Criteria
- Every file in `crates/amp-adapter-cpal/src/` and `crates/amp-adapter-decoder/src/` is strictly under 300 lines.
- Complete modular separation of concerns.
- Real-time safety guarantees preserved across all sub-modules.
