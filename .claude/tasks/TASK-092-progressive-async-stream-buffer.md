# TASK-092: Progressive Asynchronous Stream Buffer & YouTube Streaming Overhaul

## Plan Reference
- Plan: [PLAN-029](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-029-progressive-network-streaming-and-universal-resolvers.md)
- Role: `integrations` & `audio_engine`

## Objectives
1. Expose `SharedPrecacheBuffer` in `amp-ports` (or shareable pure-Rust stream crate) so adapters can stream chunks progressively.
2. In `amp-adapter-youtube`, refactor `resolve_stream` to initialize a `SharedPrecacheBuffer`, spawn a background worker that fetches chunks from `rusty_ytdl` and pushes to the buffer, and return as soon as the initial probe threshold (~128KB) is reached.
3. Ensure playback starts in <500ms without buffering whole multi-megabyte streams in RAM.
4. Add unit test for progressive buffer stream decoding.

## Verification Command
`cargo test -p amp-adapter-youtube`

## Status
- **Status:** Completed
