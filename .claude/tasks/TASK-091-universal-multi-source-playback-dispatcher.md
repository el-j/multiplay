# TASK-091: Universal Multi-Source Dispatcher in AudioPlaybackDriver

## Plan Reference
- Plan: [PLAN-029](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-029-progressive-network-streaming-and-universal-resolvers.md)
- Role: `audio_engine`

## Objectives
1. Implement `AudioStreamDecoder::open_stream<S: Read + Seek + Send + Sync + 'static>(stream: S)` in `amp-adapter-decoder` so any stream handle can be decoded.
2. In `AudioPlaybackDriver` (`amp-adapter-gui` and `amp-app`), remove the hardcoded restriction to `AudioSource::LocalFile`.
3. Support resolving streams for `AudioSource::YouTube`, `AudioSource::DirectHttp`, `AudioSource::Soulseek`, and `AudioSource::Spotify` through registered `StreamResolverPort` adapters.
4. Update UI track title and duration dynamically once the stream header is probed.

## Verification Command
`cargo test -p amp-adapter-decoder -p amp-adapter-gui -p amp-app`

## Status
- **Status:** Completed
