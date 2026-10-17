# PLAN-035: Remote Control Daemon Security Hardening

## Plan Overview
- **ID:** `PLAN-035`
- **Title:** Remote Control Daemon Security Hardening
- **Role:** `security, integrations`
- **Milestone:** M0 — Stop the bleeding
- **Priority:** P0
- **Status:** Pending
- **Dependencies:** `PLAN-034`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Close the unauthenticated, LAN- and browser-reachable remote control surface before any build leaves the developer's machine.

## Objectives
1. **TASK-116** (P0): Remote Daemon Off by Default, Loopback Bind, Explicit LAN Opt-In
2. **TASK-117** (P0): Enforce Pairing, Session Secrets, Origin Allow-List & Rate Limits
3. **TASK-118** (P0): Constrain Remote Command Surface & Fix Transport Semantics
4. **TASK-119** (P1): Remote Daemon Security Test Suite

## Tasks
- [ ] [TASK-116: Remote Daemon Off by Default, Loopback Bind, Explicit LAN Opt-In](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-116-remote-daemon-off-by-default-and-loopback-bind.md)
- [ ] [TASK-117: Enforce Pairing, Session Secrets, Origin Allow-List & Rate Limits](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-117-enforce-pairing-session-secrets-and-origin-checks.md)
- [ ] [TASK-118: Constrain Remote Command Surface & Fix Transport Semantics](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-118-constrain-remote-command-surface.md)
- [ ] [TASK-119: Remote Daemon Security Test Suite](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-119-remote-daemon-security-test-suite.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
