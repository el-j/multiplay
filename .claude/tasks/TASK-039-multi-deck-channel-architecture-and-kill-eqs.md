---
id: TASK-039
title: Multi-Deck Channel Architecture & 3-Band Kill Isolator EQs
planId: PLAN-013
status: planned
role: audio_engine
verificationCommand: cargo test -p amp-core-engine test_dj_deck
---

# TASK-039: Multi-Deck Channel Architecture & 3-Band Kill Isolator EQs

## Description
Implement the 4-deck DJ mixing channel architecture in `crates/amp-core-engine/src/dj/deck.rs`:
1. `DeckId`: Four independent channels (`DeckA`, `DeckB`, `DeckC`, `DeckD`) supporting arbitrary input audio sources.
2. `DeckChannelState`: Tracks loaded track, playback status, cue PFL flag, pitch sliders, beat-grid alignment, and volume faders.
3. 3-Band Isolator Kill EQs: Steep crossover biquad filters with Low, Mid, and High dials ($-60\text{ dB}$ to $+6\text{ dB}$) and instantaneous kill switches.
4. Cue Headphone (PFL) Bus: Independent secondary audio channel routing for pre-listening before bringing faders up.

## Verification
```bash
cargo test -p amp-core-engine test_dj_deck
```
