# AUDIT-07 — Persistence, Library & Configuration

Part of [AUDIT-2026-10-17](README.md).

## DAT-01 — Four ad-hoc storage locations, none platform-correct · P1
**Evidence:**
| Data | Location | Source |
| :--- | :--- | :--- |
| TUI library DB | `$TMPDIR/multiplay_db_<random uuid>/library.redb` | `crates/amp-app/src/app_state.rs:47` |
| GUI session playlist | `$HOME/.multiplay/session_playlist.json` | `crates/amp-adapter-storage/src/playlist.rs` |
| Window layout | `$HOME/.config/multiplay/window_layout.json` | `crates/amp-adapter-gui/src/layout_manager.rs` |
| Config (unused) | `$HOME/.config/multiplay/multiplay.toml` | `crates/amp-core-engine/src/config_store.rs:97` |
| Downloads (unused) | `~/.multiplay/downloads` (literal `~`, never expanded) | `crates/amp-domain/src/config.rs` |

`HOME` usually isn't set on Windows, so everything falls back to `.`, the current working directory.
**Fix:** One `AppPaths` service using the `directories` crate (`~/Library/Application Support/multiplay`, `%APPDATA%\multiplay`, XDG on Linux), injected everywhere, with a migration for existing files. → TASK-147

## DAT-02 — TUI library is wiped on every launch · P0
**Evidence:** `temp_dir().join(format!("multiplay_db_{}", Uuid::new_v4()))` (`crates/amp-app/src/app_state.rs:47`). It's a fresh DB every run, and orphaned DBs pile up in `$TMPDIR`. The GUI doesn't use redb at all; its "library" is the session playlist JSON.
**Fix:** One persistent library DB in the data directory, shared by both front-ends. → TASK-148

## DAT-03 — Library records have no stable identity or schema versioning · P1
**Evidence:** `extract_track_from_file` assigns `TrackId::new()` (random v4) on every scan (`crates/amp-adapter-storage/src/scanner.rs:125`), so every rescan duplicates the library. Records are `bincode 1.x` of serde structs (`crates/amp-adapter-storage/src/lib.rs`): adding any field to `Track`/`TrackMetadata` makes every stored record undeserializable, and one bad record fails `fetch_all` entirely. `date_added_epoch` is always 0.
**Fix:** A deterministic ID (hash of the canonical source locator), a versioned envelope (`{v: u16, payload}`) with migrations, per-record error tolerance, and secondary indexes (artist/album/path). → TASK-148

## DAT-04 — Scanner is synchronous, blocking, symlink-unsafe · P1
**Evidence:** The GUI calls `scan_user_music_dir()` on the UI thread at **first launch**. It recursively lofty-probes every file under `~/Music` before the window appears (`crates/amp-adapter-gui/src/app_state.rs:222`), and again whenever a file dialog is cancelled. `collect_audio_files` recurses with `path.is_dir()`, which follows symlinks, so loops recurse infinitely (`scanner.rs:48`). There is no progress or cancellation, the extension list is hard-coded and duplicated three times, and there is no watcher despite `watch_filesystem: true`.
**Fix:** A background scanner task with walkdir-style traversal (no symlink following by default, plus an inode visited-set), mtime/size-based incremental rescan, progress events, cancellation, and a `notify`-based watcher. → TASK-149

## DAT-05 — Configuration is never loaded, saved or applied · P1
**Evidence:** The GUI starts from `config: AppConfig::default()` (`crates/amp-adapter-gui/src/app_state.rs:60`). The settings panel mutates it, but nothing persists it (`ConfigStore` is unused), and only `crossfade_duration_ms` affects behaviour. `sample_rate`, `buffer_frames`, `precache_ram_mb`, `loudness_normalization`, `target_lufs`, `headphone_crossfeed`, `enable_*` providers, `music_directories`, `visualizer_fps` and `magnetic_snap_distance` are inert.
**Fix:** Wire `ConfigStore` (it already exists and is atomic) as the single settings source. Each setting gets a consumer or is removed from the UI. → TASK-150

## DAT-06 — Playlist I/O lossy and non-portable · P2
**Evidence:** Relative M3U paths aren't resolved against the playlist location. YouTube and Spotify URIs don't round-trip (SRC-02, SRC-08). `#EXTINF` "Artist - Title" splitting breaks on titles containing " - ". PLS and XSPF aren't supported. Import has no dedupe. The session JSON stores the entire playlist, so a 50k-track `~/Music` produces a multi-MB JSON rewritten on every ingest.
**Fix:** → TASK-152 (formats), TASK-148 (session references library IDs, not full tracks)

## DAT-07 — Tests write to the user's real session file · P1
**Evidence:** `test_drag_and_drop` and `test_playlist_m3u8_export_and_import` (`crates/amp-adapter-gui/src/app.rs:49`) build a real `MultiplayGuiApp`, which loads the developer's real session (or scans the real `~/Music`), and then call `ingest_path` → `save_session()`. That overwrites `~/.multiplay/session_playlist.json` with test data.
**Fix:** Inject `AppPaths` and use temp dirs in tests. → TASK-122
