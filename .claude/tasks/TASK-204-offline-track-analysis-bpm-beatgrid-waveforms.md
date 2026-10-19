# TASK-204: Background Track Analysis: BPM, Beat Grid, Waveforms (Cached)

## Plan Reference
- Plan: [PLAN-054](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-054-dj-engine-multi-deck-cue-and-beat-analysis.md)
- Design: [Signature features design §5](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `dsp_engineering`
- Priority: **P0**
- Audit findings: —
- Depends on: TASK-202, TASK-148

## Objectives
1. Background analysis job: BPM, beat grid with downbeat phase, overview waveform and three-band detail waveform; optional musical key (P2).
2. Cache results in the library DB keyed by stable track ID with an analysis version; analyse on load when missing, never on the audio thread.
3. Reuse/replace `beat_detector.rs` based on measured accuracy.

## Acceptance Criteria (user-visible / measurable)
- BPM within ±0.1 on synthetic click tracks (90–175 BPM) and within ±1 % on the TASK-184 corpus; beat-grid phase error < 10 ms.
- A 5-minute track analyses in < 3 s on CI release build.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/analysis/ (new)`
- `crates/amp-adapter-storage/src/`

## Verification Command
`cargo test -p amp-core-engine dj_analysis`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.
- 2026-10-19: offline analyser `dj::analysis` (replaces `beat_detector.rs` for DJ use; the real-time detector cannot place a grid). It makes one streaming pass:
  - two-band onset envelopes at about 2.9 ms;
  - autocorrelation for coarse tempo, with octave handling for the 85-176 BPM range;
  - a comb-fold search to 0.002 BPM for fine tempo and beat phase;
  - low-band accents to place the downbeat;
  - a 1,024-column overview and a 100/s three-band (low/mid/high) detail waveform.
- Results: `amp_domain::TrackAnalysis` with grid helpers (`beat_time`, `nearest_beat`). Cached through the new `AnalysisCachePort`, which the redb library DB implements in table `dj_analysis_v1`. `spawn_analysis` reuses a cached result of the current `ANALYSIS_VERSION`, otherwise waits for the deck's track to finish decoding, analyses it off the audio thread and stores it.
- Evidence (`cargo test -p amp-core-engine dj_analysis`):
  - BPM within ±0.1 and beat phase within 10 ms on click tracks at 90, 100, 120, 126, 128, 140, 150 and 174 BPM;
  - 123.45 BPM within ±0.05;
  - downbeat on the accented beat at 96, 128, 140 and 172 BPM;
  - silence gives no tempo;
  - waveform shapes;
  - cache hit means no re-analysis, and an old version is redone;
  - the redb cache survives a reopen.
  Release timing: a 5-minute track in 0.22 s including generating the test signal (target under 3 s; `cargo test --release -p amp-core-engine dj_analysis_is_fast`).
- Deviations / open:
  - Cache key is the track source (`analysis_key`) until stable IDs land (TASK-148).
  - The ±1 % check on real music waits for the TASK-184 corpus.
  - Musical key (P2) is not done.

## Status
- **Status:** Pending
