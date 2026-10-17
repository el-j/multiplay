# AUDIT-04 — DSP & Audiophile Claims

Part of [AUDIT-2026-10-17](README.md).

The product pitch is "audiophile-grade" and "bit-perfect". The first promise users of such a player check is: *with everything neutral, is the output identical to the source?* Today it is not.

## DSP-01 — Default signal path is not transparent (always-on soft clipper) · P0 for the brand
**Evidence:** `GraphicEqualizer::process_interleaved` runs every sample through `soft_limit()` (`crates/amp-core-engine/src/dsp/equalizer.rs:112`) whenever the EQ is enabled, even with all bands at 0 dB. The EQ is enabled by default (`eq_enabled: true`). `soft_limit` is linear only up to 0.85 FS. Above that its slope drops discontinuously from 1.0 to ~0.15, a hard knee that generates odd harmonics on every modern master (most peak above −1.4 dBFS).
**Fix:** EQ bypasses entirely when all gains and the preamp are 0. No saturation inside the EQ. Clipping protection belongs only in a single final limiter, and only when net gain > 0 dB. Add a *null test* (input − output == 0) to CI. → TASK-140

## DSP-02 — Second limiter always on; no bit-transparent path · P1
**Evidence:** `PlaybackDspChain` always applies `PureVolumeNormalizer` (enabled, ceiling −0.5 dBFS, hard clamp to 0.999) (`crates/amp-core-engine/src/dsp/chain.rs:30`). Volume is applied before the EQ, and crossfeed always runs. Even at 100% volume with DSP off, samples above −0.5 dBFS are altered.
**Fix:** A "Bit-perfect" mode: unity volume, no DSP, native device rate, and a UI indicator ("BIT-PERFECT" when the path is identity). → TASK-140

## DSP-03 — "True-peak" limiter is a per-sample waveshaper · P2
**Evidence:** `loudness.rs` compresses `|x| > ceiling` sample by sample. There is no lookahead, no oversampling and no gain-reduction envelope, so it distorts rather than limits.
**Fix:** A lookahead limiter (1–5 ms) with 4× oversampled peak detection and attack/release smoothing, engaged only when needed. → TASK-141

## DSP-04 — Resampler zero-pads partial blocks mid-stream and truncates output · P1
**Evidence:** `PureResampler::process_interleaved_block` copies `frames < chunk_size` into a zeroed planar buffer and processes a full block (`crates/amp-core-engine/src/dsp/resampler.rs:75`), then truncates the output by ratio. Partial blocks happen routinely: decoder reads that end on packet boundaries, `WouldBlock` partials, and the gapless "stitch" path. Each one inserts silence and phase discontinuities (clicks). Buffers are allocated per call. A single resampler instance is shared between the single-track path and both DJ decks, and between the crossfade's outgoing and incoming tracks, even when their sample rates differ.
**Fix:** An input FIFO that feeds rubato only full blocks, with a flush (`process_partial`) only at true EOF. One resampler per stream. Pre-allocated buffers. → TASK-142

## DSP-05 — Loudness normalization uses wrong data · P1
**Evidence:**
- `R128_TRACK_GAIN` is parsed as LUFS (`crates/amp-adapter-storage/src/scanner.rs:99`). Per RFC 7845 it is a **Q7.8 integer gain in 1/256 dB relative to −23 LUFS**. A value like `-512` becomes "−5.12 LUFS", giving a gain of about −9 dB instead of −2 dB.
- `replay_gain_mb` is extracted but never used (no consumer in any crate).
- The settings UI target LUFS and the on/off toggle are not applied (the chain hard-codes −14).
- There is no measurement for untagged files ("EBU R128 normalizer" in TASK-035/109).
**Fix:** A correct ReplayGain (track/album) and R128 gain implementation, with preamp and clipping prevention using peak tags. Optional background BS.1770 analysis (for example the `ebur128` crate, pure Rust) stored in the library. → TASK-143

## DSP-06 — Visualizer computes FFT over interleaved stereo; bands are linear · P2
**Evidence:** `visualizer.push_samples(&samples)` receives `[L,R,L,R,…]` (`chain.rs:122`), so the 512-point FFT sees a signal at twice the rate with channel alternation, which mirrors the spectrum around Nyquist/2. Bands are 16 equal-width linear slices of the bins (`visualizer.rs:71`), not log-spaced as documented. Band 0 includes DC. Scaling is `avg × 4` with no dB conversion.
**Fix:** Mono downmix, log-spaced bands from ~40 Hz to 16 kHz, dB scaling with peak-hold and decay, fed from an output-aligned tap (AUD-03). → TASK-144

## DSP-07 — Crossfade on by default breaks gapless albums · P1
**Evidence:** `AudioHardwareConfig::default().crossfade_duration_ms = 2000` (`crates/amp-domain/src/config.rs:67`). The GUI pushes it to the driver every frame. Live albums, classical works and DJ mixes then get a 2 s fade between tracks, which is the opposite of gapless. Crossfade blends the two decoders' raw PCM *before* resampling, so a 44.1 kHz track faded into a 48 kHz one plays at the wrong pitch for the fade.
**Fix:** Crossfade off by default. "Smart" mode that never crossfades consecutive tracks from the same album. Resample each stream before mixing. → TASK-145

## DSP-08 — Linear volume taper, no dither · P3
**Evidence:** Volume is a linear multiplier (`chain.rs`), so most of the slider travel sits in the top 6 dB. There is no dither when output is integer (once AUD-05 is fixed).
**Fix:** A dB-mapped taper (for example −60 dB…0 dB with a cubic curve) and TPDF dither for 16-bit output. → TASK-146
