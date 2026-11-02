# Release checklist

Run this on **macOS, Windows and Linux** for every release candidate, with the downloads from the
GitHub release (not a local build). Before 1.0, two release candidates in a row must pass on all
three systems. Copy the checklist into the release's tracking issue and tick it there.

Automated checks cover a lot (CI on three systems, measured CPU and memory budgets, the decode
matrix, gapless and DJ tests). This list is for what only a person at the machine can judge.

## 1. Install

- [ ] The download's SHA256 matches `SHA256SUMS`.
- [ ] macOS: the `.dmg` opens, Multiplay drags to Applications and starts. Signed builds: no
      security warning. Unsigned beta: the release-notes workaround works.
- [ ] Windows: the `.zip` unpacks and `multiplay.exe` starts. Signed builds: no SmartScreen
      warning.
- [ ] Linux: the AppImage starts after `chmod +x`; the tarball's `multiplay` starts.
- [ ] `multiplay --version` and `multiplay-tui --version` print the release's version.
- [ ] The app icon shows in the Dock / taskbar / launcher.

## 2. First run

On a fresh user profile (or after deleting `~/.multiplay` and `~/.config/multiplay`):

- [ ] The first-run setup appears; ~/Music is suggested.
- [ ] Add a second folder with the folder dialog; remove one; the window keeps drawing while the
      dialog is open.
- [ ] Choose an output device; switch Bandcamp off; leave remote control off; Start.
- [ ] The scan shows a live count, then "Added N tracks"; the first track plays **within 60 s**
      of starting the app.
- [ ] Restart: the setup does not appear again; playlist, library and window layout are back.

## 3. Playing music

- [ ] A gapless album (live or classical) plays with no gap or click between tracks.
- [ ] Seek by dragging (seeks once on release), by clicking, and with ←/→: lands where expected.
- [ ] FLAC 24/96, MP3, AAC, Opus, DSD and a CUE album each play; the faceplate shows the real
      format and BIT-PERFECT at unity volume on a matching device rate.
- [ ] Keyboard only: Z X C V B, space, arrows, J, Ctrl/Cmd+L.
- [ ] Media keys and the system's Now Playing panel (Control Center / Windows overlay /
      `playerctl status` on Linux) show the track and control it.
- [ ] An internet radio station plays for **30 minutes** without dropouts; song titles update.

## 4. Library

- [ ] LIB: albums show covers; artists, tracks (each sort column) and recently added work.
- [ ] Double-click plays, right-click adds; "+ Add all" adds the filtered list.

## 5. Online sources

- [ ] YouTube: a search finds results; one plays; a pasted YouTube link plays.
- [ ] Bandcamp (switch it back on): search finds an album; a track plays.
- [ ] With both switched off, a search says online search is off.

## 6. Devices and power

- [ ] Unplug the output device (USB DAC or headphones) while playing: playback moves on or
      stops with a message; plugging back in works.
- [ ] Sleep and wake the computer while playing: the app recovers.

## 7. Remote control

- [ ] Turn it on, allow the network, pair a phone with the QR code.
- [ ] The phone controls playback; a song request appears in the Remote window; accept and
      reject both work.
- [ ] Turn it off: the phone disconnects.

## 8. Diagnostics and uninstall

- [ ] Preferences → About → Copy diagnostics pastes a report with no password or token in it.
- [ ] Save diagnostics creates a `.zip` in the log folder.
- [ ] Uninstall (delete the app; on Linux the AppImage) and delete `~/.multiplay` and
      `~/.config/multiplay`: nothing of multiplay is left.

## Record

| | macOS | Windows | Linux |
|---|---|---|---|
| Version | | | |
| Tester / date | | | |
| Passed | | | |
| CPU idle / playing (Activity Monitor, Task Manager, `top`) | | | |
| Issues filed | | | |
