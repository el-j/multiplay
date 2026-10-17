# AUDIT-08 — User Experience (GUI, TUI, DJ, OS Integration)

Part of [AUDIT-2026-10-17](README.md).

"People will love it" depends on the first five minutes and on daily friction. This audit walks both journeys.

## First five minutes today (GUI, macOS)

1. Double-click → the window appears only **after** a synchronous lofty scan of all of `~/Music` (DAT-04). A large library can mean minutes of beachball.
2. macOS firewall prompt: "Allow multiplay to accept incoming network connections?", caused by the remote daemon (SEC-01). No context is given.
3. A playlist with every file from `~/Music` in filesystem order, with no library view.
4. Typing into "🔍 Search playlist…" filters the playlist only. There's no way to search YouTube, Soulseek or anything else (UX-01).
5. Pasting a YouTube URL needs drag-and-drop of a URL (no "Add URL" field). Playback likely fails (SRC-09), and the error goes to `stderr`, which is invisible in a `.app`.
6. Two tracks into an album, a 2 s crossfade kicks in (DSP-07).

## UX-01 — Universal search is not reachable from any UI · P0 (headline feature)
**Evidence:** The TUI `SearchViewState` collects keystrokes (`crates/amp-app/src/app_state.rs:127`), but nothing ever calls `search_coordinator.search(..)`, so results are permanently empty. The GUI has no provider search at all; `search_query` filters the local playlist (`playlist_window.rs`).
**Fix:** An omnibox in both UIs that runs an async filtered search, streams results per provider, shows source badges, and offers play / enqueue / download actions. → TASK-153

## UX-02 — Demo and hard-coded data shipped in the TUI · P1
**Evidence:** Hard-coded "Aphex Twin – Polynomial-C" (fake Soulseek) and "Boards of Canada" (`music/boc_music_is_math.flac`, nonexistent) tracks (`crates/amp-app/src/app_state.rs:68`). The faceplate always shows `44.1 kHz / 320 kbps / stereo` (`:260`). The TUI test asserts on demo-track behaviour.
**Fix:** → TASK-170

## UX-03 — No onboarding · P1
**Fix:** A first-run flow covering music folder selection with a background scan and progress, provider opt-ins with honest notes (account needed, legal note), remote control off by default, and output device choice. → TASK-171

## UX-04 — No library browsing or artwork · P1
**Evidence:** Only a flat playlist exists. `cover_art_uri` is populated for YouTube but never rendered, and embedded art isn't extracted. There are no artist/album/genre views, no sorting, no "recently added", and no ratings.
**Fix:** → TASK-172

## UX-05 — No OS media integration · P1
**Evidence:** No media keys, no macOS Now Playing / Control Center, no Linux MPRIS, no Windows SMTC, and no notifications on track change.
**Fix:** The `souvlaki` crate (pure-Rust bindings over OS APIs) behind a `MediaControlsPort`. → TASK-173

## UX-06 — TUI is not usable as a player · P1
**Evidence:** No way to add files or folders, open URLs, seek, go next/prev, or adjust EQ/balance. Search is non-functional. `eprintln!` from the audio thread writes into the raw-mode alternate screen and corrupts the display (`crates/amp-app/src/audio_driver.rs:199`). There's no help overlay or key map, and `q` quits with no confirmation while music plays.
**Fix:** → TASK-174

## UX-07 — GUI feedback and ergonomics · P1
**Evidence:**
- Errors only go to `stderr`. There are no toasts or status line.
- The seek slider fires `seek()` on every drag `changed()` (`main_window.rs:150`), which triggers a seek storm. It should seek on release.
- Cancelling a file or folder dialog triggers a full `~/Music` rescan (`app_state.rs` `open_file_dialog`/`open_folder_dialog`).
- File dialogs shell out to `osascript`/`zenity`/`powershell` and block the UI thread. `zenity` is often absent on KDE and minimal distros.
- There are no keyboard shortcuts (Winamp users expect Z/X/C/V/B, J for jump), no font scaling or HiDPI review, and no accessibility labels.
**Fix:** → TASK-175

## UX-08 — DJ mode is fragile and undiscoverable · P2
**Evidence:** DJ decks load decoders on the audio thread (`dj_playback.rs`), share the main DSP chain's resampler (DSP-04), clone the deck state every tick, and BPM/WSOLA are not integrated (ARC-07). It is presented as a pro feature.
**Fix:** Mark it **Experimental**. Give it its own engine thread, per-deck resamplers and cue output, or postpone it to after v1. → TASK-176

## UX-09 — Transport semantics inconsistent across surfaces · P1
See ARC-05: remote "Next" = +10 s seek; the TUI reports Paused after Stop; seek is ignored while paused in the GUI. → TASK-131

## UX-10 — Branding uses the "Winamp" trademark · P1 (legal + UX)
**Evidence:** Window titles "WINAMP :: MAIN PLAYER" (`crates/amp-adapter-gui/src/main.rs:13`), "WINAMP PLAYLIST EDITOR" and 9 occurrences in total. "Winamp" is a registered trademark (Llama Group).
**Fix:** Brand as **multiplay**. Keep "compatible with classic Winamp (.wsz) skins" as nominative, descriptive use only. → TASK-194
