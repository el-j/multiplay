# TASK-167: ADR: P2P Mesh — Implement with libp2p or Cut from v1

## Plan Reference
- Plan: [PLAN-046](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-046-p2p-mesh-and-spotify-scope-decisions.md)
- Role: `architecture`
- Priority: **P1**
- Audit findings: `SRC-17`, `SEC-06`, `ARC-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Write ADR comparing: (A) libp2p (Ed25519 identity, Noise XX, Yamux, request-response, invite-code group allowlist), (B) defer to v1.x.
2. If (A): spawn follow-up tasks for transport, catalog sync, transfer, NAT traversal and CRDT queue fixes. If (B): hide P2P from UI/config and feature-gate the crate.

## Acceptance Criteria (user-visible / measurable)
- ADR merged; UI reflects the decision.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `docs/adr/ADR-002-p2p-mesh.md`

## Verification Command
`test -f docs/adr/ADR-002-p2p-mesh.md`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
