# TASK-079: Multi-Viewport Native Window Architecture & Root Main Player Window Configuration

## Plan Reference
- Plan: [PLAN-026](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-026-multi-viewport-native-desktop-architecture.md)
- Role: `gui_engineering`
- Status: Completed

## Objectives
1. Configure `eframe::NativeOptions` in `crates/amp-adapter-gui/src/main.rs` so that the root native window is sized specifically for the Main Player (480x190), titled `"WINAMP :: MAIN PLAYER"`, positioned initially at (120, 100).
2. Refactor `crates/amp-adapter-gui/src/app_render.rs` so that the root viewport's `egui::CentralPanel` hosts `render_main_window_content(self, ui)` directly, eliminating the outer black desktop frame.
3. Ensure the root viewport receives OS native drag-and-drop file/directory events and handles gapless playback advancement.

## Verification Command
`cargo check -p amp-adapter-gui && cargo test -p amp-adapter-gui`
