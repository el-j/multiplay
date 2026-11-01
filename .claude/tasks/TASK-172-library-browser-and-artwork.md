# TASK-172: Library Browser (Artists/Albums/Tracks) & Artwork

## Plan Reference
- Plan: [PLAN-047](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-047-ux-first-run-to-daily-driver.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: `UX-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-148

## Objectives
1. Views: Artists, Albums (grid with covers), Tracks (sortable), Recently Added, Sources.
2. Embedded art extraction (lofty) + remote cover URIs, cached in cache dir with size limits.
3. Virtualized lists for 100k tracks.

## Acceptance Criteria (user-visible / measurable)
- Scrolling 100k tracks stays at 60 FPS; album grid shows covers.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-gui/src/library_window.rs`
- `crates/amp-adapter-storage/src/artwork.rs`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending

## Implementation Evidence (2026-11-01, branch feat/plan-047-ux-2)
- Model: `amp_core_engine::library_index::LibraryIndex` — artists (album artist first, case-insensitive grouping), albums (disc/track order), tracks sortable by artist/title/album/length/added with text/artist/album filters, recently added, per-source counts. 100k tracks index and sort well under a second even in debug (`tests/library_index.rs`).
- Persistence: every import (`ingest_paths`) is stored once in `library.redb`, dated on first add (`library_store.rs`); loads on a background thread at start-up. `Track::library_key()` (source + CUE segment start) keeps the songs of a single-file album as separate entries (`amp-adapter-storage/tests/library_keys.rs`); whole-file keys unchanged.
- Artwork: `amp_adapter_storage::artwork::ArtworkCache` — 160 px PNG thumbnails from embedded art or the folder's cover/folder/front/album image, negative results remembered, pruned oldest-first above 64 MB (`tests/artwork.rs`). Local files only (no new connections).
- Window: `WindowId::Library` ("LIB" toggle), tabs Artists / Albums (cover grid) / Tracks (sortable columns, "+ Add all") / Recently added / Sources, filter box; `show_rows` virtualized lists; covers decoded on a worker thread, at most 120 textures (LRU, `CoverLru`). Click an artist/album → its tracks; double-click → play; right-click → add.
- Tests: `amp-adapter-gui/tests/library_window.rs` (remembered once, survives restart, rows to playlist, LRU cap).
- Not done: online cover URIs (would add connections), 60 FPS measured by hand at 100k.
