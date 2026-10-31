# ADR-006: DSD, WavPack and the Remaining Audio Formats

- **Status:** Accepted
- **Date:** 2026-10-31
- **Task:** `TASK-231` (`PLAN-058`: Universal Audio Format Support)

## Context

After TASK-227 (Symphonia's codecs) and TASK-229 (Opus, ADR-005), the formats a listener may
still own are DSD (DSF/DSDIFF), WavPack, Monkey's Audio (APE), Musepack, TTA and WMA. The rules
are unchanged: no C, `#![forbid(unsafe_code)]` in our crates, an allowed license, playback under
1 % of one core, and a golden fixture for every format marked playable.

## Decisions

| Format | Decision | Why |
|---|---|---|
| **DSD** (`.dsf`, `.dff`, DSD64 and up, ≤ 6 channels, uncompressed) | **Playable**, own implementation (`amp-adapter-decoder/src/dsd/`) | Both containers are simple. Three-stage decimation (96-tap byte-LUT FIR ÷8, two half-band ÷2) to 88.2 kHz, flat to 24 kHz; 0.86 % of one core for continuous playback (Apple M-series). Fixtures are made by `scripts/make_dsd_fixtures.py` (sigma-delta modulator) and cross-checked with ffmpeg's DSD decoder. DST-compressed DSDIFF is refused with a clear error. |
| **WavPack** (`.wv`, lossless, mono/stereo, 16/24/32-bit int and float) | **Playable** via `wavicle` `=0.1.0` (MIT/Apache, `#![forbid(unsafe_code)]`, no dependencies) | Bit-exact against ffmpeg on 16- and 32-bit files, ~690× realtime. Its API decodes whole streams and stops on the trailing APEv2 tag real files carry; our adapter walks the self-contained blocks itself (one block in memory, stops at the tag, seeks by block headers). Hybrid/lossy (`.wvc`) and > 2 channels are refused with a clear error. |
| `symphonia-codec-wavpack` | Rejected | GPL-3.0-or-later: incompatible with MPL-2.0, blocked by `deny.toml`. |
| **Monkey's Audio** (`.ape`) | Not yet; `ape-decoder` 0.3.2 (MIT/Apache, 18 k downloads) is the candidate | No free encoder is available to make golden fixtures (ffmpeg only decodes APE), so conformance cannot be shown in CI yet. Next step: obtain a redistributable APE sample set. |
| **TTA**, **Musepack**, **WMA** | Not yet | Only `oxideav-*` 0.0.4 crates exist (< 1 k downloads each); too early to depend on. Re-check in three months. |

Formats that remain unplayable are listed in `amp_domain::formats` with these reasons, which
the GUI shows when files are skipped.

## Consequences

- `SymphoniaDecoderFactory` routes `.dsf`/`.dff`/`.wv` (and hint-less streams starting with
  `DSD `, `FRM8` or `wvpk`) to the new decoders; everything else still goes through Symphonia.
- Neither lofty nor Symphonia read DSD tags; DSD files enter the library with the title/artist
  from their file name and the decoder's duration.
