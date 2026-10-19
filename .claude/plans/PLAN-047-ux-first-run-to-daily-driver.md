# PLAN-047: UX: From First Run to Daily Driver

## Plan Overview
- **ID:** `PLAN-047`
- **Title:** UX: From First Run to Daily Driver
- **Role:** `frontend_gui, frontend_tui`
- **Milestone:** M4 — Feels great
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-037`, `PLAN-041`, `PLAN-042`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
The experience people fall in love with: fast first launch, a real library with artwork, OS media integration, clear feedback, and a TUI that is a complete player.

## Objectives
1. **TASK-170** (P1): Remove Demo Data & Show Real Stream Info
2. **TASK-171** (P1): First-Run Onboarding Wizard
3. **TASK-172** (P1): Library Browser (Artists/Albums/Tracks) & Artwork
4. **TASK-173** (P1): OS Media Keys & Now Playing (MPRIS / macOS / SMTC)
5. **TASK-174** (P1): TUI as a Complete Player
6. **TASK-175** (P1): GUI Feedback, Shortcuts, Seek-on-Release & Accessibility
7. **TASK-176** (P2): DJ Mode: Experimental Flag, Own Engine Thread, Per-Deck Resampling — superseded by PLAN-054/PLAN-055

## Tasks
- [ ] [TASK-170: Remove Demo Data & Show Real Stream Info](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-170-remove-demo-data-show-real-track-info.md)
- [ ] [TASK-171: First-Run Onboarding Wizard](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-171-first-run-onboarding.md)
- [ ] [TASK-172: Library Browser (Artists/Albums/Tracks) & Artwork](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-172-library-browser-and-artwork.md)
- [ ] [TASK-173: OS Media Keys & Now Playing (MPRIS / macOS / SMTC)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-173-os-media-keys-and-now-playing.md)
- [ ] [TASK-174: TUI as a Complete Player](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-174-tui-complete-player.md)
- [ ] [TASK-175: GUI Feedback, Shortcuts, Seek-on-Release & Accessibility](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-175-gui-feedback-shortcuts-accessibility.md)
- [ ] [TASK-176: DJ Mode: Experimental Flag, Own Engine Thread, Per-Deck Resampling](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-176-dj-mode-stabilization-or-experimental-flag.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
