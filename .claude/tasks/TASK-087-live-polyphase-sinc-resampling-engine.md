# TASK-087: Live Polyphase Sinc Resampling Engine

## Plan Reference
- Plan: [PLAN-028](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-028-bit-perfect-audiophile-playback-pipeline-and-resampling.md)
- Role: `dsp_engineering`

## Objectives
1. Integrate `amp_core_engine::PureResampler` (`rubato`) into `AudioPlaybackDriver` in `amp-adapter-gui` and `amp-app`.
2. When loading a track, detect the source sample rate from `AudioStreamDecoder::sample_rate()`.
3. If source sample rate does not match the active CPAL hardware sample rate (e.g., 44.1 kHz file played on 48 kHz hardware), instantiate `PureResampler` to dynamically resample interleaved chunks.
4. If sample rates match, ensure bit-perfect zero-copy passthrough without resampling overhead.
5. Add unit and integration tests verifying correct pitch, frame conversion, and resampler lifecycle.

## Verification Command
`cargo test -p amp-adapter-gui -p amp-app`

## Status
- **Status:** COMPLETED
- **Outcome:** Integrated `amp_core_engine::PlaybackDspChain` (wrapping `PureResampler` with rubato polyphase sinc) into `AudioPlaybackDriver` in both `amp-adapter-gui` and `amp-app`. Dynamically resamples source rates to CPAL hardware output rate (e.g. 44.1 kHz -> 48 kHz). Verified with unit and pipeline tests passing.

