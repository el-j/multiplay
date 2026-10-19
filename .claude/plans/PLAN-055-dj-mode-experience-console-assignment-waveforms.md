# PLAN-055: DJ Mode Experience: Console, Deck Assignment, Waveforms & Controls

## Plan Overview
- **ID:** `PLAN-055`
- **Title:** DJ Mode Experience: Console, Deck Assignment, Waveforms & Controls
- **Role:** `frontend_gui`
- **Milestone:** M3.5 — Signature features
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-053`, `PLAN-054`
- **Design:** [Signature features design §6](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)

## Goal
DJ mode swaps the main window into a deck console without interrupting the music, the single playlist assigns tracks to deck A, B or both, stacked beat-aligned waveforms make beat matching visible, and every control is reachable by mouse and keyboard. Mixing must feel fun on the first try.

## Objectives
1. **TASK-209** (P0): DJ Mode Toggle: Main Window Becomes the Deck Console Without a Gap
2. **TASK-210** (P0): One Playlist, Deck Assignment (A / B / Both) & 'Next on Deck' Markers
3. **TASK-211** (P1): Stacked Beat-Aligned Waveforms, Phase Meter & Overviews
4. **TASK-212** (P1): DJ Controls, Keyboard Map, Cheat Sheet & First-Run Hint
5. **TASK-213** (P0): DJ Verification Harness & Live-Mix Acceptance

## Tasks
- [ ] [TASK-209: DJ Mode Toggle: Main Window Becomes the Deck Console Without a Gap](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-209-dj-mode-swaps-main-window-into-console.md)
- [ ] [TASK-210: One Playlist, Deck Assignment (A / B / Both) & 'Next on Deck' Markers](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-210-playlist-deck-assignment-a-b-both.md)
- [ ] [TASK-211: Stacked Beat-Aligned Waveforms, Phase Meter & Overviews](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-211-beat-aligned-waveforms-and-phase-meter.md)
- [ ] [TASK-212: DJ Controls, Keyboard Map, Cheat Sheet & First-Run Hint](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-212-dj-controls-shortcuts-and-first-run-hint.md)
- [ ] [TASK-213: DJ Verification Harness & Live-Mix Acceptance](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-213-dj-verification-harness-and-mix-acceptance.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
