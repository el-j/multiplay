# TASK-107: Lookahead Track Preloader & Sample-Accurate Gapless Transition Feeder

## Plan Reference
- Plan: [PLAN-033](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-033-audiophile-seamless-gapless-playback-automix-and-loudness.md)
- Role: `audio_engine` & `qa_and_hardening`

## Objectives
1. Add `next_track` preloader queue in `AudioPlaybackDriver` allowing the engine to know the queued upcoming track in advance.
2. When the active track reaches the end-of-track window (remaining duration <= 4 seconds), asynchronously initialize and prime the next track's `AudioStreamDecoder` in the background.
3. In `playback_worker.rs`, when the active track finishes reading the final sample frame, immediately stitch the pre-decoded initial frames of the next track into the output chunk without waiting for GUI thread polling.
4. Notify the GUI of the automated track advance via atomic index/signal so UI selection and metadata update smoothly.

## Verification Command
`cargo test -p amp-adapter-gui -p amp-core-engine`

## Status
- **Status:** Completed
