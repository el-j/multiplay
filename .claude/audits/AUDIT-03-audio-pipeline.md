# AUDIT-03 — Audio Output, Transport, Decoder & Streaming Buffer

Part of [AUDIT-2026-10-17](README.md).

Pipeline today (GUI; the TUI is a reduced copy):

```
resolver ──► SharedPrecacheBuffer (Vec<u8>, unbounded) ──► Symphonia AudioStreamDecoder (stereo downmix)
        ──► PlaybackDspChain (resample → vol → EQ+softclip → crossfeed → limiter → FFT)
        ──► rtrb Producer<f32> (65 536 samples) ──► CPAL callback (f32 only)
```

## Output & transport

### AUD-01 — Pause/stop/seek don't flush queued audio; ~0.7 s of stale audio plays · P1
**Evidence:** The ring holds `DEFAULT_RING_BUFFER_FRAMES = 65536` **samples** (`crates/amp-adapter-cpal/src/lib.rs`). That is ~0.68 s at 48 kHz stereo. Pause and stop only flip `is_playing`, and the worker stops pushing, but the CPAL stream keeps draining the ring: the sink is started once at construction (`audio_driver.rs`) and never paused. Seeks and track changes replace the decoder without discarding queued samples.
**Impact:** Pause takes ~0.7 s to take effect. "Stop" plays a tail. Seek audibly lags. Skipping tracks plays the end of the previous one.
**Fix:** Add a generation/epoch counter. The callback drops samples whose epoch is stale, or the consumer side does an `rtrb` `read_chunk` drain on flush. Pause the CPAL stream on pause. Shrink the ring to ~100–200 ms. → TASK-127

### AUD-02 — Chunks partially dropped on pause/device switch · P2
**Evidence:** The TUI push loop `break`s mid-chunk when `is_playing` turns false (`crates/amp-app/src/audio_driver.rs`), so the rest of the chunk is lost and resume clicks. `audio_pusher.rs:24` drops the entire chunk if `try_lock` fails.
**Fix:** Engine-owned pending buffer that survives pause. → TASK-127

### AUD-03 — Reported position is decoder position, not playback position · P1
**Evidence:** `elapsed` is set from `dec.elapsed()` after each decode (`playback_worker.rs`), which runs ahead of the speaker by the ring latency. The spectrum is computed on the decode thread at the same lead. Seeking sets `frames_played = requested_ts` instead of the actual seeked timestamp (`crates/amp-adapter-decoder/src/stream.rs:193`).
**Fix:** The callback increments an `AtomicU64` frames-played counter tagged with the epoch. Position = seek base + consumed frames. The visualizer reads from a tap aligned to output. → TASK-128

### AUD-04 — Network resolution and preloading run on the audio worker thread · P0
**Evidence:** `load_track_decoder` calls `block_on(resolver.resolve_stream())`, which can wait up to ~1.5 s for a 128 KB probe plus network latency. It is called inside the playback loop for track changes **and** for lookahead preload near the end of the current track (`crates/amp-adapter-gui/src/playback_worker.rs:132`). A failed preload is retried on *every tick* until the track ends.
**Impact:** Audible dropouts at the end of any track followed by a network track. A request storm against YouTube when a preload fails.
**Fix:** Resolve and open decoders on async tasks. Hand the ready decoder to the worker over a channel. Use backoff for failures. → TASK-129

### AUD-05 — Only f32 output streams supported · P1
**Evidence:** `build_output_stream` is always called with an `f32` callback (`crates/amp-adapter-cpal/src/actor.rs:44`), using `default_output_config()`, which can report i16/u16/i32 formats (common on Windows WASAPI shared mode and many USB DACs on Linux). Stream creation then fails, and the error is only `eprintln!`ed.
**Fix:** Match on `SampleFormat` and convert, with TPDF dither for integer formats. → TASK-132

### AUD-06 — Channel count mismatch: always stereo into an N-channel device · P1
**Evidence:** The decoder always downmixes to stereo (`stream.rs` `downmix_frame`). The ring is filled with stereo frames. The device is opened with its default channel count, which can be 4, 6 or 8 on audio interfaces and HDMI. On such devices samples are reinterpreted as the wrong channels, and playback runs at the wrong speed and pitch.
**Fix:** Request a 2-channel config when one is supported. Otherwise map stereo onto the first two channels. → TASK-132

### AUD-07 — No device-change or stream-error recovery · P1
**Evidence:** The error callback only prints (`actor.rs`). Unplugging headphones or a USB DAC, or changing the default device, kills audio until restart. `switch_device` exists (GUI only) but replaces the producer while the worker may hold it.
**Fix:** Watch the error callback and default-device changes. Rebuild the stream on the same ring with the epoch flushed. → TASK-133

### AUD-08 — Dishonest sink port · P2
**Evidence:** `AudioSinkPort::push_interleaved_samples` returns `0` unconditionally (`crates/amp-adapter-cpal/src/lib.rs:199`). `configure()` stores a new rate and channel count in atomics but does not reconfigure the stream (`actor.rs:84`), so later latency maths are wrong. `get_buffer_latency()` treats samples as frames.
**Fix:** Redesign the port around the real data path. → TASK-134

## Decoder

### AUD-09 — `WouldBlock` from the precache buffer corrupts demuxer state · P0
**Evidence:** `SharedPrecacheBuffer::read` returns `io::ErrorKind::WouldBlock` on underrun (`crates/amp-ports/src/stream.rs:79`). Symphonia's `MediaSourceStream` and format readers are not resumable mid-packet. An error inside `next_packet()` after a partial read leaves the reader desynchronized. `PureDecoderEngine` then retries `next_packet()` and `AudioStreamDecoder` returns `Err("WouldBlock")`, which callers string-match.
**Impact:** Random decode errors, skipped audio or premature "track finished" on slow networks, which is exactly where progressive streaming matters.
**Fix:** A blocking reader (`Condvar`) with a timeout and cancellation token. The decoder thread blocks and the engine shows `Buffering`. → TASK-135

### AUD-10 — Unbounded RAM buffering · P0 for radio, P1 otherwise
**Evidence:** `push_chunk` appends to a `Vec<u8>` forever (`stream.rs:35`); `capacity` is only a pre-allocation. For internet radio (infinite) this grows ~58 MB/h at 128 kbps. The background download task is never cancelled when the user switches tracks (`crates/amp-adapter-http/src/lib.rs:105`, the YouTube equivalent), so abandoned streams keep downloading into memory.
**Fix:** A sliding-window ring for live streams. Spill to a disk cache for long files. A `CancellationToken` on drop of the stream handle. → TASK-136

### AUD-11 — No probe hint; Opus/WebM unsupported though advertised · P1
**Evidence:** `Hint::new()` is never given an extension or MIME (`crates/amp-adapter-decoder/src/stream.rs:51`). Symphonia 0.5 features enabled: mp3, flac, pcm, vorbis, aac, isomp4, ogg, wav. There is **no Opus codec and no MKV/WebM demuxer**, yet `.opus` is scanned (`scanner.rs`), offered in the file dialog, and claimed in `CLAUDE.md`. See SRC-09 for the YouTube consequence.
**Fix:** Pass hints. Enable `mkv`. Add a pure-Rust Opus path (for example `symphonia` + an Opus decoder crate, or a documented decision). Remove the claim until it is supported. → TASK-139

### AUD-12 — `ResetRequired` and mid-stream format changes are fatal · P2
**Evidence:** Any decode error other than `DecodeError` ends the track (`stream.rs`). Chained Ogg streams (common for Icecast Vorbis radio) emit `ResetRequired` at every song boundary.
**Fix:** Rebuild the decoder on `ResetRequired` and re-read the track spec. → TASK-139

### AUD-13 — Seek precision · P2
**Evidence:** `SeekMode::Coarse` with `ts = secs × sample_rate`. This assumes the timebase equals the sample rate, ignores the returned `SeekedTo.actual_ts`, and does not discard pre-roll frames. Seek on a non-seekable network source silently clamps to the buffered length (`SharedPrecacheBuffer::seek`).
**Fix:** `SeekTo::Time`, accurate mode for local files, sample-accurate trim, and an honest `is_seekable`. → TASK-139

### AUD-14 — Busy-polling worker loops · P2
**Evidence:** `while !std::thread::panicking()` loops with 2–15 ms sleeps (`playback_worker.rs:34`, `amp-app/src/audio_driver.rs`). The thread is never joined or shut down. Ring full → `sleep(2ms)` per sample retry.
**Fix:** Event-driven worker: park until there is ring space or a command arrives, with clean shutdown on drop. Measure idle CPU against the policy budget. → TASK-125, TASK-179
