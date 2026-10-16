# PLAN-026: Multi-Viewport Native Desktop Architecture (True Independent OS Windows)

## Overview & User Requirement
Currently, modular panels (Main Player, Equalizer, Playlist, Preferences, DJ Console) are rendered as internal subwindows (`egui::Window`) trapped inside a single large desktop window with a black background container. The user's explicit directive is:
> *"every window needs to be its very own window not all in one big window. deeply investigate what to do to make this work! so each is really one real application window!"*

To fulfill this, Multiplay GUI must be re-architected using `eframe`/`egui` 0.29's native multi-viewport engine. Each module becomes an authentic, independent operating system window (winit OS native viewport) floating independently on the desktop with OS window decorations, independent positioning, independent drag-and-drop, and magnetic docking.

---

## Technical Architecture

```
                                  [ DESKTOP SCREEN SPACE ]
 ┌────────────────────────────────┐
 │ WINAMP :: MAIN PLAYER (ROOT)   │  <-- eframe::run_native root viewport
 │ [Marquee/VFD] [Visualizer]     │      Inner size: 480x190, Position: (120, 100)
 │ [Scrubber] [Controls] [Toggles]│      Renders directly into egui::CentralPanel
 └──────────────┬─────────────────┘
                │ Magnetic Docking (Delta / ViewportCommand)
 ┌──────────────▼─────────────────┐
 │ WINAMP :: EQUALIZER (VIEWPORT) │  <-- OS Native Window (ViewportId: "win_eq")
 │ [Preamp] [10-Band EQ Sliders]  │      Inner size: 480x155, Position: (120, 290)
 └──────────────┬─────────────────┘      Renders directly into egui::CentralPanel
                │ Magnetic Docking
 ┌──────────────▼─────────────────┐
 │ WINAMP :: PLAYLIST (VIEWPORT)  │  <-- OS Native Window (ViewportId: "win_playlist")
 │ [Files/Folder Scan] [Filters]  │      Inner size: 480x260, Resizable
 │ [Track Rows] [Drop Zone]       │      Accepts Native OS File Drag & Drop
 └────────────────────────────────┘

 ┌────────────────────────────────┐  ┌────────────────────────────────┐
 │ MULTIPLAY :: PREFERENCES       │  │ WINAMP :: DJ CONSOLE           │
 │ (OS Native Window)             │  │ (OS Native Window)             │
 │ [Tabs: Folders, Audio, P2P]    │  │ [Deck A/B, Crossfader, CUE]    │
 └────────────────────────────────┘  └────────────────────────────────┘
```

1. **Root Viewport = Main Player Window:**
   - The native OS window created by `eframe::run_native` is configured to the exact dimensions of the Main Player (`480x190`).
   - Sized compactly with title `"WINAMP :: MAIN PLAYER"`.
   - The root `CentralPanel` directly hosts the Main Player faceplate widgets. No outer desktop container or black filler frame exists.
2. **Child Viewports = Modular Satellite Windows:**
   - Spawned via `ctx.show_viewport_immediate(ViewportId, ViewportBuilder, |ctx, _class| { ... })`.
   - Each satellite window is a true OS native window managed by the operating system window manager.
   - Closing a satellite window via its OS close button or toggle button disposes the viewport; reopening spawns it back into view.
3. **Cross-Viewport Coordinate Tracking & Magnetic Docking:**
   - Positions are tracked in screen coordinates via `ctx.input(|i| i.viewport().outer_rect)`.
   - Moving the Main Player moves transitively docked windows via `ViewportCommand::OuterPosition`.
   - Proximity snapping activates when windows are within the magnetic docking threshold.
4. **Independent Drag-and-Drop Ingestion:**
   - Audio files and folders can be dragged and dropped from macOS Finder / OS desktop onto the Main Player OR the Playlist window directly.

---

## Tasks Breakdown

- **TASK-079**: Multi-Viewport Native Window Architecture & Root Main Player Window Configuration
- **TASK-080**: Satellite Windows OS Native Viewport Immediate Dispatch & Close Lifecycle Handling
- **TASK-081**: Cross-Viewport Coordinate Tracking, Magnetic Snapping & Group Movement Synchronization
- **TASK-082**: Comprehensive Testing, Verification, Zero Clippy Warnings, and Build Packaging

---

## Verification & Definition of Done
1. Zero C/C++ dynamic or static linkage (`./scripts/verify_pure_rust.sh` passes 100%).
2. `#![forbid(unsafe_code)]` preserved across all crates.
3. Every modular panel is rendered in its own real OS application window on the desktop.
4. All source files strictly $\le 300$ lines.
5. `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass with 0 errors and 0 warnings.
6. `make all` passes and produces valid standalone executables and application bundles.
