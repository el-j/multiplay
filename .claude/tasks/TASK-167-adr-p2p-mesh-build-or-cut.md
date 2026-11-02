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

## Evidence (2026-11-01, branch feat/p2p-mesh)
- Decision: build, on libp2p 0.56 with pnet (product owner: "use libp2p"). `docs/adr/ADR-009-private-mesh-libp2p.md`.
- `crates/amp-adapter-p2p/tests/mesh.rs`: two real nodes on 127.0.0.1 — search + byte-exact streaming, a different code cannot connect, unshared ids refused, jam operations delivered over gossipsub (5 passed).
- `crates/amp-domain/tests/jam_crdt.rs`: convergence under any delivery order (proptest, 20,000 cases locally).
- `crates/amp-adapter-gui/tests/mesh_setup.rs`: off by default, code in the vault (not config), bad code refused, leave stops the node (2 passed).
- `docs/privacy.md` "Private mesh" section; privacy guard passes.
- `./scripts/verify_pure_rust.sh` (+ `--self-test`), `cargo deny check`, clippy `-D warnings`, `cargo test --workspace`: 628 passed, 0 failed.
- Not yet: jam-session window, members on other networks (no relay), mesh in the terminal app.
