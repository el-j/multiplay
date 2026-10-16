---
id: PLAN-011
title: Advanced Audio Playback Intelligence (Smart Shuffle, Gapless Crossfading & EBU R128 Normalization)
status: planned
role: audio_engine
tasks:
  - TASK-033
  - TASK-034
  - TASK-035
---

# PLAN-011: Advanced Audio Playback Intelligence (Smart Shuffle, Gapless Crossfading & EBU R128 Normalization)

## Objective
Implement audiophile-grade playback intelligence capabilities as specified in `docs/vision/advanced_playback_engines_audio_intelligence.md`:
1. Smart balanced non-repeating shuffle engine avoiding artist clustering and tempo disjunctions.
2. Dual-pipeline gapless decoder engine and real-time interpolated crossfader (Linear & Equal Power curves).
3. Pure-Rust ITU-R BS.1770-4 / EBU R128 loudness normalization and peak limiter preventing clipping.

## Architecture Context
- Crate Targets:
  - `crates/amp-core-engine/src/playback/shuffle.rs`: `SmartQueueManager`, `ShuffleMode`, `RepeatMode`, constraint distance matrix.
  - `crates/amp-core-engine/src/dsp/crossfade.rs`: `Crossfader`, `CrossfadeCurve`, dual-pipeline frame blending.
  - `crates/amp-core-engine/src/dsp/loudness.rs`: `PureVolumeNormalizer`, `LoudnessConfig`, K-weighting filter & gated LUFS integrator.
- Hexagonal Boundary: Pure internal core engine components; no direct adapter imports.
- Real-Time Constraints: DSP crossfading and loudness processing are 100% allocation-free and lock-free in audio streaming loops.

## Tasks
1. [TASK-033](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-033-smart-balanced-shuffle-engine.md): Smart Balanced Shuffle Engine & Constraint-Based Queue Manager
2. [TASK-034](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-034-dual-pipeline-gapless-crossfader.md): Dual-Pipeline Gapless Decoder Mixer & Real-Time Crossfader DSP
3. [TASK-035](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-035-pure-rust-ebu-r128-loudness-normalizer.md): Pure-Rust EBU R128 Loudness Normalizer & Peak Limiter

## Definition of Done
- Smart shuffle satisfies artist spacing $\ge 3$ and non-repeating permutation guarantee.
- Gapless crossfader achieves seamless transition between track boundaries with $< 1\text{ ms}$ jitter.
- Volume normalizer processes frames without clipping overs ($< -0.5\text{ dBFS}$) and satisfies unit test benchmarks.
- 100% pure Rust verified with zero C dependencies.
