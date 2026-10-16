---
id: TASK-029
title: Dynamic Background Audio Playback Streamer in Runtime Composition Root
planId: PLAN-010
status: in_progress
role: audio_engine
verificationCommand: cargo test -p amp-app --test e2e_pipeline
---

# TASK-029: Dynamic Background Audio Playback Streamer in Runtime Composition Root

## Description
Wire the dynamic audio decoding thread into `crates/amp-app/src/main.rs`:
- On `PlayerCommand::PlayTrack(track)`:
  - Invoke `StreamResolverPort` corresponding to the track's `AudioSource`.
  - Stream data through `SharedPrecacheBuffer`.
  - Spin up decoding thread using `PureDecoderEngine::decode_stream_to_ring_buffer`.
  - Feed decoded PCM through `PureResampler` and `GraphicEqualizer`.
  - Push processed samples into `CpalAudioSink`'s lock-free `rtrb::Producer<f32>` ring buffer.
  - Compute spectrum bars using `SpectrumVisualizer` and update TUI visualizer bars in real-time.

## Verification
```bash
cargo test -p amp-app --test e2e_pipeline
```
