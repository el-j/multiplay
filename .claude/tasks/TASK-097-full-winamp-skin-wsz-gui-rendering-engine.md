# TASK-097: Full Winamp Skin (.wsz) GUI Rendering Engine

## Plan Reference
- Plan: [PLAN-030](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-030-native-file-dialogs-playlist-persistence-and-wsz-skins.md)
- Role: `frontend_gui`

## Objectives
1. Wire `SkinLoader` from `amp-core-engine` into `MultiplayGuiApp`.
2. Extract and load authentic Winamp 2.x bitmap textures (`MAIN.BMP`, `CBUTTONS.BMP`, `MONOSTER.BMP`, `POSBAR.BMP`, `VOLUME.BMP`, `TITLEBAR.BMP`, `EQ_EX.BMP`) into egui textures.
3. Parse `viscolor.txt` (spectrum analyzer 24-color gradient palette) and `pledit.txt` (playlist colors and fonts) when a `.wsz` skin is selected.
4. Add skin switcher in GUI settings tab with live hot-reloading.

## Verification Command
`cargo test -p amp-adapter-gui -p amp-core-engine`

## Status
- **Status:** Completed
