# TASK-202: ADR & N-Deck DJ Engine on PlayerEngine (Per-Deck Decode, Resample, Stretch)

## Plan Reference
- Plan: [PLAN-054](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-054-dj-engine-multi-deck-cue-and-beat-analysis.md)
- Design: [Signature features design §5](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `audio_engine`
- Priority: **P0**
- Audit findings: `UX-08`, `ARC-07`, `DSP-04` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: TASK-125, TASK-127, TASK-142

## Objectives
1. ADR: deck model (2 active, 4 supported), threads, command/parameter channels, buses, real-time rules; reconcile TASK-039/041/099/100/101/102 and supersede TASK-176.
2. Deck = async loader + own decoder + own resampler + own time-stretcher, fed through the PlayerEngine transport (no decoding on the audio thread).
3. Parameters (fader, EQ, pitch, crossfader) via lock-free atomics/triple buffers; no deck state cloning per tick (UX-08).
4. Hidden behind `experimental.dj_mode` until PLAN-055 acceptance.

## Acceptance Criteria (user-visible / measurable)
- Two decks at 44.1 and 48 kHz mix at correct pitch with zero underruns for 10 minutes (capture-sink test + soak).
- Audio callback verified allocation- and lock-free (test harness or `assert_no_alloc`-style guard in debug).
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/`
- `crates/amp-ports/src/driving.rs`
- `docs/adr/`

## Verification Command
`cargo test -p amp-core-engine dj`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.

## Status
- **Status:** Pending
