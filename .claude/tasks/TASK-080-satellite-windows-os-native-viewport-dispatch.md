# TASK-080: Satellite Windows OS Native Viewport Immediate Dispatch & Close Lifecycle Handling

## Plan Reference
- Plan: [PLAN-026](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-026-multi-viewport-native-desktop-architecture.md)
- Role: `gui_engineering`
- Status: Completed

## Objectives
1. Implement native OS window rendering for each satellite panel using `ctx.show_viewport_immediate`:
   - Equalizer: `ViewportId::from_hash_of("win_eq")`, titled `"WINAMP :: EQUALIZER"`, size 480x155.
   - Playlist: `ViewportId::from_hash_of("win_playlist")`, titled `"WINAMP :: PLAYLIST"`, size 480x260, resizable.
   - Preferences: `ViewportId::from_hash_of("win_settings")`, titled `"MULTIPLAY :: PREFERENCES"`, size 480x360, resizable.
   - DJ Console: `ViewportId::from_hash_of("win_dj")`, titled `"WINAMP :: DJ CONSOLE"`, size 480x220, resizable.
2. In each satellite window's immediate callback, render content directly into its own `egui::CentralPanel` with Winamp styling.
3. Wire the OS native close button (`ctx.input(|i| i.viewport().close_requested())`) to toggle visibility off in `dock_manager`, properly destroying the OS window when closed and allowing toggling back on from the Main Player.
4. Support direct drag-and-drop of audio files and folders onto the native Playlist window.

## Verification Command
`cargo check -p amp-adapter-gui && cargo test -p amp-adapter-gui`
