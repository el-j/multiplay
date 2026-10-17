# TASK-189: Wire or Delete Dead Engine Modules (with ADR)

## Plan Reference
- Plan: [PLAN-051](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-051-codebase-consolidation-and-architecture-integrity.md)
- Role: `architecture`
- Priority: **P2**
- Audit findings: `ARC-07` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125

## Objectives
1. For PluginRegistry, GaplessTrackScheduler, ComfortSilenceGenerator, WSOLA, BPM detector, HeadphoneCrossfeed, VideoWindowView, CollaborativeQueue: decide wire / feature-gate / delete in `docs/adr/ADR-004-module-pruning.md` and execute.

## Acceptance Criteria (user-visible / measurable)
- `cargo udeps`/`cargo machete` clean; no exported type without a production consumer or a feature gate.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-core-engine/src/`
- `crates/amp-adapter-tui/src/video_view.rs`

## Verification Command
`./scripts/verify_pure_rust.sh && cargo test --workspace`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
