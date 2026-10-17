# TASK-088: Soft-Knee Limiter & Headroom Safeguard

## Plan Reference
- Plan: [PLAN-028](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-028-bit-perfect-audiophile-playback-pipeline-and-resampling.md)
- Role: `dsp_engineering`

## Objectives
1. Connect `amp_core_engine::PureVolumeNormalizer` into `AudioPlaybackDriver` in `amp-adapter-gui` and `amp-app`.
2. Configure a -0.5 dBFS true-peak limit with soft-knee saturator in the real-time buffer processing chain after equalizer and volume stages.
3. Verify that boosted preamp (+12 dB) and 10-band EQ peaks smoothly saturate rather than hard-clipping or producing numeric overflow.
4. Add unit and property-based tests verifying peak envelope bounds and harmonic cleanliness.

## Verification Command
`cargo test -p amp-adapter-gui -p amp-core-engine`

## Status
- **Status:** COMPLETED
- **Outcome:** Connected `PureVolumeNormalizer` with -0.5 dBFS true-peak ceiling into `PlaybackDspChain`. Prevents digital clipping and inter-sample peaks even under extreme +12 dB preamp or EQ band boosts. Verified with `test_dsp_chain_soft_knee_limiter_prevents_clipping`.

