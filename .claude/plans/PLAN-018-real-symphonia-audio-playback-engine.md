---
id: PLAN-018
title: Real Symphonia Audio Playback Engine & Gapless Decoder Integration
status: completed
role: audio_engine
tasks:
  - TASK-057
  - TASK-058
---

# PLAN-018: Real Symphonia Audio Playback Engine & Gapless Decoder Integration

## Objective
Replace synthetic 4-tone melody generator with a genuine, production-grade audio decoding and streaming pipeline powered by `amp-adapter-decoder` (`symphonia`) and CPAL hardware sinks, enabling true playback of user music with accurate scrubbing and gapless track transitions.

## Architecture Context
- **Audio Decoder Pipeline (`amp-adapter-gui`, `amp-adapter-decoder`)**:
  - Decode local audio files (`std::fs::File`) through `PureDecoderEngine` / Symphonia into 32-bit floating point PCM audio frames.
  - Stream decoded frames through the lock-free ring buffer directly to the CPAL output callback.
  - Automatic polyphase sinc resampling via `RubatoSincResampler` if file sample rate differs from the DAC hardware rate.
- **Playback Tracking & Control**:
  - Compute accurate elapsed time based on decoded frames consumed: `elapsed_duration = frames / sample_rate`.
  - Precise seeking / scrubbing along the track timeline.
  - End-of-file (EOF) detection triggering automatic gapless advancement to the next playlist track.

## Tasks
1. [TASK-057](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-057-real-time-audio-decoder-worker-and-stream-feeder.md): Real-Time Audio Decoder Worker & Multi-Source Stream Feeder
2. [TASK-058](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-058-playback-progress-scrubbing-and-gapless-advancement.md): Playback Progress Tracking, Scrubbing & Gapless Track Advancement
