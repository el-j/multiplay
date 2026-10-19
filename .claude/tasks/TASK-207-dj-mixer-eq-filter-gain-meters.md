# TASK-207: DJ Mixer: Trim/Auto-Gain, 3-Band EQ & Kills, Filter, Faders, Crossfader, Limiter, Meters

## Plan Reference
- Plan: [PLAN-054](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-054-dj-engine-multi-deck-cue-and-beat-analysis.md)
- Design: [Signature features design §5](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)
- Role: `dsp_engineering`
- Priority: **P1**
- Audit findings: —
- Depends on: TASK-202, TASK-143

## Objectives
1. Per deck: trim with optional loudness auto-gain, 3-band EQ with kills, one-knob LP/HP filter, channel fader; crossfader with curve choice; master limiter.
2. Per-deck and master peak/RMS meters exposed through a lock-free tap.
3. Classic EQ window acts on the master bus in DJ mode.

## Acceptance Criteria (user-visible / measurable)
- Unit tests: EQ/filter responses within ±0.5 dB of design; equal-power crossfader at centre; limiter prevents overs on a +6 dB test mix.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.
- Pure-Rust verification passed; files within `maxFileLines` (500).

## Key Files
- `crates/amp-core-engine/src/dj/mixer.rs`
- `crates/amp-core-engine/src/dj/deck.rs`

## Verification Command
`cargo test -p amp-core-engine dj_mixer`

## Manual Verification
Describe the release-build demo performed and link evidence (screenshot, screen recording, log, CI run) here before closing.
- 2026-10-19: mixer in the engine:
  - **Channel strip** (`strip.rs`): 3-band isolator EQ on 4th-order Linkwitz-Riley crossovers at 250 Hz and 2.5 kHz, with the low band all-passed so 0 dB sums flat. Bands run -40..+6 dB with kills.
  - **Filter**: one knob, low-pass down to 60 Hz and high-pass up to 8 kHz, 12 dB/oct; the cutoff moves without resetting filter state.
  - **Trim and auto-gain**: auto-gain uses the analysed loudness, clamped to ±12 dB and on by default.
  - **Crossfader curves**: Smooth (equal power) and Sharp (scratch).
  - **Master section**: the classic 10-band EQ on the master bus (`MasterControls`), then a lookahead brickwall limiter at a 0.98 ceiling. The cap is exact by construction, with an O(1) window minimum. The cue bus is delayed by the same 1.3 ms so the headphone mix stays aligned.
  - **Meters**: per-deck pre-fader peak/RMS, master peak/RMS, and limiter gain, all through atomics.
  - `params.rs` was split (`status.rs`, `mixer_params.rs`) to stay under 500 lines.
- 2026-10-19: analysis v2 adds integrated loudness (EBU R128: exact BS.1770 K-weighting as in libebur128, 400 ms blocks with 75 % overlap, absolute and relative gates). A -20 dBFS 1 kHz stereo sine reads -19.99 LUFS at 44.1 and 48 kHz. The first cookbook-shelf version read -20.25 and was replaced. Cached v1 results are recomputed.
- Evidence (`cargo test -p amp-core-engine dj_mixer`):
  - EQ flat within ±0.5 dB from 30 Hz to 16 kHz;
  - +6 dB band boosts within ±0.5 dB;
  - kills below -25/-30 dB;
  - filter -3 dB ±0.5 at its cutoff with a flat pass band;
  - equal power at the crossfader centre;
  - Sharp curve keeps both decks full at 0.9;
  - a +6 dB two-deck mix peaks at or below 0.98 while still loud;
  - the limiter catches spikes and is transparent below the ceiling;
  - meters match the levels;
  - auto-gain is exactly -6 dB;
  - master EQ adds +6 dB at 1 kHz;
  - render stays allocation-free with every stage on.
  Release: 4 decks (2 with key lock) and all stages render 10 s of audio in well under 0.5 s, i.e. more than 20x real time on one core.
- Open: the GUI must show the classic EQ window acting on the master in DJ mode, and the meters (PLAN-055).

## Status
- **Status:** Pending
