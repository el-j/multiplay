---
id: TASK-073
title: Multi-Format E2E Audio Pipeline Test Harness
planId: PLAN-024
status: pending
role: testing
verificationCommand: cargo test -p amp-core-engine test_e2e_playback_pipeline
---

# TASK-073: Multi-Format E2E Audio Pipeline Test Harness

## Description
Build a comprehensive end-to-end test suite proving the real audio playback engine functions authentically without mocks:
1. Multi-Format Fixtures: Provide minimal valid audio stream fixtures for FLAC, MP3, WAV, OGG Vorbis, and AAC.
2. In-Memory Capture Sink: Implement a real-time `CaptureAudioSink` implementing `AudioSinkPort` that records rendered PCM float32 samples from the lock-free ring buffer.
3. Pipeline Verification: Verify that the decoded sample stream passes through:
   - Volume attenuation (exact decibel attenuation check)
   - 10-Band Graphic Equalizer (frequency boost/cut validation)
   - Soft-knee limiter (verifying peak ceiling $\le 1.0$ when pushed +12dB)
   - Accurate duration tracking and elapsed time reporting.
4. Fail-Fast & Corrupt File Testing: Validate that truncated, corrupted, or unsupported audio containers fail gracefully with typed errors rather than panic.

## Verification
```bash
cargo test -p amp-core-engine test_e2e_playback_pipeline
```

## Acceptance Criteria
- End-to-end verification passing across FLAC, MP3, WAV, OGG, and AAC.
- Numerical validation of DSP EQ response and limiter ceiling.
- Zero panics on corrupt or truncated streams.
