# PLAN-057: Remix Decks, Stems & Mix Recording (Post-1.0, Experimental)

## Plan Overview
- **ID:** `PLAN-057`
- **Title:** Remix Decks, Stems & Mix Recording (Post-1.0, Experimental)
- **Role:** `audio_engine, dsp_engineering`
- **Milestone:** M3.5 — Signature features (post-1.0, experimental)
- **Priority:** P2
- **Status:** Pending
- **Dependencies:** `PLAN-055`
- **Design:** [Signature features design §8](file:///Users/rex-fab-alt/Documents/private/multiplay/docs/superpowers/specs/2026-10-19-signature-features-design.md)

## Goal
Four decks, beat-synced remix loop slots fed from any deck, stem separation (drums, bass, melody, vocals) gated by a pure-Rust inference ADR, and recording of the master mix, so a live remix can become a new track. Ships after 1.0 behind `experimental.remix_decks`.

## Objectives
1. **TASK-220** (P2): Four-Deck Layout & Deck Count Setting
2. **TASK-221** (P2): Remix Slots: Capture Beat-Synced Loops From Any Deck
3. **TASK-222** (P2): ADR Spike: Pure-Rust Stem Separation (Go/No-Go)
4. **TASK-223** (P3): Stem Decks: Drums, Bass, Melody, Vocals per Track
5. **TASK-224** (P2): Record the Master Mix to WAV/FLAC

## Tasks
- [ ] [TASK-220: Four-Deck Layout & Deck Count Setting](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-220-four-deck-layout-and-deck-count.md)
- [ ] [TASK-221: Remix Slots: Capture Beat-Synced Loops From Any Deck](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-221-remix-loop-slots.md)
- [ ] [TASK-222: ADR Spike: Pure-Rust Stem Separation (Go/No-Go)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-222-adr-pure-rust-stem-separation-spike.md)
- [ ] [TASK-223: Stem Decks: Drums, Bass, Melody, Vocals per Track](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-223-stem-decks.md)
- [ ] [TASK-224: Record the Master Mix to WAV/FLAC](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-224-record-master-mix-to-file.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace` plus each task's verification command and manual acceptance evidence (see `docs/definition-of-done.md`, TASK-115).
