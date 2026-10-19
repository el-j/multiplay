# TASK-195: ADR & Self-Drawn Classic Window Chrome (Frameless, No Maximize)

## Plan Reference
- Plan: [PLAN-053](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-053-classic-self-drawn-windows-and-magnetic-docking.md)
- Design: [Signature features design §4](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P0**
- Audit findings: —
- Depends on: — (coordinate with TASK-194)

## Objectives
1. ADR `docs/adr/ADR-00x-classic-windowing.md`: frameless egui viewports, app-driven drag, geometry module boundaries, platform matrix (spec §4).
2. All dockable viewports and the main window: `with_decorations(false)`, `with_resizable(false)`, maximize disabled, macOS titlebar hidden; no OS fullscreen path.
3. Shared `ClassicChrome` widget: title bar (active/inactive), minimize, shade and close buttons, drawn from an original built-in theme or from `.wsz` sprites when a skin is loaded.
4. Original default artwork in the classic style; no 'Winamp' text or copied assets (TASK-194).

## Acceptance Criteria (user-visible / measurable)
- No window in the GUI shows an OS title bar, border, maximize or fullscreen control on macOS, Windows and Linux X11.
- Close/minimize/shade buttons work on every dockable window; Esc never closes the main window.
- Snapshot/unit tests cover chrome hit-testing (button rects at 1x and 2x).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/viewports.rs`
- `crates/amp-adapter-gui/src/main.rs`
- `crates/amp-adapter-gui/src/chrome/ (new)`
- `crates/amp-adapter-gui/src/skin_texture.rs`
- `docs/adr/`

## Verification Command
`cargo test -p amp-adapter-gui chrome`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

- 2026-10-19 (macOS arm64, debug build): all windows frameless with the self-drawn classic strip (grip ridges, centered title, minimize/shade/close; gold when focused, grey when not). No OS title bar, maximize or fullscreen. Owner screenshot shows main, playlist and EQ docked flush in a column.
- Implemented: `chrome::classic_viewport`, `chrome::title_bar`, `chrome::apply_action` (close quits on main, hides satellites; minimize; shade to 14 pt), egui-free hit-testing `chrome::layout` (4 tests, 1x and 2x). ADR: `docs/adr/ADR-002-classic-windowing.md`. Window titles no longer say WINAMP.
- Open: `.wsz` title sprites (needs classic 275 px widths, TASK-198); content strings still say WINAMP (TASK-194); Windows/X11 manual check (TASK-200).

## Status
- **Status:** Pending
