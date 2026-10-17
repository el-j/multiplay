# PLAN-042: Universal Search Wired End-to-End

## Plan Overview
- **ID:** `PLAN-042`
- **Title:** Universal Search Wired End-to-End
- **Role:** `frontend_gui, frontend_tui, integrations`
- **Milestone:** M3 — Finds everything
- **Priority:** P0
- **Status:** Pending
- **Dependencies:** `PLAN-037`, `PLAN-041`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Deliver the headline promise: type once, get results from library, YouTube, Bandcamp, SoundCloud and Soulseek, streamed as they arrive.

## Objectives
1. **TASK-153** (P0): Async Omnibox Search in GUI & TUI with Streaming Results
2. **TASK-154** (P1): Per-Provider Status, Timeouts & Error Surfacing
3. **TASK-155** (P2): Cross-Source Ranking & Deduplication

## Tasks
- [ ] [TASK-153: Async Omnibox Search in GUI & TUI with Streaming Results](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-153-async-omnibox-search-in-gui-and-tui.md)
- [ ] [TASK-154: Per-Provider Status, Timeouts & Error Surfacing](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-154-per-provider-status-and-error-surfacing.md)
- [ ] [TASK-155: Cross-Source Ranking & Deduplication](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-155-cross-source-ranking-and-dedupe.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
