---
id: TASK-054
title: Native OS Drag-and-Drop Ingestion for Audio Files, Folders & URLs
planId: PLAN-017
status: completed
role: frontend_gui
verificationCommand: cargo test -p amp-adapter-gui test_drag_and_drop
---

# TASK-054: Native OS Drag-and-Drop Ingestion for Audio Files, Folders & URLs

## Description
Implement operating system drag-and-drop ingestion directly into the Multiplay desktop GUI:
1. Drag-and-Drop Hook: In `eframe` / `egui`, read `ctx.input(|i| &i.raw.dropped_files)` on every frame.
2. File Ingestion: For each dropped file path, verify if it has a supported audio extension (`.mp3`, `.flac`, `.wav`, `.ogg`, `.m4a`, `.aac`, `.opus`, `.aiff`).
3. Recursive Folder Ingestion: If a directory is dropped from macOS Finder / file manager, recursively traverse directory entries and queue all valid audio files.
4. Link & URL Ingestion: If text or URL is dropped, inspect URI scheme:
   - `http://` / `https://` audio stream -> `AudioSource::HttpStream`.
   - YouTube URL (`youtube.com/watch?v=...`, `youtu.be/...`) -> `AudioSource::YouTube`.
   - Spotify URI / URL -> `AudioSource::Spotify`.
5. Visual Drop Zone: Highlight playlist window with glowing neon border and icon when files or links are dragged over the window.

## Verification
```bash
cargo test -p amp-adapter-gui test_drag_and_drop
```
