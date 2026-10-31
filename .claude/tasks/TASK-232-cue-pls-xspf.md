# TASK-232: Playlist & Album Containers: CUE Sheets, PLS, XSPF

## Plan Reference
- Plan: [PLAN-058](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-058-universal-audio-format-support.md)
- Role: `library_engineering`
- Priority: **P1**
- Depends on: TASK-225

## Objectives
1. **CUE sheets:** a single-file album (`album.flac` + `album.cue`, also embedded CUESHEET
   in FLAC) shows as separate tracks with titles, and each plays from its index to the next,
   gaplessly across track boundaries.
2. **PLS and XSPF:** import alongside the existing M3U/M3U8, from drop, dialog and scan;
   export M3U8 stays the default.
3. Relative paths, `file://` URLs, non-UTF-8 (Latin-1) CUE files and Windows line endings handled.

## Acceptance Criteria (user-visible / measurable)
- Dropping a FLAC+CUE album adds one row per track; seeking and next/previous stay inside the file.
- PLS and XSPF fixtures (local files and HTTP streams) import with titles.
- Pure-Rust verification passed (`./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test`).
- File length within `.nexus/policy.json` `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-storage/src/playlist.rs`, `src/cue.rs` (new)
- `crates/amp-domain/src/lib.rs` (track start/end offsets)
- `crates/amp-core-engine/src/player/` (play a range of a file)

## Automated Verification Command
`./scripts/verify_pure_rust.sh && cargo test -p amp-adapter-storage -p amp-core-engine`

## Manual Verification & Evidence
- **Build / Run Command:** `cargo run --release -p amp-adapter-gui --bin multiplay-gui`
- **Observed Behavior:** A FLAC+CUE album appears as tracks and plays through without gaps.
- **Evidence Links:** Screenshot.

## Status
- **Status:** Pending

## Implementation Evidence (2026-10-31)
- Domain: `TrackSegment { start, end }` and `TrackMetadata::segment` (`#[serde(default)]`).
- Engine: `player/segment.rs` `SegmentedStream` decorator (seek to start, EOF at end, duration/elapsed/seek relative to the range) applied in all three decoder open paths (async loader incl. gapless preload, blocking/DJ loader, worker's direct local-file open). Tests `amp-core-engine/tests/segment.rs` (3).
- Storage: `cue.rs` parses FILE/TRACK/TITLE/PERFORMER/INDEX 01/REM GENRE/DATE, UTF-8 (BOM) or Latin-1, CRLF; resolves a renamed audio file (`album.wav` → `album.flac`); one track per song with album, album artist, genre, year, track number, segment and duration; stable per-song ids. Folder scans list `cue_sheets` and leave out the single files they cover. Tests `amp-adapter-storage/tests/cue.rs` (4) with a 3-tone FLAC + UTF-8 and Latin-1/CRLF sheets.
- GUI: `playlist_import.rs` imports M3U/M3U8, PLS, XSPF (now relative to the playlist's folder; M3U previously ignored it) and CUE from drop, dialog and folder; dialog offers `.pls/.xspf/.cue`. Tests `amp-adapter-gui/tests/playlist_files.rs` (4), including the real engine playing CUE song 2 for exactly 1 s at 550 Hz.
- Not done: embedded `CUESHEET` tags inside FLAC files (separate follow-up).
- Gate: verify_pure_rust, cargo-deny, clippy `-D warnings`, `cargo test --workspace --no-fail-fast` 500 passed / 0 failed.
