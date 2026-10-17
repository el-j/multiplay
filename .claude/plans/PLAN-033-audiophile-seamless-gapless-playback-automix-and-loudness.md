# PLAN-033: Audiophile Seamless Gapless Playback, Automix Crossfader & EBU R128 Normalization

## Plan Overview
- **ID:** `PLAN-033`
- **Title:** Audiophile Seamless Gapless Playback, Automix Crossfader & EBU R128 Normalization
- **Role:** `audio_engine`, `dsp_engineering` & `frontend_gui`
- **Status:** Completed
- **Dependencies:** `PLAN-028`, `PLAN-030`, `PLAN-031`

## Objectives
1. **TASK-107**: Lookahead Track Preloader & Sample-Accurate Gapless Transition Feeder — Eliminate buffer underrun pops and silence gaps during sequential track advancement with lookahead decoder priming.
2. **TASK-108**: Configurable Equal-Power Automix Crossfader — Seamlessly crossfade consecutive playlist tracks with configurable duration (0-12s) and equal-power energy preservation.
3. **TASK-109**: EBU R128 & ReplayGain Loudness Normalization Engine — Extract ReplayGain/R128 tags from files, normalize playback gain to target LUFS (-14 dBFS default), and prevent inter-sample overs.
4. **TASK-110**: Audiophile Headphone Crossfeed DSP (Bauer / Meier Binaural Simulation) — Implement zero-allocation crossfeed DSP for reduced headphone fatigue and natural spatial imaging.
5. **TASK-111**: Gapless, Crossfade & DSP Pipeline Verification Harness — Automated integration tests validating gapless sample stitching, crossfade energy continuity, and full `make all` compliance.

## Tasks
- [x] [TASK-107: Lookahead Track Preloader & Sample-Accurate Gapless Transition Feeder](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-107-lookahead-track-preloader-and-gapless-feeder.md)
- [x] [TASK-108: Configurable Equal-Power Automix Crossfader](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-108-configurable-equal-power-automix-crossfader.md)
- [x] [TASK-109: EBU R128 & ReplayGain Loudness Normalization Engine](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-109-ebu-r128-and-replaygain-loudness-engine.md)
- [x] [TASK-110: Audiophile Headphone Crossfeed DSP](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-110-audiophile-headphone-crossfeed-dsp.md)
- [x] [TASK-111: Gapless, Crossfade & DSP Pipeline Verification Harness](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-111-gapless-crossfade-and-dsp-verification-harness.md)

## Verification
`cargo test --workspace && ./scripts/verify_pure_rust.sh && make all`
