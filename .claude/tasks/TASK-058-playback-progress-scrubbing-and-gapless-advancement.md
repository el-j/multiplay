---
id: TASK-058
title: Playback Progress Tracking, Scrubbing & Gapless Track Advancement
planId: PLAN-018
status: completed
role: audio_engine
verificationCommand: cargo test -p amp-adapter-gui test_playback_tracking_and_seek
---

# TASK-058: Playback Progress Tracking, Scrubbing & Gapless Track Advancement

## Description
Provide precise playback tracking, interactive timeline scrubbing, and automatic playlist advancement:
1. Frame Accurate Progress: Track total frames output to calculate precise elapsed duration: `elapsed = frames / sample_rate`.
2. Interactive Timeline Scrubbing:
   - Render track progress bar in the main player window with current time (`mm:ss`) and total track duration (`mm:ss`).
   - Allow user to drag or click progress slider to seek.
   - On seek, signal decoder worker to seek reader to target frame/timestamp, flush ring buffer, and resume streaming.
3. Automatic Gapless Advancement:
   - Detect when audio decoder reaches EOF (end of file).
   - Automatically trigger playback of the next track in the playlist.
   - Obey Repeat (Off, All, One) and Shuffle playback modes.

## Verification
```bash
cargo test -p amp-adapter-gui test_playback_tracking_and_seek
```
