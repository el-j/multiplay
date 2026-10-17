# TASK-096: Session Playlist Persistence & M3U8 Export/Import

## Plan Reference
- Plan: [PLAN-030](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-030-native-file-dialogs-playlist-persistence-and-wsz-skins.md)
- Role: `storage_and_state`

## Objectives
1. Automatically persist playlist state (tracks, order, active index, volume, EQ gains) to `~/.multiplay/session_playlist.json` on changes or shutdown.
2. Restore active playlist state seamlessly when starting `multiplay-gui` or `multiplay`.
3. Support exporting and importing standard `.m3u` / `.m3u8` playlist files with UTF-8 encoding and EXTINF metadata tags.
4. Add comprehensive unit and integration tests.

## Verification Command
`cargo test -p amp-adapter-gui -p amp-adapter-storage`

## Status
- **Status:** Completed
