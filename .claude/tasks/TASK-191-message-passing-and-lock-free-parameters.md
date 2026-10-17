# TASK-191: Replace Arc<Mutex> State Soup with Commands, Atomics & Triple Buffers

## Plan Reference
- Plan: [PLAN-051](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-051-codebase-consolidation-and-architecture-integrity.md)
- Role: `architecture`
- Priority: **P2**
- Audit findings: `ARC-09` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125

## Objectives
1. Engine parameters (volume, EQ, preamp, balance) via atomics/`triple_buffer`; meters out via lock-free snapshot; no public mutable fields on engine handles.

## Acceptance Criteria (user-visible / measurable)
- No `Mutex` acquired on the decode worker hot path per chunk (lint via review + benchmark).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/player/`

## Verification Command
`cargo test -p amp-core-engine && cargo bench -p amp-core-engine`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
