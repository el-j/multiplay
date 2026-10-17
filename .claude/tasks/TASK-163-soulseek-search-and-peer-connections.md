# TASK-163: Soulseek Search & Peer Connections (Direct + Firewalled)

## Plan Reference
- Plan: [PLAN-045](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-045-real-soulseek-client.md)
- Role: `integrations`
- Priority: **P1**
- Audit findings: `SRC-14` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-162

## Objectives
1. FileSearch with token; accept incoming peer connections on listen port; outgoing direct connect with fallback to ConnectToPeer/PierceFirewall.
2. Parse zlib-compressed FileSearchResponse incl. attributes (bitrate, duration, VBR, sample rate, bit depth); stream results to the search coordinator.

## Acceptance Criteria (user-visible / measurable)
- Searching a common query returns ≥ 50 results within 10 s on a real network (manual).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-soulseek/src/{peer,search}.rs`

## Verification Command
`cargo test -p amp-adapter-soulseek search`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
