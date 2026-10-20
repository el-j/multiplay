# TASK-219: Shares View: Manage What This Host Shares

## Plan Reference
- Plan: [PLAN-056](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-056-remote-and-sharing-window-host-moderated-requests.md)
- Design: [Signature features design §7](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `frontend_gui`
- Priority: **P2**
- Audit findings: —
- Depends on: TASK-214, TASK-165

## Objectives
1. Shares section lists shared folders, counts and upload activity from the Soulseek share index; add/remove folders; pause sharing.
2. Hidden until PLAN-045 sharing is real (no facade UI).

## Acceptance Criteria (user-visible / measurable)
- Manual: add a folder, see it indexed, remove it, confirm it is no longer offered.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-adapter-gui/src/remote_window.rs`
- `crates/amp-adapter-soulseek/src/`

## Verification Command
`cargo test -p amp-adapter-gui shares`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Evidence (PLAN-056 execution, 2026-10-20)
- Not built on purpose: the Soulseek adapter has no share index yet (PLAN-045), and this task says no facade UI. The Remote window has no Shares section until sharing is real.

## Status
- **Status:** Pending
