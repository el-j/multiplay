---
id: TASK-005
title: Pure-Rust CPAL Hardware Audio Sink & Lock-Free Ring Buffer
planId: PLAN-002
status: pending
role: audio_engine
verificationCommand: cargo test -p amp-adapter-cpal
---

# TASK-005: Pure-Rust CPAL Hardware Audio Sink & Lock-Free Ring Buffer

## Description
Implement `CpalAudioSink` in `crates/amp-adapter-cpal`:
- Integrates with `cpal` (CoreAudio on macOS, WASAPI on Windows, ALSA on Linux) using pure-Rust bindings.
- Uses `rtrb::RingBuffer<f32>` with 65,536 frames capacity for lock-free SPSC communication between audio decoding thread (Producer) and CPAL hardware callback (Consumer).
- Strictly follows real-time safety constraints: audio callback contains NO memory allocations, NO system calls, NO file I/O, and NO mutex locks.
- Returns `rtrb::Producer<f32>` to the engine for feeding decoded samples.

## Verification
```bash
cargo test -p amp-adapter-cpal
```
