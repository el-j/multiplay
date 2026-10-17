---
id: TASK-064
title: Pure-Rust Soulseek Protocol Search & Message Frame State Machine
planId: PLAN-021
status: re-opened
role: integrations
auditFindings: ["ORC-01", "SRC-09"]
mappedTo: ["PLAN-045", "TASK-162", "TASK-163"]
verificationCommand: cargo test -p amp-adapter-soulseek
---

# TASK-064: Pure-Rust Soulseek Protocol Search & Message Frame State Machine

## Description
Overhaul the Soulseek adapter to eliminate placeholder logic identified in the audit (`docs/orchestration-audit.md` section 3.3 & 6):
1. Remove Hard-coded Query Branching: Delete the special-case query logic that only responds to `"aphex"` and delete synthetic WAV byte generation fallbacks.
2. Full Message Framing: Implement a complete pure-Rust binary message encoder and decoder for Soulseek protocol frames (Login, SetListenPort, FileSearch, SearchReply, PeerInit, TransferRequest).
3. Peer Connection & Transfer State Machine: Model peer connections as an explicit state machine (`Connecting`, `Handshaking`, `Transferring`, `Completed`, `Failed`) over pure-Rust tokio TCP streams.
4. Error Handling & Fail-Fast: When peer transfers fail or peers are unreachable, return explicit error variants (`SoulseekError::PeerUnreachable`, `SoulseekError::QueueFull`, `SoulseekError::FileBlocked`) instead of synthesized fallback data.

## Verification
```bash
cargo test -p amp-adapter-soulseek
```

## Acceptance Criteria
- No hard-coded search query strings or synthetic audio buffers exist in `amp-adapter-soulseek`.
- Binary message encode/decode conforms to the Soulseek protocol specification.
- Deterministic unit tests cover message encoding, frame parsing, error handling, and state machine transitions.
