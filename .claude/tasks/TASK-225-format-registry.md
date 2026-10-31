# TASK-225: One Format Registry for Scanner, Dialog, Drop and Upload

## Plan Reference
- Plan: [PLAN-058](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-058-universal-audio-format-support.md)
- Role: `codec_engineering`
- Priority: **P0**
- Depends on: —

## Objectives
1. Add `amp_domain::formats`: one table of audio formats with extension(s), MIME types,
   container, codec, and a status (`Playable`, `Planned`, `Unsupported { reason }`).
2. Derive every list from it: `amp-adapter-storage::scanner::SUPPORTED_AUDIO_EXTENSIONS`,
   `amp-adapter-gui::file_dialog::SUPPORTED_EXTENSIONS` (and the macOS panel script),
   `amp-adapter-remote` upload allow-list, `amp-core-engine::downloader`, and the playlist
   "mp3 · flac · …" hint text.
3. Decoder hints (extension → Symphonia hint, MIME → hint) come from the same table.
4. Scanning and dropping report skipped files by format ("3 files skipped: WavPack is not
   supported yet") instead of ignoring them.

## Acceptance Criteria (user-visible / measurable)
- A test fails if any of the lists above defines extensions on its own.
- A test asserts every `Playable` entry has a golden fixture in TASK-226's corpus.
- Dropping a folder with unsupported files shows how many were skipped and why.
- Pure-Rust verification passed (`./scripts/verify_pure_rust.sh && ./scripts/verify_pure_rust.sh --self-test`).
- File length within `.nexus/policy.json` `maxFileLines` (500).

## Key Files
- `crates/amp-domain/src/formats.rs` (new)
- `crates/amp-adapter-storage/src/scanner.rs`
- `crates/amp-adapter-gui/src/file_dialog.rs`, `playlist_window.rs`
- `crates/amp-adapter-remote/src/upload.rs`
- `crates/amp-core-engine/src/downloader.rs`

## Automated Verification Command
`./scripts/verify_pure_rust.sh && cargo test -p amp-domain -p amp-adapter-storage -p amp-adapter-gui -p amp-adapter-remote`

## Manual Verification & Evidence
- **Build / Run Command:** `cargo run --release -p amp-adapter-gui --bin multiplay-gui`
- **Observed Behavior:** Drop a folder mixing FLAC, MP3 and an unsupported format; supported files are added, the rest are reported.
- **Evidence Links:** Screenshot.

## Status
- **Status:** Pending
