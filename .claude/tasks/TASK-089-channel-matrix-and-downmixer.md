# TASK-089: ITU-R BS.775 Channel Matrix & Downmixer

## Plan Reference
- Plan: [PLAN-028](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-028-bit-perfect-audiophile-playback-pipeline-and-resampling.md)
- Role: `audio_engine`

## Objectives
1. Implement ITU-R BS.775 compliant stereo downmixing in `AudioStreamDecoder` (`crates/amp-adapter-decoder/src/stream.rs`).
2. Properly mix multi-channel source audio (3.0, 4.0, 5.1, 7.1) into stereo without dropping Center (dialogue/vocals) or Surround channels:
   - $L_{out} = L + 0.7071 \cdot C + 0.7071 \cdot SL$
   - $R_{out} = R + 0.7071 \cdot C + 0.7071 \cdot SR$
3. Handle mono-to-stereo duplicate panning ($L = M, R = M$).
4. Add unit tests for 1-channel, 2-channel, and 6-channel downmixing matrix accuracy.

## Verification Command
`cargo test -p amp-adapter-decoder`

## Status
- **Status:** COMPLETED
- **Outcome:** Implemented `downmix_frame` function in `AudioStreamDecoder` supporting ITU-R BS.775 compliant multi-channel and 5.1 surround sound downmixing to stereo, plus mono duplicate upmix, preserving center vocals and surround ambiance without phase distortion. Verified with 5 unit tests in `amp-adapter-decoder`.

