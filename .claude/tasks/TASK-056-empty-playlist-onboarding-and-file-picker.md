---
id: TASK-056
title: Empty Playlist Onboarding State & File Picker Integration
planId: PLAN-017
status: completed
role: frontend_gui
verificationCommand: cargo test -p amp-adapter-gui test_playlist_empty_state
---

# TASK-056: Empty Playlist Onboarding State & File Picker Integration

## Description
Provide an intuitive, aesthetic onboarding experience when starting the player without existing tracks, eliminating confusing placeholder tones:
1. Empty Playlist State: When playlist has 0 tracks, render a sleek retro-futuristic drop zone:
   - "🎧 MULTIPLAY WINAMP AUDIO ENGINE"
   - "Drag & Drop your audio files (.mp3, .flac, .wav, .ogg, .m4a) or stream URLs here"
2. Interactive Action Buttons:
   - `[📁 Add Files...]`: Opens native OS file picker to select one or more audio files.
   - `[📂 Add Directory...]`: Opens folder picker to ingest entire albums/folders.
   - `[🎵 Scan ~/Music]`: Quick action button to scan standard user music folder.
3. Remove hardcoded fake tracks on fresh launch, ensuring every displayed track corresponds to playable audio.

## Verification
```bash
cargo test -p amp-adapter-gui test_playlist_empty_state
```
