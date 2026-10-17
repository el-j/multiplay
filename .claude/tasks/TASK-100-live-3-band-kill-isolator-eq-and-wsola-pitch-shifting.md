# TASK-100: Live 3-Band Kill-Isolator EQ & WSOLA Pitch Shifting

## Plan Reference
- Plan: [PLAN-031](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-031-pro-live-dj-engine-and-realtime-mixer.md)
- Role: `dsp_engineering`

## Objectives
1. Integrate `KillIsolatorEq` into Deck A and Deck B audio processing chains.
2. Support instantaneous low, mid, and high kill switch toggles with zero pop/click artifacts.
3. Wire `WsolaTimeStretcher` into deck pitch adjustment slider (-16% to +16% tempo adjustment) with key-lock mode preserving musical pitch.
4. Add unit test verifying kill switch frequency cuts and WSOLA speed scaling.

## Verification Command
`cargo test -p amp-core-engine test_dj`

## Status
- **Status:** Completed
