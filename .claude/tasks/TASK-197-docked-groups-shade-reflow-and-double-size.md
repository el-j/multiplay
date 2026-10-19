# TASK-197: Docked Groups, Group Move, Shade/Resize Reflow & Double Size

## Plan Reference
- Plan: [PLAN-053](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-053-classic-self-drawn-windows-and-magnetic-docking.md)
- Design: [Signature features design §4](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-196

## Objectives
1. Dock graph derived from edge contact, transitive from the main window (main → EQ → playlist chains).
2. Dragging main moves the whole group rigidly; dragging any other window detaches only it and re-docks on contact.
3. Shade (title-bar double-click) and playlist resize move windows docked below/right so the group stays closed; unshade restores.
4. Double size (Ctrl+D) scales all dockable windows and their geometry 2x around the main window.
5. No-overlap guarantee (owner decision 2026-10-19): a dragged window stops flush against windows
   it would cover; showing a window, restoring a layout, unshading or resizing pushes overlapping
   windows out of the way.

## Acceptance Criteria (user-visible / measurable)
- Unit tests: group membership, rigid group move, detach, reflow on shade/resize, 2x scaling.
- Property test: after any drag, resize, show or shade sequence, no two visible windows overlap.
- Manual: main + EQ + playlist docked in a column move together; pulling the EQ out leaves main and playlist docked if they still touch.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/docking/manager.rs`
- `crates/amp-adapter-gui/src/docking/groups.rs (new)`

## Verification Command
`cargo test -p amp-adapter-gui docking`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

- 2026-10-19: no-overlap guarantee implemented: `docking::reflow` (overlaps, separate_from, attached_beyond, clamp_growth, resize_with_attached, push_apart; 6 tests incl. a resize-sequence no-overlap check). Dragged groups stop flush instead of landing on a window (drag test). Shade/unshade moves the docked column below with the edge. Newly shown windows open clear of others at least at their minimum size.
- Open: double size (Ctrl+D); property test over random drag/show/shade sequences.

## Status
- **Status:** Pending
