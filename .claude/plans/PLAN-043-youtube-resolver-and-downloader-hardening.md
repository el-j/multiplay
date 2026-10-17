# PLAN-043: YouTube Resolver & Offline Downloader Hardening

## Plan Overview
- **ID:** `PLAN-043`
- **Title:** YouTube Resolver & Offline Downloader Hardening
- **Role:** `integrations`
- **Milestone:** M3 — Finds everything
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-039`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Make YouTube playback dependable and make downloading work for any resolvable source.

## Objectives
1. **TASK-156** (P1): YouTube URL/ID Normalization (watch, youtu.be, shorts, music, playlists)
2. **TASK-157** (P0): YouTube Format Selection, URL Refresh, Retry & Daily Canary
3. **TASK-158** (P1): Working Offline Downloader (Async, Correct Containers, Safe Names, Progress)

## Tasks
- [ ] [TASK-156: YouTube URL/ID Normalization (watch, youtu.be, shorts, music, playlists)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-156-youtube-url-and-id-normalization.md)
- [ ] [TASK-157: YouTube Format Selection, URL Refresh, Retry & Daily Canary](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-157-youtube-format-selection-retry-and-canary.md)
- [ ] [TASK-158: Working Offline Downloader (Async, Correct Containers, Safe Names, Progress)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-158-working-offline-downloader.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
