# PLAN-039: Decoder & Progressive Streaming Correctness

## Plan Overview
- **ID:** `PLAN-039`
- **Title:** Decoder & Progressive Streaming Correctness
- **Role:** `audio_engine, integrations`
- **Milestone:** M1 — It plays, reliably
- **Priority:** P0
- **Status:** Pending
- **Dependencies:** `PLAN-037`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Make network playback robust: blocking buffered reads, bounded memory, ICY/radio support, range seeking and a hardened decoder.

## Objectives
1. **TASK-135** (P0): Blocking Precache Reader (Condvar) with Timeouts & Cancellation
2. **TASK-136** (P0): Bounded-Memory Streaming: Live Ring Buffer & Disk Spill Cache
3. **TASK-137** (P0): ICY Metadata Demux & PLS/M3U/XSPF Radio Playlist Resolution
4. **TASK-138** (P2): HTTP Range Requests for Seekable Remote Files
5. **TASK-139** (P1): Decoder Hardening: Probe Hints, ResetRequired, Accurate Seek, Opus/WebM

## Tasks
- [ ] [TASK-135: Blocking Precache Reader (Condvar) with Timeouts & Cancellation](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-135-blocking-precache-reader-with-timeouts-and-cancellation.md)
- [ ] [TASK-136: Bounded-Memory Streaming: Live Ring Buffer & Disk Spill Cache](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-136-bounded-memory-streaming-and-disk-cache.md)
- [ ] [TASK-137: ICY Metadata Demux & PLS/M3U/XSPF Radio Playlist Resolution](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-137-icy-metadata-and-radio-playlist-resolution.md)
- [ ] [TASK-138: HTTP Range Requests for Seekable Remote Files](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-138-http-range-seeking.md)
- [ ] [TASK-139: Decoder Hardening: Probe Hints, ResetRequired, Accurate Seek, Opus/WebM](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-139-decoder-hardening-hints-reset-seek-opus.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
