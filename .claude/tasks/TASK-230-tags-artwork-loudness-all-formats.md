# TASK-230: Tags, Artwork and Loudness for Every Decodable Format

## Plan Reference
- Plan: [PLAN-058](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-058-universal-audio-format-support.md)
- Role: `library_engineering`
- Priority: **P1**
- Depends on: TASK-225, TASK-226

## Objectives
1. For every `Playable` registry format, read title, artist, album, album artist, track/disc
   number, year, genre and embedded cover art through `lofty` (Vorbis comments, ID3v2, MP4
   atoms, AIFF/CAF chunks, APE tags).
2. Accurate duration for VBR MP3 without a Xing header and for streams without a length.
3. ReplayGain (track/album) and Opus R128 gain feed the loudness normalizer.
4. Files without tags fall back to `Artist - Title` parsed from the file name, then the bare file name.

## Acceptance Criteria (user-visible / measurable)
- Each fixture's expected tags and cover-art size are asserted in `amp-adapter-storage` tests.
- Loudness: a ReplayGain-tagged fixture and an R128-tagged Opus fixture play at the target level (capture-sink RMS test).
- Pure-Rust verification passed (`./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test`).
- File length within `.nexus/policy.json` `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-storage/src/scanner.rs`
- `crates/amp-core-engine/src/dsp/loudness.rs`

## Automated Verification Command
`./scripts/verify_pure_rust.sh && cargo test -p amp-adapter-storage -p amp-core-engine`

## Manual Verification & Evidence
- **Build / Run Command:** `cargo run --release -p amp-adapter-gui --bin multiplay-gui`
- **Observed Behavior:** A mixed-format library shows correct tags and artwork for every format.
- **Evidence Links:** Screenshot.

## Status
- **Status:** Pending

## Implementation Evidence (2026-10-31)
- New `crates/amp-adapter-storage/src/tags.rs` (moved out of `scanner.rs`): lofty first; for playable files lofty cannot open (CAF, Matroska/WebM, ADPCM WAV) a Symphonia format-reader fallback supplies duration, stream properties and standard tags, so every decodable file gets a library entry (before: dropped CAF/MKA/WebM/ADPCM files were silently not added).
- `TrackMetadata` gained `album_artist`, `disc_number`, `year`, `genre` (`#[serde(default)]`, JSON storage stays compatible); filled from lofty and Symphonia tags; Matroska album-level `ARTIST`/`ALBUM`/`TITLE` raw tags mapped.
- Embedded art: `embedded_cover_art(path) -> CoverArt { mime_type, data }`; tracks with art get `cover_art_uri = "embedded:<path>"`. (No GUI shows artwork yet.)
- Loudness: ReplayGain tags were read but never used; `TrackMetadata::integrated_lufs()` now falls back to ReplayGain (reference −18 LUFS), so the engine's normalizer uses it. R128 (Opus) was already used; OpusHead output gain is applied in the decoder (TASK-229).
- File names: untagged files take "Artist - Title" from the name (scanner test updated to the new behaviour).
- Fixtures: AIFF and MP2 now carry ID3 tags; `cover.flac/.mp3/.m4a` with album artist, genre, year, disc, track, ReplayGain −6.5 dB and the 64 px app icon as PNG cover.
- Tests: `amp-adapter-storage/tests/format_tags.rs` (6): every playable fixture → track with expected tags and duration (±60 ms); rich tags; cover bytes equal the source PNG; ReplayGain → −11.5 LUFS; name fallback.
- Known limits: CAF and ADPCM WAV tags are not read (neither lofty nor Symphonia expose them); ffmpeg-written MKA/WebM titles live outside the tags.
- Gate: verify_pure_rust, cargo-deny, clippy `-D warnings`, `cargo test --workspace --no-fail-fast` 489 passed / 0 failed.
