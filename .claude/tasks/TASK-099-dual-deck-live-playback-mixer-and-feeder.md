# TASK-099: Dual-Deck Live Playback Mixer & Feeder

## Plan Reference
- Plan: [PLAN-031](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-031-pro-live-dj-engine-and-realtime-mixer.md)
- Role: `audio_engine`

## Objectives
1. Add dual-deck stream decoding capability into `AudioPlaybackDriver` allowing Deck A and Deck B to stream independent audio sources concurrently.
2. Mix Deck A and Deck B PCM audio buffers using `DjCrossfader` with selectable curve profile (SmoothDip, Linear, SharpCut).
3. Connect `MultiplayGuiApp` and DJ console controls to load tracks into Deck A and Deck B.
4. Add unit test verifying dual-stream concurrent decoding and crossfading.

## Verification Command
`cargo test -p amp-core-engine -p amp-adapter-gui`

## Status
- **Status:** Completed
