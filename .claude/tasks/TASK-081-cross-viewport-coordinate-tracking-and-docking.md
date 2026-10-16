# TASK-081: Cross-Viewport Coordinate Tracking, Magnetic Snapping & Group Movement Synchronization

## Plan Reference
- Plan: [PLAN-026](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-026-multi-viewport-native-desktop-architecture.md)
- Role: `gui_engineering`
- Status: Completed

## Objectives
1. Track real OS window positions in screen space using `ctx.input(|i| i.viewport().outer_rect)`.
2. Manage viewport initialization so windows spawn at saved/docked positions without fighting user dragging on subsequent frames.
3. Synchronize movement across magnetically docked windows: when the Main Player is moved by the user, send `ViewportCommand::OuterPosition` to docked satellite viewports so they move together smoothly as a cluster.
4. Calculate magnetic docking snaps when satellite windows are dragged close to adjacent windows, and update docking bonds accordingly.

## Verification Command
`cargo test -p amp-adapter-gui test_docking`
