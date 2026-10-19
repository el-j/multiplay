# TASK-199: Persist & Restore Window Layout, Docking and Visibility

## Plan Reference
- Plan: [PLAN-053](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-053-classic-self-drawn-windows-and-magnetic-docking.md)
- Design: [Signature features design §4](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-197, TASK-150

## Objectives
1. Persist positions, dock graph, shade, visibility, playlist size and double-size through the config store (TASK-150); replaces the ad-hoc layout file in `layout_manager.rs`.
2. On start, clamp layouts outside current monitors back on screen with the group intact.
3. Versioned schema with migration from the current layout file.

## Acceptance Criteria (user-visible / measurable)
- Quit and relaunch restores the exact layout on macOS, Windows and X11.
- Test: layout saved on a disconnected second monitor reopens fully visible on the primary.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/layout_manager.rs`
- `crates/amp-domain/src/config.rs`

## Verification Command
`cargo test -p amp-adapter-gui layout`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
