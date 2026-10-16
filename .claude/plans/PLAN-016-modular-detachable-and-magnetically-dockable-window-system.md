---
id: PLAN-016
title: Modular Detachable & Magnetically Dockable Window System
status: completed
role: frontend_gui
tasks:
  - TASK-051
  - TASK-052
  - TASK-053
---

# PLAN-016: Modular Detachable & Magnetically Dockable Window System

## Objective
Implement a modular, multi-window desktop GUI system where the Main Player, Equalizer, Playlist, Settings, and DJ Console are independent, draggable, resizable windows that magnetically snap and dock together flush at adjacent borders, support group movement when docked, and preserve layout across sessions.

## Architecture Context
- **Window Management (`amp-adapter-gui`)**:
  - Convert monolithic single-panel layout into independent `egui::Window` modules.
  - Dedicated windows: `MainWindow`, `EqualizerWindow`, `PlaylistWindow`, `SettingsWindow`, and `DjConsoleWindow`.
- **Magnetic Snapping Geometry Engine**:
  - Bounding box intersection and edge proximity detection ($\le 14$ px snap threshold).
  - Docked group propagation: dragging an anchor window moves docked companion windows.
  - Separation / tear-off threshold: dragging a docked window away beyond threshold snaps it free.
- **Layout State Persistence**:
  - Save window rects, open/closed states, shaded states, and dock relationships in `AppConfig`.

## Tasks
1. [TASK-051](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-051-magnetic-snapping-and-window-docking-geometry-engine.md): Magnetic Snapping & Window Docking Geometry Engine
2. [TASK-052](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-052-modular-windows-player-eq-playlist-settings-dj.md): Modular Windows Implementation (Player, Equalizer, Playlist, Settings, DJ Console)
3. [TASK-053](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-053-window-layout-persistence-and-session-restore.md): Window Layout Persistence & Session Restore
