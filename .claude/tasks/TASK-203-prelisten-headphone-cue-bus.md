# TASK-203: Prelisten: Headphone Cue Bus, Cue Mix & Output Routing

## Plan Reference
- Plan: [PLAN-054](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-054-dj-engine-multi-deck-cue-and-beat-analysis.md)
- Design: [Signature features design §5](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `audio_engine`
- Priority: **P0**
- Audit findings: —
- Depends on: TASK-202, TASK-133

## Objectives
1. Master and cue buses; per-deck PFL toggle; cue/master mix and cue volume.
2. Routing modes: cue to a second output device; split mode on one device (master mono L, cue mono R); no-cue mode when neither is available.
3. Device selection for cue output, hot-plug safe (TASK-133).
4. A deck can be loaded, played, looped and synced into the cue bus while silent on master.

## Acceptance Criteria (user-visible / measurable)
- Capture-sink tests: PFL deck appears only on the cue bus with fader down; split mode channels contain the expected signals.
- Manual: laptop + USB headphones prelisten a track while another plays on speakers.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/bus.rs (new)`
- `crates/amp-adapter-cpal/src/`

## Verification Command
`cargo test -p amp-core-engine dj_cue`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
