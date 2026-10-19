# TASK-198: Meaningful Resizing & Responsive, Never-Clipping Window Content

## Plan Reference
- Plan: [PLAN-053](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-053-classic-self-drawn-windows-and-magnetic-docking.md)
- Design: [Signature features design §4](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-195

## Objectives
1. App-driven resize for every window except via OS: bottom-right grip plus right and bottom edge
   zones; per-window minimum sizes (spec §4 table); resized edge snaps to neighbour edges.
2. Responsive layouts: main (truncating title, device dropdown, icon-only transport when narrow,
   toggles on their own row, visualizer fills), equalizer (sliders scale with height, columns
   share width, short labels, preset dropdown), playlist (list fills height, wrapping/compact
   toolbar, source dropdown, truncating rows, badge hides when narrow), settings (wrapping tabs,
   scrolling content), DJ console (scrolls until PLAN-055).
3. Runtime guard: a window whose content does not fit grows to fit instead of clipping.
4. Owner decision 2026-10-19 supersedes the classic fixed 275x116 sizes.

## Acceptance Criteria (user-visible / measurable)
- Offscreen egui test renders every window at its minimum size and at 300/400/600/900 pt widths;
  no content rect exceeds the window (zero overflow).
- No text is cut mid-glyph: long titles show an ellipsis.
- Resizing never makes windows overlap (unit tests on resize sessions).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/chrome/resize.rs (new)`
- `crates/amp-adapter-gui/src/main_window.rs`, `playlist_window.rs`, `equalizer_widget.rs`, `settings_panel.rs`
- `crates/amp-adapter-gui/tests/responsive_layout.rs (new)`

## Verification Command
`cargo test -p amp-adapter-gui responsive`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

- 2026-10-19 (macOS arm64, debug build): main 480x210, EQ 480x200, playlist 480x300 render fully inside their windows (window captures); grip drawn bottom-right on every resizable window.
- Implemented: per-window minimums (`chrome::resize::min_inner_size`), app-driven resize from right edge, bottom edge and grip (`chrome::handles`, `chrome::resize::ResizeSession`; moving edge snaps to neighbour/screen edges; 5 tests), responsive layouts for main, EQ, playlist, settings (all tabs) and DJ (scrolls), text truncates by default, runtime grow-to-fit guard.
- Automated: `tests/responsive_layout.rs` renders every window offscreen at its minimum and 300/400/600/900 pt widths x 3 heights, every settings tab, and the empty playlist: zero overflow (4 tests). Before the change: main overflowed by 727 pt, playlist by 303 pt.
- Open: interactive resize recording on macOS/Windows (no input automation available in this session); double-size scaling (TASK-197).

## Status
- **Status:** Pending
