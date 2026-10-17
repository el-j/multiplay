# PLAN-041: Persistent Library, Configuration & Secrets

## Plan Overview
- **ID:** `PLAN-041`
- **Title:** Persistent Library, Configuration & Secrets
- **Role:** `storage_and_state`
- **Milestone:** M1 — It plays, reliably
- **Priority:** P0
- **Status:** Pending
- **Dependencies:** `PLAN-036`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
One platform-correct home for all state; a durable, versioned library with stable IDs and a background scanner; settings that persist and take effect; real secret storage.

## Objectives
1. **TASK-147** (P1): Platform AppPaths Service (config/data/cache/log) & Migration
2. **TASK-148** (P0): Durable, Versioned Library DB with Stable Track IDs
3. **TASK-149** (P1): Background Incremental Scanner, Symlink-Safe, with File Watcher
4. **TASK-150** (P1): Wire ConfigStore: Persist Settings & Apply (or Remove) Every Option
5. **TASK-151** (P1): OS Keychain Credential Vault (with Encrypted-File Fallback)
6. **TASK-152** (P2): Lossless Playlist I/O: M3U/M3U8 (relative paths), PLS, XSPF

## Tasks
- [ ] [TASK-147: Platform AppPaths Service (config/data/cache/log) & Migration](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-147-platform-app-paths-service-and-migration.md)
- [ ] [TASK-148: Durable, Versioned Library DB with Stable Track IDs](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-148-durable-versioned-library-db-with-stable-ids.md)
- [ ] [TASK-149: Background Incremental Scanner, Symlink-Safe, with File Watcher](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-149-background-incremental-scanner-and-watcher.md)
- [ ] [TASK-150: Wire ConfigStore: Persist Settings & Apply (or Remove) Every Option](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-150-wire-config-store-and-apply-every-setting.md)
- [ ] [TASK-151: OS Keychain Credential Vault (with Encrypted-File Fallback)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-151-os-keychain-credential-vault.md)
- [ ] [TASK-152: Lossless Playlist I/O: M3U/M3U8 (relative paths), PLS, XSPF](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-152-lossless-playlist-io-m3u-pls-xspf.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
