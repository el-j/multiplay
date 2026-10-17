# TASK-145: Gapless-First Crossfade Policy (Off by Default, Album-Aware, Rate-Matched)

## Plan Reference
- Plan: [PLAN-040](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-040-audiophile-dsp-transparency.md)
- Role: `dsp_engineering`
- Priority: **P1**
- Audit findings: `DSP-07` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-142, TASK-131

## Objectives
1. Default `crossfade_duration_ms = 0`; Smart mode never crossfades consecutive same-album tracks.
2. Resample each stream to device rate before blending.

## Acceptance Criteria (user-visible / measurable)
- Gapless fixture album (split sine) plays without discontinuity; 44.1k→48k crossfade has correct pitch.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-domain/src/config.rs`
- `crates/amp-core-engine/src/player/`

## Verification Command
`cargo test -p amp-core-engine gapless`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
