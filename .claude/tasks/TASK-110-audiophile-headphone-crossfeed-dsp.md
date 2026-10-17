# TASK-110: Audiophile Headphone Crossfeed DSP

## Plan Reference
- Plan: [PLAN-033](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-033-audiophile-seamless-gapless-playback-automix-and-loudness.md)
- Role: `dsp_engineering` & `audio_engine`

## Objectives
1. Implement pure-Rust `HeadphoneCrossfeed` (Bauer / Meier stereophonic simulation) in `amp-core-engine::dsp::crossfeed`.
2. Apply low-pass frequency cross-bleed (typically ~700 Hz cutoff) with an interaural delay buffer (~300 µs / ~14 samples at 48kHz) to eliminate unnatural hyper-separated stereo fatigue on headphones.
3. Integrate `HeadphoneCrossfeed` into `PlaybackDspChain` with zero allocations on the audio path.
4. Expose crossfeed toggle and intensity profile (Subtle / Standard / Deep) in the Equalizer / Settings GUI panel.

## Verification Command
`cargo test -p amp-core-engine -p amp-adapter-gui`

## Status
- **Status:** Completed
