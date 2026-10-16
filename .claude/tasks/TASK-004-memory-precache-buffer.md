---
id: TASK-004
title: In-Memory Precaching Stream Buffer
planId: PLAN-002
status: completed
role: audio_engine
verificationCommand: cargo test -p amp-core-engine test_precache
---

# TASK-004: In-Memory Precaching Stream Buffer

## Description
Implement `SharedPrecacheBuffer` in `crates/amp-core-engine/src/precache.rs`:
- Implements `std::io::Read` and `std::io::Seek` over an in-memory byte buffer.
- Allows background asynchronous tasks to push chunks via `push_chunk(&[u8])` and signal completion via `mark_finished()`.
- Handles underflow cleanly: when `available == 0` and stream has not ended, returns `io::ErrorKind::WouldBlock` so Symphonia yields instead of aborting.
- Thread-safe via `Arc<RwLock<PrecacheBufferState>>`.

## Verification
```bash
cargo test -p amp-core-engine test_precache
```
