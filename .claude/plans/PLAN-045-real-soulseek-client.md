# PLAN-045: Real Soulseek Client: Session, Search, Transfers & Sharing

## Plan Overview
- **ID:** `PLAN-045`
- **Title:** Real Soulseek Client: Session, Search, Transfers & Sharing
- **Role:** `integrations, distributed_systems`
- **Milestone:** M3 — Finds everything
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-039`, `PLAN-041`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Replace the in-memory facade with a protocol-correct Soulseek client — the product's flagship differentiator — including being a good citizen of the network by sharing.

## Objectives
1. **TASK-162** (P1): Soulseek Server Session: Login, Keepalive, Reconnect, Correct Message Namespaces
2. **TASK-163** (P1): Soulseek Search & Peer Connections (Direct + Firewalled)
3. **TASK-164** (P1): Soulseek Transfers: Queue, F-Connection, Resume & Progressive Play
4. **TASK-165** (P1): Soulseek Sharing: Share Index, Browse/Search Responses, Upload Slots & Limits
5. **TASK-166** (P2): Soulseek UI: Account Settings, Transfer Manager, User Browse

## Tasks
- [ ] [TASK-162: Soulseek Server Session: Login, Keepalive, Reconnect, Correct Message Namespaces](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-162-soulseek-server-session.md)
- [ ] [TASK-163: Soulseek Search & Peer Connections (Direct + Firewalled)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-163-soulseek-search-and-peer-connections.md)
- [ ] [TASK-164: Soulseek Transfers: Queue, F-Connection, Resume & Progressive Play](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-164-soulseek-transfers-and-progressive-play.md)
- [ ] [TASK-165: Soulseek Sharing: Share Index, Browse/Search Responses, Upload Slots & Limits](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-165-soulseek-sharing-and-uploads.md)
- [ ] [TASK-166: Soulseek UI: Account Settings, Transfer Manager, User Browse](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-166-soulseek-ui.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
