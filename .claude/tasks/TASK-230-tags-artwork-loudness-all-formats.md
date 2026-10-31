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
