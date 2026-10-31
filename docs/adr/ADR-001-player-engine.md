# ADR-001: Unified Core Playback Engine (`PlayerEngine`) & Output-Clock Transport

- **Status:** Accepted (TASK-125, TASK-126, TASK-127, TASK-128, TASK-129, TASK-130, TASK-131)
- **Date:** 2026-10-27
- **Plan:** [PLAN-037](../../.claude/plans/PLAN-037-unified-core-playback-engine.md)
- **Addresses:** `ARC-01..ARC-05`, `AUD-01..AUD-04`, `AUD-14`, `UX-09`

## Context

Previously, `amp-core-engine` only provided passive DSP building blocks while both UI crates (`amp-adapter-gui` and `amp-app`) maintained duplicated, diverging playback workers (`AudioPlaybackDriver`, `playback_worker.rs`, `track_loader.rs`):

1. Driving ports (`PlayerControlPort`, `PlaylistManagementPort`) had zero implementations in the workspace (`ARC-01`).
2. Pause, stop, and seek only flipped flags or replaced the decoder without flushing the 65,536-sample (~680 ms) hardware ring buffer, causing ~0.7 s transport latency and stale audio tails after seeking (`AUD-01`, `AUD-02`).
3. Playback position and visualizer spectrum were reported from the decoder/DSP clock rather than the speaker output clock (`AUD-03`).
4. Stream resolution (`resolve_stream`) and lookahead preloading blocked the real-time decode worker thread on `block_on(...)`, causing dropouts and retry storms on network failures (`AUD-04`).
5. Port boundaries used `Result<_, String>` and `e.contains("WouldBlock")` substring matching (`ARC-04`).

## Decision

1. **Single Engine in `amp-core-engine::player::PlayerEngine`:**
   - Implements both `PlayerControlPort` and `PlaylistManagementPort`, consumes `PlayerCommand`, and broadcasts `PlayerEvent` via `EventBus`.
   - Depends strictly on driven ports (`AudioSinkPort`, `DecoderFactoryPort`, `PcmStreamPort`, `StreamResolverPort`) and shared lock-free `OutputClock` telemetry in `amp-ports`. Zero concrete adapter crates are imported by `amp-core-engine`.

2. **Transport State Machine:**
   - Explicit state transitions owned by the engine:
     `Stopped → Loading → Buffering { progress_percent } → Playing ⇄ Paused → Stopped | Error`
   - Every transition updates atomic state and emits `PlayerEvent::PlaybackStateChanged`.

3. **Thread & Channel Topology:**
   - **Control & Async Loader Tasks (Tokio):** Resolve network streams with a configurable per-provider timeout (default 10 s) and cancellation token when the user skips or changes tracks. Ready `Box<dyn PcmStreamPort>` decoders (both active and lookahead-preloaded) are handed to the decode worker over a channel. Preload failures retry with exponential backoff (max 3 attempts) off the audio worker and emit `PlayerEvent::Error`.
   - **Decode & DSP Worker Thread (`multiplay-player-worker`):** Event-driven thread that parks on a `Condvar` when idle/paused (0.0% idle CPU) or waits for ring buffer space. Owns the active/next `PcmStreamPort`, `PlaybackDspChain`, crossfader, DJ handover (`DjHost`), and a persistent `pending_out` sample buffer that survives pause and device switches without dropping mid-chunk samples.
   - **Real-Time Audio Callback (`amp-adapter-cpal`):** Lock-free, allocation-free, syscall-free SPSC consumer draining ~150 ms of interleaved audio frames.

4. **Epoch-Flushed Ring & Output-Clock Position (`OutputClock`):**
   - Producer tracks monotonic `total_pushed_samples`; consumer tracks monotonic `total_popped_samples`.
   - On `stop`, `seek`, or explicit track change, the producer sets `flush_until_sample = total_pushed_samples` and increments `epoch`. On its next tick, the RT callback / capture drain discards all stale samples (`total_popped < flush_until_sample`) via `rtrb::Consumer::read_chunk` in `O(1)` without locks or allocations.
   - On `pause`, the sink applies a 5 ms click-free fade-out and pauses the hardware stream while keeping remaining ring samples intact for instant resume.
   - Playback position is computed from the output clock:
     `position = seek_base + (total_popped_samples - epoch_start_sample) / channels / device_sample_rate`.
   - The RT callback / drain writes output samples into a lock-free `AtomicU32` ring tap in `OutputClock` so the spectrum visualizer and oscilloscope reflect what is currently heard at the speaker.

5. **Engine-Owned Queue Semantics:**
   - `PlayerQueue` integrates `SmartQueueManager`, `RepeatMode { Off, RepeatOne, RepeatAll }`, `ShuffleMode`, `Next`, `Previous` (restarts current track when elapsed > 3 s, otherwise jumps to previous track), and automatic gapless/crossfade advancement.

6. **Typed Port Errors:**
   - All `Result<_, String>` signatures in `amp-ports` are replaced by typed errors (`DecodeError`, `AudioSinkError`, `StorageError`, `VaultError`, `PlaylistError`, `DownloadError`).
   - `DecodeError::WouldBlock` replaces string matching on `"WouldBlock"`.
   - Engine emits structured `PlayerEvent::Error { kind, source, message, recoverable }`.

## Consequences

- `amp-app` and `amp-adapter-gui` delete their private `audio_driver`, `playback_worker`, `audio_pusher`, and `track_loader` modules and drive `PlayerEngine` directly.
- Pause/stop latency is < 30 ms, seeks never play stale audio from the old position, and displayed position stays within ±20 ms of heard audio.
