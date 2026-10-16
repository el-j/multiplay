---
id: TASK-006
title: Symphonia Pure-Rust Multi-Format Demuxer & Decoder
planId: PLAN-002
status: completed
role: audio_engine
verificationCommand: cargo test -p amp-adapter-decoder
---

# TASK-006: Symphonia Pure-Rust Multi-Format Demuxer & Decoder

## Description
Implement `PureDecoderEngine` in `crates/amp-adapter-decoder`:
- Uses pure-Rust `symphonia` with zero C-dependencies (no FFmpeg, libav, or libmad).
- Probes container formats (MP3, FLAC, WAV, AAC, Vorbis, Opus, Ogg).
- Handles gapless playback and format options.
- Decodes demuxed packets into `SampleBuffer<f32>` and writes interleaved normalized `f32` samples to `rtrb::Producer<f32>`.
- Provides backpressure yielding when the hardware ring buffer is full without dropping frames.

## Verification
```bash
cargo test -p amp-adapter-decoder
```
