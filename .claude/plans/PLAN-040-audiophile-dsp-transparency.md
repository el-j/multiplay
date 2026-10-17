# PLAN-040: Audiophile DSP Transparency & Correctness

## Plan Overview
- **ID:** `PLAN-040`
- **Title:** Audiophile DSP Transparency & Correctness
- **Role:** `dsp_engineering`
- **Milestone:** M2 — Sounds right
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-037`
- **Source audit:** [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)

## Goal
Earn the 'audiophile' label: identity path by default, a real limiter only when needed, a streaming-correct resampler, correct loudness data and an honest visualizer.

## Objectives
1. **TASK-140** (P0): Bit-Transparent Default Path & CI Null Test
2. **TASK-141** (P2): Lookahead, Oversampled True-Peak Limiter
3. **TASK-142** (P1): Streaming-Correct Resampler (No Mid-Stream Padding, Per-Stream Instances)
4. **TASK-143** (P1): Correct ReplayGain/R128 Normalization (+ Optional BS.1770 Analysis)
5. **TASK-144** (P2): Visualizer: Mono Downmix, Log-Spaced Bands, dB Scaling
6. **TASK-145** (P1): Gapless-First Crossfade Policy (Off by Default, Album-Aware, Rate-Matched)
7. **TASK-146** (P3): Perceptual Volume Taper & TPDF Dither

## Tasks
- [ ] [TASK-140: Bit-Transparent Default Path & CI Null Test](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-140-bit-transparent-default-path-and-null-test.md)
- [ ] [TASK-141: Lookahead, Oversampled True-Peak Limiter](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-141-lookahead-oversampled-true-peak-limiter.md)
- [ ] [TASK-142: Streaming-Correct Resampler (No Mid-Stream Padding, Per-Stream Instances)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-142-streaming-correct-resampler.md)
- [ ] [TASK-143: Correct ReplayGain/R128 Normalization (+ Optional BS.1770 Analysis)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-143-correct-replaygain-and-r128-normalization.md)
- [ ] [TASK-144: Visualizer: Mono Downmix, Log-Spaced Bands, dB Scaling](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-144-correct-log-spaced-mono-visualizer.md)
- [ ] [TASK-145: Gapless-First Crossfade Policy (Off by Default, Album-Aware, Rate-Matched)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-145-gapless-first-crossfade-policy.md)
- [ ] [TASK-146: Perceptual Volume Taper & TPDF Dither](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-146-perceptual-volume-taper-and-dither.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
