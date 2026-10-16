---
id: TASK-033
title: Smart Balanced Shuffle Engine & Constraint-Based Queue Manager
planId: PLAN-011
status: completed
role: audio_engine
verificationCommand: cargo test -p amp-core-engine test_smart_shuffle
---

# TASK-033: Smart Balanced Shuffle Engine & Constraint-Based Queue Manager

## Description
Implement the intelligent non-repeating shuffle engine in `crates/amp-core-engine/src/playback/shuffle.rs`:
1. `ShuffleMode`: `Off`, `TrueRandom` (Fisher-Yates permutation), and `SmartBalanced { artist_spacing: usize }`.
2. `RepeatMode`: `Off`, `RepeatAll`, and `RepeatOne`.
3. `SmartQueueManager`: Maintains pristine playlist order, deterministic playback order queue, and backward playback history.
4. Constraint Matrix: Ensures artist spacing $\ge N$ tracks and smoothly balances tempo transitions where possible, falling back gracefully when constraints cannot be met.

## Verification
```bash
cargo test -p amp-core-engine test_smart_shuffle
```
