---
id: PLAN-017
title: Playlist Drag-and-Drop Ingestion & Multi-Format Local Media Scanner
status: completed
role: frontend_gui
tasks:
  - TASK-054
  - TASK-055
  - TASK-056
---

# PLAN-017: Playlist Drag-and-Drop Ingestion & Multi-Format Local Media Scanner

## Objective
Enable full OS-level drag-and-drop file, folder, and URL ingestion into the playlist window, parse audio tags via pure-Rust decoders/taggers, and provide a welcoming onboarding empty state with file picker integration.

## Architecture Context
- **OS Native Drag-and-Drop**:
  - Catch `ctx.input(|i| &i.raw.dropped_files)` in the GUI event loop.
  - Recursively traverse dropped directories to collect all supported audio formats (`.mp3`, `.flac`, `.wav`, `.ogg`, `.m4a`, `.aac`, `.opus`, `.aiff`).
  - Detect dropped URL strings (HTTP/HTTPS audio streams, YouTube, Spotify links) and create appropriate `AudioSource` entities.
- **Pure-Rust Metadata Extraction**:
  - Extract ID3v2, Vorbis Comments, and FLAC tags using `lofty` and `symphonia`.
  - Fallback to clean filename heuristic parsing (`"Artist - Title"`) when tags are missing.
- **Empty State & File Chooser**:
  - Replace hardcoded fake mock tracks on launch with an interactive retro drop zone and "Add Files / Add Folder" dialogs.

## Tasks
1. [TASK-054](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-054-native-drag-and-drop-ingestion-files-folders-urls.md): Native OS Drag-and-Drop Ingestion for Audio Files, Folders & URLs
2. [TASK-055](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-055-metadata-extraction-and-playlist-tag-probing.md): Pure-Rust Metadata Extraction & Playlist Item Probing
3. [TASK-056](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-056-empty-playlist-onboarding-and-file-picker.md): Empty Playlist Onboarding State & File Picker Integration
