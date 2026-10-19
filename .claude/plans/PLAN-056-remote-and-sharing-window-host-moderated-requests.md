# PLAN-056: Remote & Sharing Window: Host-Moderated Requests

## Plan Overview
- **ID:** `PLAN-056`
- **Title:** Remote & Sharing Window: Host-Moderated Requests
- **Role:** `frontend_gui, security, integrations`
- **Milestone:** M3.5 — Signature features
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-035`, `PLAN-053`, M3 resolvers and search (`PLAN-042`, `PLAN-043`, `PLAN-044`)
- **Design:** [Signature features design §7](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)

## Goal
A dockable, playlist-style Remote & Sharing window where the host sees paired phones, approves or rejects their track requests into the playlist or a deck, and manages shares. Requests carry a source reference; the host resolves and plays it itself. Phone audio upload exists only as an explicit opt-in.

## Objectives
1. **TASK-214** (P0): Remote & Sharing Window (Dockable, Playlist-Style): Requests, Devices, Shares
2. **TASK-215** (P0): Remote Protocol v2: RequestTrack with Allow-Listed References
3. **TASK-216** (P0): Host Moderation: Accept to Playlist / Play Next / Deck, Reject, Drag & Trusted Devices
4. **TASK-217** (P1): Phone Web Client: Search, Paste Link, Request Status
5. **TASK-218** (P2): Opt-In Phone Audio Upload (Off by Default)
6. **TASK-219** (P2): Shares View: Manage What This Host Shares

## Tasks
- [ ] [TASK-214: Remote & Sharing Window (Dockable, Playlist-Style): Requests, Devices, Shares](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-214-remote-and-sharing-window.md)
- [ ] [TASK-215: Remote Protocol v2: RequestTrack with Allow-Listed References](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-215-remote-protocol-v2-track-requests.md)
- [ ] [TASK-216: Host Moderation: Accept to Playlist / Play Next / Deck, Reject, Drag & Trusted Devices](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-216-host-request-moderation-to-playlist-and-decks.md)
- [ ] [TASK-217: Phone Web Client: Search, Paste Link, Request Status](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-217-phone-web-client-search-and-request.md)
- [ ] [TASK-218: Opt-In Phone Audio Upload (Off by Default)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-218-opt-in-phone-audio-upload.md)
- [ ] [TASK-219: Shares View: Manage What This Host Shares](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-219-shares-management-view.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
