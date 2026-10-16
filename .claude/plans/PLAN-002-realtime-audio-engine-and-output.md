---
id: PLAN-002
title: Real-Time Audio Engine, Precaching Buffer & Hardware Output
status: completed
role: audio_engine
tasks:
  - TASK-004
  - TASK-005
  - TASK-006
---

# PLAN-002: Real-Time Audio Engine, Precaching Buffer & Hardware Output

## Objective
Implement the low-latency real-time audio pipeline: an in-memory precaching stream buffer supporting concurrent non-blocking reads and async network chunk appends, the CPAL hardware audio sink with lock-free `rtrb` ring buffer, and the pure-Rust Symphonia multi-format decoding engine.

## Architecture Context
- Vision Docs: `docs/vision/audio_engine_precaching.md`, `docs/vision/system_architecture_topology.md`.
- Target Crates:
  - `crates/amp-core-engine/src/precache.rs`: `SharedPrecacheBuffer` implementing `std::io::Read` and `std::io::Seek` with `WouldBlock` underflow semantics.
  - `crates/amp-adapter-cpal`: `CpalAudioSink` configuring output device, stream callback, and `rtrb::Consumer<f32>` popping.
  - `crates/amp-adapter-decoder`: `PureDecoderEngine` probing format container, initializing codec, and pushing samples to `rtrb::Producer<f32>`.

## Tasks
1. [TASK-004](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-004-memory-precache-buffer.md): In-Memory Precaching Stream (`SharedPrecacheBuffer`)
2. [TASK-005](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-005-cpal-hardware-audio-sink.md): Pure-Rust CPAL Hardware Audio Sink & Lock-Free Ring Buffer
3. [TASK-006](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-006-symphonia-decoder-engine.md): Symphonia Pure-Rust Multi-Format Demuxer & Decoder

## Definition of Done
- `cargo test -p amp-core-engine` validates `SharedPrecacheBuffer` concurrent read, seek, and chunk push without data race or deadlock.
- Audio callback adheres to real-time safety: no allocations, syscalls, or mutex locks on audio thread.
- Symphonia decodes MP3, FLAC, and WAV files cleanly into normalized interleaved `f32` samples.
