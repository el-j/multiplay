# PLAN-030: Native File Dialogs, Playlist Persistence & Classic Winamp Skin (.wsz) Engine

## Plan Overview
- **ID:** `PLAN-030`
- **Title:** Native File Dialogs, Playlist Persistence & Classic Winamp Skin (.wsz) Engine
- **Role:** `frontend_gui` & `storage_and_state`
- **Status:** Completed
- **Dependencies:** `PLAN-028`, `PLAN-029`

## Objectives
1. **TASK-095**: Native OS File & Directory Selection Dialogs — Replace the `~/Music` fallback with real OS file and folder selection dialogs without C compiler or OpenSSL linkage.
2. **TASK-096**: Session Playlist Persistence & M3U8 Export/Import — Save and restore active playlist, cursor position, and track playback state across app restarts. Support loading and exporting standard `.m3u` / `.m3u8` playlists.
3. **TASK-097**: Full Winamp Skin (.wsz) GUI Rendering Engine — Wire `SkinLoader` to `MultiplayGuiApp`: support loading `.wsz` files, extracting bitmap textures, `viscolor.txt`, and `pledit.txt`, dynamically skinning player windows.
4. **TASK-098**: Dual Oscilloscope & Spectrum Visualizer Mode — Add toggleable real-time phosphor oscilloscope and peak-decay spectrum analyzer.

## Tasks
- [x] [TASK-095: Native OS File & Directory Selection Dialogs](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-095-native-os-file-and-directory-dialogs.md)
- [x] [TASK-096: Session Playlist Persistence & M3U8 Export/Import](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-096-session-playlist-persistence-and-m3u8-support.md)
- [x] [TASK-097: Full Winamp Skin (.wsz) GUI Rendering Engine](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-097-full-winamp-skin-wsz-gui-rendering-engine.md)
- [x] [TASK-098: Dual Oscilloscope & Spectrum Visualizer Mode](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-098-dual-oscilloscope-and-spectrum-visualizer-mode.md)

## Verification
`make all`
