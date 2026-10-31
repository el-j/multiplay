# Changelog

What changed for people using multiplay. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/). Until 1.0 every release is a beta: expect rough edges.

## [Unreleased]

The first public beta, to be tagged `v0.1.0-beta.1`.

### Added
- **Preferences → About:** version and commit, a summary of what goes online, and links to the
  privacy statement and the bundled third-party licenses.
- `multiplay --version` and `multiplay-tui --version` (also `-V`).
- [`docs/privacy.md`](docs/privacy.md): every connection the app makes, what is sent and when. A test
  keeps it in step with the code.
- Every download now includes `THIRD_PARTY_LICENSES.html`, `CHANGELOG.md` and `privacy.md`.
- **Plays almost every audio format** (PLAN-058): FLAC, MP3, MP2, AAC/M4A, ALAC, Ogg Vorbis, Opus
  (Ogg, WebM, MKA), WAV, AIFF, CAF, WavPack and DSD (DSF/DFF). Formats that still can't play (APE,
  Musepack, TTA, WMA) are skipped with a reason instead of failing silently.
- **Album containers and playlists:** CUE sheets (each track of a single-file album is its own
  playlist entry), M3U/M3U8, PLS and XSPF.
- **Tags and artwork for every format:** album artist, disc, year, genre and embedded cover art;
  ReplayGain is used for loudness when a file has no measured loudness yet.
- **Bandcamp search** finds albums and tracks; **pasting a link** (YouTube, Bandcamp, a stream URL)
  into the search box adds it.
- **Drop files and folders** onto any window. The Add dialog on macOS takes several files and
  folders at once.
- **Playback problems are shown** in the main window (for example a stream that can't be opened)
  instead of playback just stopping.
- **Remote & Sharing window** (PLAN-056): phones on your Wi-Fi can request tracks; you accept them to
  the playlist, play next or a DJ deck, or reject them. Trusted devices, opt-in phone uploads.
- **DJ mode** (PLAN-054/055): the main window becomes a two-deck console with beat-aligned
  waveforms, sync, key lock, loops, hot cues, a mixer with EQ and filter, headphone cue and auto mix.
- **Classic self-drawn windows** (PLAN-053) with live magnetic docking, responsive resizing, double
  size, and a remembered layout. Settings open as an overlay (Cmd/Ctrl+P).
- App icon on every platform.
- Downloads for macOS (universal, Apple Silicon and Intel), Linux x86_64 and ARM64 (tarball and
  AppImage) and Windows x86_64, with SHA256 checksums.

### Changed
- HTTP requests (Bandcamp, radio, stream links) send an honest User-Agent with the real version,
  `multiplay/<version> (+https://github.com/el-j/multiplay)`, instead of a browser-like one.

### Fixed
- **YouTube results play again.** rusty_ytdl 0.7.4 (the latest crates.io release) returned empty
  audio links; multiplay now uses the library's unreleased fix, pinned to commit `b1c6eb7`.
- Dropping onto the playlist window did nothing.
- Bandcamp releases were never found by search.
- Copying and pasting into the search box did not work.
- Re-importing a playlist on Windows created duplicate tracks.
- Audio dropouts on stream underruns; DJ mode toggling could click.
