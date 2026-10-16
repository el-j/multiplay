---
id: PLAN-019
title: Audiophile Equalizer DSP Overhaul & Clipping Safeguard Engine
status: completed
role: dsp_engineering
tasks:
  - TASK-059
  - TASK-060
---

# PLAN-019: Audiophile Equalizer DSP Overhaul & Clipping Safeguard Engine

## Objective
Remediate equalizer sound degradation by eliminating per-block delay-line state wipeouts, implementing continuous biquad coefficient updates, adding preamp gain staging with automatic headroom compensation, and integrating an audiophile soft-knee limiter.

## Architecture Context
- **DSP Engine (`amp-core-engine`)**:
  - `BiquadFilter`: Add `update_coefficients(...)` that updates $b_0, b_1, b_2, a_1, a_2$ without zeroing $x_1, x_2, y_1, y_2$ state variables.
  - `GraphicEqualizer`: Add gain change detection threshold ($|\Delta| > 0.01$ dB) to prevent redundant filter coefficient calculation.
  - Fix caller in `audio_driver.rs` to eliminate per-block coefficient reassignment.
- **Headroom & Limiting**:
  - Preamp gain slider (-12dB to +12dB).
  - Headroom compensation when multiple frequency bands are boosted simultaneously.
  - Pure-Rust transparent soft-knee saturation/limiter at the output of the EQ filter cascade, preventing harsh digital wrap-around clipping.

## Tasks
1. [TASK-059](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-059-continuous-biquad-coefficient-morphing-and-state-preservation.md): Continuous Biquad Filter Coefficient Morphing & State Preservation
2. [TASK-060](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-060-preamp-gain-staging-headroom-and-soft-knee-limiter.md): Preamp Gain Staging, Headroom Management & Soft-Knee Limiter DSP
