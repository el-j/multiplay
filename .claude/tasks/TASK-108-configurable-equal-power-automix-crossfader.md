# TASK-108: Configurable Equal-Power Automix Crossfader

## Plan Reference
- Plan: [PLAN-033](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-033-audiophile-seamless-gapless-playback-automix-and-loudness.md)
- Role: `dsp_engineering` & `audio_engine`

## Objectives
1. Add `crossfade_seconds: Arc<Mutex<f32>>` and `crossfade_curve: Arc<Mutex<CrossfadeCurve>>` to `AudioPlaybackDriver` (0.0s = pure gapless, 1.0s to 12.0s = automix crossfade).
2. When remaining duration of outgoing track equals `crossfade_seconds`, trigger `Crossfader` in `playback_worker.rs`.
3. Concurrently decode outgoing track frames and incoming track frames, blending them using the equal-power constant acoustic energy curve.
4. Add crossfade duration slider and curve selector to the GUI settings / transport bar.

## Verification Command
`cargo test -p amp-core-engine -p amp-adapter-gui`

## Status
- **Status:** Completed
