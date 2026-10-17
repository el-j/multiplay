# PLAN-046: P2P Mesh & Spotify Scope Decisions; Remove Facades

## Plan Overview
- **ID:** `PLAN-046`
- **Title:** P2P Mesh & Spotify Scope Decisions; Remove Facades
- **Role:** `architecture, integrations`
- **Milestone:** M3 — Finds everything
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-034`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Make explicit build-or-cut decisions for the P2P mesh and Spotify, and remove every facade adapter from production wiring meanwhile.

## Objectives
1. **TASK-167** (P1): ADR: P2P Mesh — Implement with libp2p or Cut from v1
2. **TASK-168** (P1): ADR: Spotify — Drop Playback, Offer Web-API Playlist Import & Cross-Source Matching
3. **TASK-169** (P0): Remove/Feature-Gate Facade Providers from Production Builds

## Tasks
- [ ] [TASK-167: ADR: P2P Mesh — Implement with libp2p or Cut from v1](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-167-adr-p2p-mesh-build-or-cut.md)
- [ ] [TASK-168: ADR: Spotify — Drop Playback, Offer Web-API Playlist Import & Cross-Source Matching](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-168-adr-spotify-scope.md)
- [ ] [TASK-169: Remove/Feature-Gate Facade Providers from Production Builds](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-169-remove-facade-providers-from-production.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
