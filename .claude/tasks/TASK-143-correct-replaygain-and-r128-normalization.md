# TASK-143: Correct ReplayGain/R128 Normalization (+ Optional BS.1770 Analysis)

## Plan Reference
- Plan: [PLAN-040](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-040-audiophile-dsp-transparency.md)
- Role: `dsp_engineering`
- Priority: **P1**
- Audit findings: `DSP-05` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-150

## Objectives
1. Parse ReplayGain track/album gain+peak; parse `R128_TRACK_GAIN`/`R128_ALBUM_GAIN` as Q7.8 relative to −23 LUFS.
2. Modes: Off / Track / Album / Auto (album when playing an album in order); preamp; clipping prevention using peak.
3. Optional background loudness analysis (`ebur128` crate) stored in the library for untagged files.
4. Wire `loudness_normalization` and `target_lufs` settings.

## Acceptance Criteria (user-visible / measurable)
- Unit tests with known tag values; manual A/B on tagged albums.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `crates/amp-adapter-storage/src/scanner.rs`
- `crates/amp-core-engine/src/dsp/loudness.rs`

## Verification Command
`cargo test -p amp-core-engine loudness && cargo test -p amp-adapter-storage`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, log, CI run) here before closing.

## Status
- **Status:** Pending
