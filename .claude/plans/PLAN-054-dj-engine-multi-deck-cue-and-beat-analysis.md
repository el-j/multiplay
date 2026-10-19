# PLAN-054: DJ Engine: Multi-Deck Mixer, Prelisten Cue Bus & Beat Analysis

## Plan Overview
- **ID:** `PLAN-054`
- **Title:** DJ Engine: Multi-Deck Mixer, Prelisten Cue Bus & Beat Analysis
- **Role:** `audio_engine, dsp_engineering`
- **Milestone:** M3.5 — Signature features
- **Priority:** P1
- **Status:** Pending
- **Dependencies:** `PLAN-037`, `PLAN-040` (TASK-125, TASK-127, TASK-142, TASK-143)
- **Design:** [Signature features design §5](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)

## Goal
A real-time N-deck DJ engine in amp-core-engine (2 decks now, 4 in the model) with a master and a headphone cue bus, background BPM/beat-grid/waveform analysis, tempo and phase sync with key lock, A-B and beat loops, hot cues, a full DJ mixer and an automatic beat-aware mixer. Supersedes TASK-176.

## Objectives
1. **TASK-202** (P0): ADR & N-Deck DJ Engine on PlayerEngine (Per-Deck Decode, Resample, Stretch)
2. **TASK-203** (P0): Prelisten: Headphone Cue Bus, Cue Mix & Output Routing
3. **TASK-204** (P0): Background Track Analysis: BPM, Beat Grid, Waveforms (Cached)
4. **TASK-205** (P0): Beat Matching: Tempo & Phase Sync, Quantize, Pitch Range, Key Lock, Nudge
5. **TASK-206** (P1): Loops & Cues: A-B Loop, Beat Loops, Loop Roll, Hot Cues
6. **TASK-207** (P1): DJ Mixer: Trim/Auto-Gain, 3-Band EQ & Kills, Filter, Faders, Crossfader, Limiter, Meters
7. **TASK-208** (P1): Auto Mix: Automatic Beat-Aware Transitions Between Decks

## Tasks
- [ ] [TASK-202: ADR & N-Deck DJ Engine on PlayerEngine (Per-Deck Decode, Resample, Stretch)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-202-adr-n-deck-dj-engine-on-player-engine.md)
- [ ] [TASK-203: Prelisten: Headphone Cue Bus, Cue Mix & Output Routing](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-203-prelisten-headphone-cue-bus.md)
- [ ] [TASK-204: Background Track Analysis: BPM, Beat Grid, Waveforms (Cached)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-204-offline-track-analysis-bpm-beatgrid-waveforms.md)
- [ ] [TASK-205: Beat Matching: Tempo & Phase Sync, Quantize, Pitch Range, Key Lock, Nudge](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-205-tempo-and-phase-sync-key-lock.md)
- [ ] [TASK-206: Loops & Cues: A-B Loop, Beat Loops, Loop Roll, Hot Cues](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-206-loops-ab-beat-loops-and-hot-cues.md)
- [ ] [TASK-207: DJ Mixer: Trim/Auto-Gain, 3-Band EQ & Kills, Filter, Faders, Crossfader, Limiter, Meters](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-207-dj-mixer-eq-filter-gain-meters.md)
- [ ] [TASK-208: Auto Mix: Automatic Beat-Aware Transitions Between Decks](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-208-auto-mix-beat-aware-transitions.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
