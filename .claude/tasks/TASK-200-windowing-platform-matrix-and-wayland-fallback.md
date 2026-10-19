# TASK-200: Windowing Platform Matrix & Wayland Fallback

## Plan Reference
- Plan: [PLAN-053](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-053-classic-self-drawn-windows-and-magnetic-docking.md)
- Design: [Signature features design §4](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-196

## Objectives
1. Prefer the winit X11 backend (XWayland) when `DISPLAY` is set, via eframe's event-loop builder hook.
2. Pure Wayland: windows open independently, snapping disabled, one-line notice in Settings.
3. Manual test matrix: macOS (arm64), Windows 11, Ubuntu X11, Ubuntu Wayland session; multi-monitor with different scale factors.

## Acceptance Criteria (user-visible / measurable)
- Matrix recorded in the task evidence with screenshots per platform.
- No crash or invisible window on any listed platform.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/main.rs`
- `docs/adr/`

## Verification Command
`cargo test -p amp-adapter-gui && ./scripts/verify_pure_rust.sh`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
