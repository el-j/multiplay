# TASK-109: EBU R128 & ReplayGain Loudness Normalization Engine

## Plan Reference
- Plan: [PLAN-033](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-033-audiophile-seamless-gapless-playback-automix-and-loudness.md)
- Role: `dsp_engineering` & `storage_and_state`

## Objectives
1. Add `replay_gain_db: Option<f32>` and `integrated_lufs: Option<f32>` to `TrackMetadata` in `amp-domain`.
2. Extract ReplayGain (`REPLAYGAIN_TRACK_GAIN`, `R128_TRACK_GAIN`, `replaygain_track_gain`) tags in `amp-adapter-storage::scanner` and parse them into numeric dB offsets / LUFS.
3. Connect `PlaybackDspChain::process_interleaved` to receive the active track's `integrated_lufs` or `replay_gain_db` and feed it into `PureVolumeNormalizer`.
4. Add user-configurable target LUFS (-23.0 to -11.0 LUFS) and normalization enable toggle in Settings.

## Verification Command
`cargo test -p amp-domain -p amp-adapter-storage -p amp-core-engine -p amp-adapter-gui`

## Status
- **Status:** Completed
