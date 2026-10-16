---
id: TASK-019
title: Omnibar Search Modal with Filter Pills & Source Badges
planId: PLAN-006
status: pending
role: frontend_tui
verificationCommand: cargo test -p amp-adapter-tui test_search_view
---

# TASK-019: Omnibar Search Modal with Filter Pills & Source Badges

## Description
Implement the search overlay in `crates/amp-adapter-tui/src/search_view.rs`:
- Global omnibar search input (triggered by `j` or `Ctrl + F`).
- Filter pill row displaying toggleable sources (`[✔] Local`, `[✔] YouTube`, `[✔] Spotify`, `[✔] Bandcamp`, `[✔] SoundCloud`, `[✔] Soulseek`, `[✔] P2P`).
- Real-time results list formatted with source badge icon (` [YT]`, ` [SPOT]`, etc.), color attribution, artist, title, and duration.
- Keybindings: `Enter` to enqueue/play, `d` to trigger background offline download, `Tab` to cycle source filter toggles.

## Verification
```bash
cargo test -p amp-adapter-tui test_search_view
```
