# ADR-002: Classic Self-Drawn Windows with App-Driven Magnetic Docking

- **Status:** Accepted (TASK-195, TASK-196)
- **Date:** 2026-10-19
- **Design:** [Signature features design §4](../superpowers/specs/2026-10-19-signature-features-design.md)

## Context

The GUI used OS-decorated, resizable egui viewports. Snapping ran only after the mouse was
released, and on macOS the OS title-bar drag hides pointer state from the app, so it often did
not run at all. The goal is the Winamp 2 behaviour: windows that draw their own chrome, snap
while being dragged, and move as a docked group with the main window.

## Decision

1. **Frameless viewports.** Every window is created through `chrome::classic_viewport`:
   no decorations, no maximize, not resizable (the playlist keeps OS resizing only until the
   stepped grip of TASK-198). Satellites are hidden from the taskbar.
2. **Self-drawn title strip.** `chrome::title_bar` draws a 14 pt classic strip with minimize,
   shade and close buttons; hit-testing lives in the egui-free `chrome::layout`.
3. **The app drags, not the OS.** A press on the strip starts a `chrome::drag::DragSession`
   in screen space (window origin + local pointer). Every frame the session computes new
   positions, snapped by `docking::snap::snap_rect`, and sends `ViewportCommand::OuterPosition`
   to each member viewport. `ViewportCommand::StartDrag` is not used, because an OS drag
   cannot be snapped while it runs.
4. **Groups from geometry.** Windows that share an edge are docked. Dragging the main window
   moves its whole touching group (`docking::snap::connected_group`); dragging any other
   window detaches it. After each drag the dock bonds are rebuilt from contact.
5. **Pure, tested geometry.** Snapping, touching, grouping and hit-testing have no egui types
   and are unit-tested; the egui layer only translates input and sends commands.

## Consequences

- Live snapping works the same on macOS, Windows and X11. Wayland cannot position windows;
  TASK-200 prefers XWayland and documents the fallback.
- egui reports only the current monitor's size, not its origin, so screen-edge snapping
  assumes the window's monitor starts at (0, 0) until TASK-200 adds monitor origins.
- Shade currently resizes only the shaded window; reflowing windows docked below it is
  TASK-197. Classic sizes and skin title sprites follow with TASK-198.
