---
id: TASK-057
title: Real-Time Audio Decoder Worker & Multi-Source Stream Feeder
planId: PLAN-018
status: completed
role: audio_engine
verificationCommand: cargo test -p amp-adapter-gui test_real_audio_streaming
---

# TASK-057: Real-Time Audio Decoder Worker & Multi-Source Stream Feeder

## Description
Replace the 4-tone synthetic melody generator in `AudioPlaybackDriver` with an authentic pure-Rust multi-format audio decoding worker:
1. Pure Decoder Bridge: Integrate `PureDecoderEngine` (`amp-adapter-decoder` / `symphonia`) with `AudioPlaybackDriver`.
2. File Streaming: When a local track is selected, open file (`std::fs::File`), demux container (MP3, FLAC, WAV, OGG, AAC), and decode raw PCM packets into normalized `f32` samples.
3. Resampling Integration: If track native sample rate differs from CPAL hardware sink rate (e.g. 44.1 kHz -> 48 kHz), process frames through `RubatoSincResampler`.
4. Ring Buffer Streaming: Push decoded interleaved stereo samples into the lock-free `rtrb::Producer<f32>` buffer with smooth backpressure handling.
5. Thread Lifecycle: Ensure worker thread gracefully stops decoding previous track when seeking or changing tracks without deadlocks or buffer overruns.

## Verification
```bash
cargo test -p amp-adapter-gui test_real_audio_streaming
```
