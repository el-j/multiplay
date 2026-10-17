# TASK-095: Native OS File & Directory Selection Dialogs

## Plan Reference
- Plan: [PLAN-030](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-030-native-file-dialogs-playlist-persistence-and-wsz-skins.md)
- Role: `frontend_gui`

## Objectives
1. Implement real native OS file and folder picker dialogs for both GUI (`amp-adapter-gui`) and TUI (`amp-app`).
2. On macOS, use native system script or pure-Rust API without introducing C compiler dependencies (`cc`, `cmake`, `pkg-config`).
3. Replace the `~/Music` fallback in `MultiplayGuiApp::open_file_dialog` and `open_folder_dialog` with the native dialog picker.
4. Support selecting multiple audio files (.mp3, .flac, .wav, .aac, .ogg, .m4a) and importing entire directory hierarchies.

## Verification Command
`cargo test -p amp-adapter-gui && ./scripts/verify_pure_rust.sh`

## Status
- **Status:** Pending
