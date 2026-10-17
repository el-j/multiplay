# TASK-101: Auto-BPM Detection & Beatmatching Sync

## Plan Reference
- Plan: [PLAN-031](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-031-pro-live-dj-engine-and-realtime-mixer.md)
- Role: `audio_engine` & `dsp_engineering`

## Objectives
1. Connect `RealtimeBpmDetector` to live audio feeder to continuously compute BPM for Deck A and Deck B.
2. Add "SYNC" button in GUI and TUI DJ console that calculates tempo difference ratio and locks Deck B's pitch slider to match Deck A's exact BPM.
3. Display real-time BPM and pitch bend indicators in DJ console.
4. Add verification tests for BPM detection accuracy and sync calculation.

## Verification Command
`cargo test -p amp-core-engine -p amp-adapter-gui test_bpm`

## Status
- **Status:** Completed
