# PLAN-029: Progressive Network Streaming & Universal Resolvers (YouTube & HTTP)

## Overview & Objective
Enable `AudioPlaybackDriver` in both GUI and TUI/CLI to play any `AudioSource` variant (YouTube, DirectHttp, Precache, LocalFile) rather than only `LocalFile`.
Eliminate blocking memory downloads in `PureYouTubeAdapter` by streaming chunks progressively into `SharedPrecacheBuffer` so playback starts in under 500ms.
Implement a pure-Rust HTTP/HTTPS stream resolver (`reqwest-rustls`) for direct audio streams and internet radio (Icecast/Shoutcast).

## Technical Architecture & Design
1. **Universal Stream Dispatcher in AudioPlaybackDriver:**
   - In `run_playback_loop`, replace hardcoded `if let AudioSource::LocalFile` with a multi-source dispatch mechanism.
   - For `LocalFile`, open via `AudioStreamDecoder::open_file`.
   - For `YouTube`, `DirectHttp`, etc., resolve the `Box<dyn AudioSourceStream>` via registered `StreamResolverPort` implementations.
   - Decode any resolved stream using `AudioStreamDecoder::open_stream`.
2. **Progressive Asynchronous Stream Buffering:**
   - Enhance `SharedPrecacheBuffer` with non-blocking initial chunk probe threshold (~128KB).
   - In `PureYouTubeAdapter`, spawn background tokio worker to push chunks progressively into the buffer without blocking `resolve_stream`.
   - Enable instantaneous start (<500ms latency) while background download continues.
3. **Pure-Rust Direct HTTP & Internet Radio Resolver:**
   - Implement `HttpStreamResolver` adhering to `StreamResolverPort` for `AudioSource::DirectHttp`.
   - Use `reqwest` with `rustls-tls` (100% pure Rust, zero OpenSSL).
   - Support streaming HTTP audio (`.mp3`, `.flac`, `.aac`, `.ogg`) and live internet radio streams.
4. **Resilient Streaming Verification:**
   - Integration tests validating live chunk buffering, EOF detection, error recovery, and seeking in network streams.

## Tasks Breakdown
- **TASK-091**: Universal Multi-Source Dispatcher in AudioPlaybackDriver
- **TASK-092**: Progressive Asynchronous Stream Buffer & YouTube Streaming Overhaul
- **TASK-093**: Pure-Rust Direct HTTP & Internet Radio Resolver (`reqwest-rustls`)
- **TASK-094**: Real Network Streaming Verification Harness
