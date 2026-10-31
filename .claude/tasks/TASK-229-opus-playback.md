# TASK-229: Opus Playback: Ogg Opus, WebM/MKA and YouTube Opus Streams

## Plan Reference
- Plan: [PLAN-058](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-058-universal-audio-format-support.md)
- Role: `codec_engineering, provider_integration`
- Priority: **P1**
- Depends on: TASK-228 (go decision), TASK-227 (`mkv` container)

## Objectives
1. Register the ADR-005 decoder with Symphonia's codec registry so `.opus` (Ogg) and Opus in
   WebM/MKA open through the existing `AudioStreamDecoder` path.
2. Honour RFC 7845: pre-skip, output gain, 48 kHz output (resampled by the engine), channel
   mapping families 0 and 1; gapless joins between Opus tracks.
3. Remove ADR-004's explicit Opus rejection; mark Opus `Playable` in the TASK-225 registry so
   scanner, dialog, drop and phone upload accept `.opus`.
4. YouTube: keep AAC (itag 140) first, but fall back to Opus/WebM (itag 251/250/249) when no
   AAC stream exists, and allow it for downloads.
5. Read R128 gain tags for loudness (with TASK-230).

## Acceptance Criteria (user-visible / measurable)
- Opus fixtures (Ogg, WebM; mono, stereo, 5.1) pass the TASK-226 matrix and engine playback.
- Two consecutive Opus tracks play gaplessly (capture-sink test, no inserted silence).
- A YouTube video without an AAC stream plays (live canary, `#[ignore]`d in normal runs).
- Pure-Rust verification passed (`./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test`).
- File length within `.nexus/policy.json` `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-decoder/src/stream.rs`, `Cargo.toml`
- `crates/amp-adapter-youtube/src/lib.rs` (format scoring)
- `docs/adr/ADR-004-opus-codec-scope.md` (mark superseded)

## Automated Verification Command
`./scripts/verify_pure_rust.sh && cargo test -p amp-adapter-decoder -p amp-core-engine -p amp-adapter-youtube`

## Manual Verification & Evidence
- **Build / Run Command:** `cargo run --release -p amp-adapter-gui --bin multiplay-gui`
- **Observed Behavior:** Drop an `.opus` album; it plays gaplessly with correct loudness.
- **Evidence Links:** Screenshot, CI run URL.

## Status
- **Status:** Pending

## Implementation Evidence (2026-10-31)
- ADR-005 accepted by the owner: `rusty-opus` `=1.0.2` (pure Rust, SIMD intrinsics only); ADR-004 marked superseded.
- `crates/amp-adapter-decoder/src/opus.rs`: `OpusDecoder` implements Symphonia's `AudioDecoder`/`RegisterableAudioDecoder` over `rusty_opus::multistream::OpusMSDecoder` (families 0 and 1, up to 8 channels, Vorbis order reordered to Symphonia planes), applies the OpusHead output gain, and trims packets for gapless playback. `probe::codec_registry()` = Symphonia's enabled codecs + Opus, used for opening and for decoder rebuilds.
- Symphonia 0.6.1 findings: the Ogg reader trims end padding but leaves the pre-skip to the decoder (the adapter drops it from the packet at timestamp 0, also after a rewind), and counts the pre-skip in `num_frames` (the probe subtracts it for Opus).
- Registry: Opus `Playable`; scanner, dialogs, drop and phone upload accept `.opus`.
- Fixtures: `opus_stereo.opus`, `opus_mono.opus`, `opus_5_1.opus` (mapping family 1), `opus.webm`; all pass the decode matrix (path, stream, seek) and engine playback; new `ogg_opus_decodes_to_the_exact_encoded_length` proves gapless trimming (48,000 / 24,000 frames exactly).
- YouTube: format scoring already falls back from AAC (itag 140) to WebM/Opus; those streams are now decodable (live canary still blocked by the rusty_ytdl extractor issue).
- Gate: verify_pure_rust, cargo-deny, clippy `-D warnings`, `cargo test --workspace --no-fail-fast` 483 passed / 0 failed.
