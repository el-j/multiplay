# PLAN-044: Bandcamp & SoundCloud Resolvers

## Plan Overview
- **ID:** `PLAN-044`
- **Title:** Bandcamp & SoundCloud Resolvers
- **Role:** `integrations`
- **Milestone:** M3 — Finds everything
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-039`, `PLAN-042`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Build the two missing headline sources as separate, feature-gated adapter crates with fixture-based contract tests.

## Objectives
1. **TASK-159** (P1): New `amp-adapter-bandcamp`: Search, Album/Track Parsing, Streams, Purchase Links
2. **TASK-160** (P1): New `amp-adapter-soundcloud`: client_id Discovery, Search, Progressive/HLS Streams
3. **TASK-161** (P2): Recorded-Fixture Contract Tests & Live Canaries for Scraping Adapters

## Tasks
- [ ] [TASK-159: New `amp-adapter-bandcamp`: Search, Album/Track Parsing, Streams, Purchase Links](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-159-bandcamp-adapter.md)
- [ ] [TASK-160: New `amp-adapter-soundcloud`: client_id Discovery, Search, Progressive/HLS Streams](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-160-soundcloud-adapter.md)
- [ ] [TASK-161: Recorded-Fixture Contract Tests & Live Canaries for Scraping Adapters](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-161-scraper-contract-tests-and-canaries.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
