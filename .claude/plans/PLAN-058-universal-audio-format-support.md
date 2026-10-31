# PLAN-058: Universal Audio Format Support (FLAC, Opus & Every Common Format)

## Plan Overview
- **ID:** `PLAN-058`
- **Title:** Universal Audio Format Support (FLAC, Opus & Every Common Format)
- **Role:** `audio_engine, codec_engineering`
- **Milestone:** M2 — Release readiness
- **Priority:** P0 (FLAC proof, single format list), P1 (Opus, tags, CUE), P2 (exotic formats)
- **Status:** Pending
- **Dependencies:** `PLAN-039` (decoder hardening, ADR-004)
- **Supersedes:** ADR-004 once TASK-228 accepts an Opus decoder

## Goal
Every audio file a listener is likely to own plays in multiplay, from drop, dialog, scan and
phone upload alike, without leaving the pure-Rust policy. A file that cannot be played says
which format it is and why, instead of being skipped silently.

## Where We Stand (2026-10-31)
| Format | Decode | Scan / dialog / drop | Notes |
|---|---|---|---|
| FLAC | ✅ Symphonia `flac` | ✅ | Works; the reported failure traced to drop (bug: drops on satellite windows were ignored) and the dialog. Needs a golden fixture so it can never regress unnoticed. |
| MP3, WAV, AAC/M4A, Ogg Vorbis | ✅ | ✅ | |
| Opus (`.opus`, WebM, MKA) | ❌ | ❌ hidden by ADR-004 | Symphonia 0.6 has no Opus decoder. Pure-Rust decoders now exist (see TASK-228). Also blocks YouTube's Opus streams (itag 251). |
| ALAC, AIFF, CAF, MKA/WebM container, MP1/MP2, ADPCM | ⚙️ available in Symphonia 0.6, not enabled | ❌ | Feature flags only, plus fixtures. |
| WavPack, APE, Musepack, TTA, DSD (DSF/DFF), WMA | ❌ | ❌ | No pure-Rust decoder in the tree; evaluate one by one (TASK-231). |

Today the extension list exists five times (scanner, file dialog, remote upload allow-list,
downloader, playlist hint text) and they already disagree.

## Objectives
1. **TASK-225** (P0): One Format Registry for Scanner, Dialog, Drop and Upload
2. **TASK-226** (P0): Golden Fixture Corpus & Decode Matrix (FLAC First)
3. **TASK-227** (P0): Enable Symphonia's Remaining Codecs & Containers
4. **TASK-228** (P1): ADR-005 Spike: Pure-Rust Opus Decoder (Go/No-Go)
5. **TASK-229** (P1): Opus Playback: Ogg Opus, WebM/MKA and YouTube Opus Streams
6. **TASK-230** (P1): Tags, Artwork and Loudness for Every Decodable Format
7. **TASK-231** (P2): Remaining Formats: DSD, WavPack, APE, Musepack, TTA, WMA
8. **TASK-232** (P1): Playlist & Album Containers: CUE Sheets, PLS, XSPF

## Order
TASK-225 → TASK-226 → TASK-227 (P0 path, ships independently).
TASK-228 → TASK-229 (Opus). TASK-230 and TASK-232 after TASK-225. TASK-231 last.

## Tasks
- [ ] [TASK-225: One Format Registry for Scanner, Dialog, Drop and Upload](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-225-format-registry.md)
- [ ] [TASK-226: Golden Fixture Corpus & Decode Matrix (FLAC First)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-226-golden-format-fixtures.md)
- [ ] [TASK-227: Enable Symphonia's Remaining Codecs & Containers](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-227-enable-symphonia-codecs.md)
- [ ] [TASK-228: ADR-005 Spike: Pure-Rust Opus Decoder (Go/No-Go)](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-228-adr-pure-rust-opus-decoder.md)
- [ ] [TASK-229: Opus Playback: Ogg Opus, WebM/MKA and YouTube Opus Streams](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-229-opus-playback.md)
- [ ] [TASK-230: Tags, Artwork and Loudness for Every Decodable Format](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-230-tags-artwork-loudness-all-formats.md)
- [ ] [TASK-231: Remaining Formats: DSD, WavPack, APE, Musepack, TTA, WMA](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-231-remaining-formats-evaluation.md)
- [ ] [TASK-232: Playlist & Album Containers: CUE Sheets, PLS, XSPF](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-232-cue-pls-xspf.md)

## Verification
`./scripts/verify_pure_rust.sh && cargo test --workspace`, the decode matrix of TASK-226 green
for every format the registry marks playable, and a manual release-build session dropping one
file of each format onto the playlist (see `docs/definition-of-done.md`).

## Outcome (2026-10-31)
| Format | Now |
|---|---|
| MP3, FLAC (up to 24-bit/192 kHz, 5.1), WAV (16/24/f32, ADPCM), AAC & ALAC (M4A), Ogg Vorbis | ✅ playable, golden fixtures |
| AIFF, CAF, MKA/WebM, MP1/MP2 | ✅ playable (TASK-227) |
| Opus (Ogg, WebM; mono–7.1) | ✅ playable via `rusty-opus` (TASK-229, ADR-005) |
| DSD (DSF/DSDIFF) | ✅ playable, own decoder (TASK-231, ADR-006) |
| WavPack (lossless) | ✅ playable via `wavicle` (TASK-231, ADR-006) |
| CUE sheets, PLS, XSPF, M3U | ✅ imported from drop, dialog and folders (TASK-232) |
| Monkey's Audio, TTA, Musepack, WMA | ⏳ shown as skipped with a reason (ADR-006) |

All tasks carry implementation evidence; their Nexus status is for the owner to update.
