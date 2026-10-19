# Signature Features Design: Classic Windows, DJ Mode, Remote Requests

- **Date:** 2026-10-19
- **Status:** Draft for owner review
- **Plans:** PLAN-053 … PLAN-057 · **Tasks:** TASK-195 … TASK-224
- **Milestone:** M3.5 — Signature features (new; between M3 and M4)

## 1. Intent

These are the last features multiplay takes on before the release work (M4/M5). After them the
project only stabilizes. Three ideas, one feeling: multiplay should look and move like the classic
Winamp 2 player people loved, mix like a real DJ tool (Traktor-class, but fun rather than
intimidating), and let a room full of phones suggest music without anyone handing over control.

## 2. Goals, Non-Goals, Anti-Goals

**Goals**
1. Classic, self-drawn windows with live magnetic docking, exactly like Winamp 2.x.
2. Settings as a one-time overlay you open, adjust and close.
3. A DJ mode that swaps the main window into a deck console, with prelisten (headphone cue),
   beat matching, loops, hot cues, an automatic mixer, and one playlist with deck assignment.
4. A remote and sharing window where the host moderates requests from paired phones; requests
   share a source reference, not audio.
5. A path toward multi-deck live remixing (loops and stems from several tracks), planned now,
   shipped after 1.0 behind an experimental flag.

**Non-goals (for this milestone)**
- MIDI/HID DJ controller mapping (later; the control layer is designed so it can be added).
- Streaming the host's audio to phones.
- Video windows, media library window (covered by PLAN-047 / TASK-172).

**Anti-goals (would be a failure even if it technically works)**
- Windows that snap only after release, jitter while dragging, or can be maximized into a
  stretched faceplate.
- Any visible "Winamp" name or copied Winamp artwork in the default look (TASK-194). The style is
  classic; the pixels are ours. Loading a user's `.wsz` skin stays supported.
- A DJ mode where a transition can't be previewed before the audience hears it.
- A remote that can make the host open a file path, fetch an arbitrary URL, or start playback
  without the host's consent.
- Underruns, clicks or pitch errors during a mix (real-time rules in `CLAUDE.md` apply).

## 3. Constraints

- Pure-Rust policy (`scripts/pure_rust_allowlist.txt`), 500-line file limit, tests first,
  evidence-based Definition of Done (`docs/definition-of-done.md`).
- Hexagonal layout: DJ engine and analysis live in `amp-core-engine` behind ports; GUI only renders
  and sends commands.
- Audio callback stays lock-, allocation- and syscall-free.
- DJ engine builds on the unified PlayerEngine (PLAN-037, TASK-125/127), the streaming-correct
  resampler (TASK-142) and loudness (TASK-143). DJ work therefore starts after M1/M2.
- Remote work builds on the hardened daemon (PLAN-035) and real resolvers/search (M3).
- The GUI stack is eframe/egui 0.29 multi-viewport (`crates/amp-adapter-gui/src/viewports.rs`).

## 4. Part A — Classic Windows & Magnetic Docking (PLAN-053)

### Window set and sizes (classic reference values)
| Window | Size (1×) | Resize | Docks | Notes |
| :--- | :--- | :--- | :--- | :--- |
| Main | 275 × 116 | none | anchor | Becomes the DJ console in DJ mode (Part C) |
| Equalizer | 275 × 116 | none | yes | |
| Playlist | 275 × 232 min | grip, 25 × 29 steps | yes | Shows deck badges in DJ mode |
| Remote & Sharing | 275 × 232 min | grip, 25 × 29 steps | yes | Part D |
| Settings | content-sized | none | **no** | One-time overlay (below) |

- **Double size** (Ctrl+D) scales every dockable window 2×; docking geometry scales with it.
- No maximize, no fullscreen, no OS title bar or border on any window.

### Self-drawn chrome
- Every window is a frameless viewport (`with_decorations(false)`, `with_resizable(false)`, no
  maximize button; on macOS no titlebar/traffic lights).
- Each window draws a classic title bar (active/inactive states) with minimize, roll-up (shade)
  and close buttons, from the built-in original theme or from the loaded `.wsz` sprites
  (`titlebar.bmp`, `pledit.bmp`, `eqmain.bmp`).
- Double-clicking a title bar toggles shade mode (14 px strip at 1×).

### Live magnetic docking (the core behaviour)
- Dragging is done by the app, not the OS: on title-bar press record pointer and window
  position in **screen space**; every frame set `OuterPosition = start + (pointer − press)` after
  snapping. Screen-space math avoids feedback when the window moves under the pointer.
- **Snap while dragging**, within `snap_distance` (default 10 px, setting): to the edges of every
  other visible dockable window and to the monitor work-area edges. Edge alignment: when
  touching vertically, left/right edges also align if within distance (so a playlist lines up
  flush under the EQ); same for horizontal contact.
- **Docked groups:** windows whose edges touch the main window are docked to it, transitively
  (main → EQ → playlist). Dragging **main** moves the whole group rigidly. Dragging any other
  window moves only that window and detaches it; it re-docks when dropped touching.
- **Shade and resize reflow:** when a window shades, unshades or the playlist resizes, windows
  docked below or to the right move with that edge so the group stays closed.
- **Screen safety:** windows are clamped so a title bar is always reachable; on start, layouts
  that fall outside the current monitors are pulled back on screen.
- Geometry (snap, groups, reflow, clamping) is a pure, unit-tested module with no egui types,
  extending `crates/amp-adapter-gui/src/docking/`.

### Platform notes
- macOS, Windows, Linux X11: full behaviour.
- Linux Wayland does not let apps position their own windows. The GUI prefers the X11 backend
  (XWayland) via eframe's event-loop builder hook when `DISPLAY` is available. Pure Wayland
  falls back to independent windows without snapping, with a one-line notice in Settings.

### Settings overlay
- A single centered window above the others, opened from the main window (menu or Ctrl+P).
- It does not dock or snap; closes with ✕, Esc or "Done"; remembers the last tab.
- The pairing QR and paired-device list move from Settings to the Remote window.

### Persistence
- Positions, docking, shade, visibility, playlist size and double-size are stored through the
  config store (PLAN-041, TASK-150) and restored on launch.

## 5. Part B — DJ Engine (PLAN-054)

- **Architecture:** an N-deck engine (2 decks now, 4 supported by the data model) inside
  `amp-core-engine`, driven by commands and lock-free parameters, each deck with its own decoder,
  resampler and time-stretcher. It supersedes TASK-176 and reuses `dj/` (mixer, WSOLA,
  beat detector, kill EQ) where tests prove it correct.
- **Two buses:** master (speakers) and cue (headphones).
  - *Prelisten / cue (PFL):* each deck has a cue button sending it to the cue bus. A cue-mix
    knob blends cue and master in the headphones; a cue volume sets the level.
  - *Routing modes:* (1) cue on a second output device (USB interface/headphones), (2) split mode
    on one stereo device — master mono-summed on the left, cue mono on the right — for laptop
    plus splitter cable, (3) no cue (cue buttons hidden) when neither is available.
  - *Preview before fading:* a deck can be loaded, cued, looped, synced and played into the cue
    bus while its channel fader/crossfader keep it silent on master.
- **Track analysis (offline, background):** BPM, beat grid with downbeat phase, a waveform
  overview and a three-band coloured detail waveform; musical key is optional (P2). Results are
  cached in the library DB keyed by stable track ID (TASK-148); tracks analyse on load if needed.
- **Beat matching:** tempo sync to the master deck, phase sync (beats aligned), quantize, pitch
  range ±8/16/50 %, key lock (tempo change without pitch change), pitch-bend nudge.
- **Loops and cues:** A–B loop (one press for A, one for B; optional beat-quantized), beat loops
  1/4 … 32 beats with halve/double, loop roll, 8 hot cues per track saved in the library.
- **Mixer:** per-deck trim with optional auto-gain from loudness, 3-band EQ with kills, one-knob
  filter (low-pass ← 0 → high-pass), channel fader, crossfader with curve choice, master limiter,
  per-deck and master meters.
- **Auto mix:** when on, the engine transitions to the other deck before the current track ends
  (configurable: beats or seconds before end, or a detected outro). With compatible BPMs it syncs
  and does a beat-aligned transition; otherwise an equal-power crossfade. Then it loads that
  deck's next assigned track. Builds on PLAN-033 crossfade work.

## 6. Part C — DJ Mode Experience (PLAN-055)

- **Entering DJ mode** (button on the main window or Ctrl+J) swaps the main window into the DJ
  console. The playing track continues without a gap and becomes Deck A; Deck B is empty and
  ready. Leaving DJ mode keeps the audible deck playing as normal playback.
- **Console size:** fixed, wider than the classic main window (two 275 px deck strips plus a
  central mixer); it still docks like the main window and anchors the group.
- **One playlist, deck assignment:** in DJ mode each playlist row shows a badge — A, B, or A+B
  (playable on either deck). Assign by clicking the badge, keys (`[` A, `]` B, `\` both) or
  dragging a row onto a deck. Each deck plays its assigned tracks in playlist order; the playlist
  marks "next on A" and "next on B". Unassigned tracks are skipped in DJ mode.
- **Waveforms:** stacked scrolling detail waveforms for both decks with beat markers aligned at a
  fixed playhead, so phase alignment is visible; a phase meter; overview waveforms with cue and
  loop markers; BPM, pitch and key readouts.
- **Controls:** per deck play/pause, cue, sync, PFL, loop A/B, beat-loop size, hot cues 1–8,
  pitch fader and nudge; crossfader; full keyboard map, shown in a cheat-sheet overlay.
- **Fun first:** a first-time hint (three steps: assign, cue, fade), sensible defaults (sync on,
  quantize on, auto mix off), and nothing that can stop the music by accident.

## 7. Part D — Remote & Sharing Window (PLAN-056)

- A playlist-style, resizable, dockable window with three sections: **Requests**, **Devices**
  (paired phones, last seen, revoke one or all, pairing QR) and **Shares** (what this host shares
  over Soulseek/P2P, once PLAN-045 lands).
- **Protocol v2 request** (`RequestTrack`) carries a *reference*, never audio by default:
  a library track ID, a provider link from an allow-list (YouTube, Bandcamp, SoundCloud, as
  resolvers exist), or a search query. No file paths, no arbitrary URLs. Per-device rate limits
  and size limits; the security suite (TASK-119) is extended accordingly. This replaces PLAN-035's
  "remotes never send URLs" with "remotes never make the host open anything without approval".
- **Moderation:** each request shows requester, title/source preview (resolved metadata) and
  actions: add to end, play next, assign to deck A/B (DJ mode), or reject. Rows can be dragged
  into the playlist or onto a deck. Per-device "trusted: auto-accept" option. Playlist entries
  remember who requested them.
- **The host plays the source itself:** an approved link is resolved and streamed by the host's
  own resolvers; the phone only shared the reference.
- **Opt-in phone upload:** only when the host enables it for the session, a phone may send a
  local audio file (size/format limits, decoded with the fuzzed decoder, kept in a temp cache).
- **Phone web client:** search the host's library and providers, paste a link, see request status
  (pending/accepted/rejected/playing).

## 8. Part E — Remix Decks, Stems & Recording (PLAN-057, post-1.0, experimental)

- Four-deck layout (deck count setting 2/4).
- Remix slots: capture beat-quantized loops from any deck into a slot grid; slots play synced to
  the master tempo with per-slot volume and filter.
- Stems (drums, bass, melody, vocals): gated by an ADR spike on pure-Rust inference
  (candle / tract / burn) with an openly licensed model: model size, CPU time per track,
  licence. Fallback if the spike says no: band-split "pseudo-stems" built on the kill EQ.
- Mix recording: record the master bus to WAV/FLAC, so a live remix becomes a new track.

## 9. Decisions Made in This Design (owner may override)

1. New milestone **M3.5**; PLAN-053 (windows) may start in parallel from M1 because it touches
   only the GUI. PLAN-054/055 start after M2; PLAN-056 after M3. *(my call)*
2. Remix decks and stems (PLAN-057) are planned now but scheduled **after 1.0**, behind
   `experimental.remix_decks`. *(my call)*
3. Snap distance 10 px default, configurable. *(my call; classic value not documented)*
4. Wayland: prefer XWayland; pure Wayland gets no snapping. *(my call)*
5. "Play both tracks per deck" means a track can be assigned to A, B, or both (owner accepted the
   playback).
6. MIDI controllers are out of scope for this milestone. *(my call)*
7. Settings is a separate centered window, not drawn inside the small main window. *(my call)*
8. DJ console is 550 × 232 at 1× (two deck strips + mixer column). *(my call)*
9. In DJ mode the classic EQ window acts on the master bus; per-deck EQs live in the console.
   *(my call)*
10. Auto mix with nothing assigned to the other deck takes the next unplayed playlist track and
    assigns it to that deck; with nothing left it lets the last track end. *(my call)*
11. Shortcuts are registered through the shortcut map owned by TASK-175 so they never collide.
    *(my call)*
12. Accuracy targets: BPM within ±0.1 on synthetic click tracks and within ±1 % on the real-audio
    corpus (TASK-184); synced decks stay within 5 ms of beat phase over 5 minutes. *(my call)*

## 10. Left to the Builder

Exact pixel art of the original theme; egui widget structure; analysis algorithms (onset/BPM,
beat-grid fitting) as long as the accuracy targets in the tasks are met; internal channel and
buffer layout of the DJ engine within the real-time rules.
