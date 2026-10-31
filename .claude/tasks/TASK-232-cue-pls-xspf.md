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
