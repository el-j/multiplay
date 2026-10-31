# TASK-226: Golden Fixture Corpus & Decode Matrix (FLAC First)

## Plan Reference
- Plan: [PLAN-058](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-058-universal-audio-format-support.md)
- Role: `codec_engineering`
- Priority: **P0**
- Depends on: TASK-225

## Objectives
1. Commit tiny (≤ 100 KB each, ~1 s) fixtures under `crates/amp-adapter-decoder/tests/fixtures/`,
   generated once by a documented script (`scripts/make_audio_fixtures.sh`, ffmpeg/flac/opusenc
   as offline dev tools only; nothing links them):
   - FLAC: 16-bit/44.1 kHz, 24-bit/96 kHz, 24-bit/192 kHz, 5.1, with and without seek table
   - MP3 CBR and VBR (Xing), AAC-LC in M4A, ALAC in M4A, Ogg Vorbis
   - WAV PCM 16/24-bit and 32-bit float, AIFF, CAF
   - Opus in Ogg, Opus in WebM, Vorbis in MKA (enabled by TASK-227/229)
2. A decode-matrix test per fixture: opens by path and by stream (no extension hint), reports
   sample rate, channels and duration within tolerance, decodes to the end without error, seeks
   to the middle, and its first samples match a stored checksum.
3. Engine test: each fixture plays through the headless `PlayerEngine` (capture sink) to the
   end, including resampling of 96/192 kHz and downmix of 5.1 to stereo.

## Acceptance Criteria (user-visible / measurable)
- FLAC fixtures pass in all three ways (path, stream, engine) on macOS, Linux and Windows CI.
- Every registry entry marked `Playable` has at least one passing fixture; removing a fixture fails CI.
- **Mocks Rule:** real files only; no synthetic decoder doubles in the matrix.
- Pure-Rust verification passed (`./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test`).
- File length within `.nexus/policy.json` `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-decoder/tests/format_matrix.rs` (new), `tests/fixtures/`
- `crates/amp-core-engine/tests/format_playback.rs` (new)
- `scripts/make_audio_fixtures.sh` (new)

## Automated Verification Command
`./scripts/verify_pure_rust.sh && cargo test -p amp-adapter-decoder --test format_matrix && cargo test -p amp-core-engine --test format_playback`

## Manual Verification & Evidence
- **Build / Run Command:** `cargo run --release -p amp-adapter-gui --bin multiplay-gui`
- **Observed Behavior:** Drop each fixture onto the playlist; each plays and shows correct duration.
- **Evidence Links:** CI run URL with the matrix on three OSes.

## Status
- **Status:** Pending
