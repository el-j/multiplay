# PLAN-053: Classic Self-Drawn Windows & Live Magnetic Docking

## Plan Overview
- **ID:** `PLAN-053`
- **Title:** Classic Self-Drawn Windows & Live Magnetic Docking
- **Role:** `frontend_gui`
- **Milestone:** M3.5 — Signature features
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-036` (may start in parallel with M1)
- **Design:** [Signature features design §4](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)

## Goal
Every window draws its own classic chrome (no OS title bars, no maximize/fullscreen), fixed classic sizes with stepped playlist resizing, and live Winamp 2-style magnetic docking: snap while dragging, docked groups move with the main window, shade/resize reflow, persisted layout. Settings becomes a one-time overlay.

## Objectives
1. **TASK-195** (P0): ADR & Self-Drawn Classic Window Chrome (Frameless, No Maximize)
2. **TASK-196** (P0): Live App-Driven Drag & Magnetic Snap Engine (Windows + Screen Edges)
3. **TASK-197** (P1): Docked Groups, Group Move, Shade/Resize Reflow & Double Size
4. **TASK-198** (P1): Meaningful Resizing & Responsive, Never-Clipping Window Content
5. **TASK-199** (P1): Persist & Restore Window Layout, Docking and Visibility
6. **TASK-200** (P1): Windowing Platform Matrix & Wayland Fallback
7. **TASK-201** (P1): Settings as a One-Time Overlay Window

## Tasks
- [ ] [TASK-195: ADR & Self-Drawn Classic Window Chrome (Frameless, No Maximize)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-195-adr-and-self-drawn-classic-window-chrome.md)
- [ ] [TASK-196: Live App-Driven Drag & Magnetic Snap Engine (Windows + Screen Edges)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-196-live-drag-and-magnetic-snap-engine.md)
- [ ] [TASK-197: Docked Groups, Group Move, Shade/Resize Reflow & Double Size](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-197-docked-groups-shade-reflow-and-double-size.md)
- [ ] [TASK-198: Meaningful Resizing & Responsive, Never-Clipping Window Content](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-198-classic-fixed-sizes-and-stepped-resize.md)
- [ ] [TASK-199: Persist & Restore Window Layout, Docking and Visibility](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-199-persist-and-restore-window-layout.md)
- [ ] [TASK-200: Windowing Platform Matrix & Wayland Fallback](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-200-windowing-platform-matrix-and-wayland-fallback.md)
- [ ] [TASK-201: Settings as a One-Time Overlay Window](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-201-settings-one-time-overlay.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
