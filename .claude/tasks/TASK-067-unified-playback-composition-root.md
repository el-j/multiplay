---
id: TASK-067
title: Unified Playback Composition Root & Hardware Audio Wiring
planId: PLAN-022
status: pending
role: audio_engine
verificationCommand: cargo test -p amp-core-engine -p amp-adapter-cpal -p amp-adapter-decoder
---

# TASK-067: Unified Playback Composition Root & Hardware Audio Wiring

## Description
Build the complete, connected runtime audio playback chain identified as missing in the audit (`docs/orchestration-audit.md` section 3.4 & 8):
1. End-to-End Pipeline Wiring: Connect the entire audio pipeline:
   `TrackSource` -> `StreamResolver` -> `AudioStreamDecoder` (Symphonia) -> `DspPipeline` (10-Band EQ + Soft-Knee Limiter + EBU R128) -> `rtrb` Lock-Free Ring Buffer -> CPAL Real-Time Audio Callback.
2. Background Decode Worker: Implement an asynchronous decode thread that reads packets from Symphonia, decodes interleaved f32 PCM samples, processes them through the DSP filterbank, and pushes chunks into the lock-free ring buffer without blocking the audio thread.
3. Real-Time Audio Callback Safety: Strictly enforce real-time constraints in the CPAL output callback: zero heap allocations, zero mutex locks, zero syscalls, and zero file/network I/O.
4. Transport Controls: Wire play, pause, resume, seek, stop, volume gain, and mute commands to immediately reflect in audio output.

## Verification
```bash
cargo test -p amp-core-engine -p amp-adapter-cpal -p amp-adapter-decoder
```

## Acceptance Criteria
- Full connected path from track selection to CPAL output buffer verified.
- Real-time safety guarantees maintained in audio output callback.
- Unit and integration tests verify playback state changes, buffer flow, and sample-accurate seeking.
