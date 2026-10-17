# AUDIT-10 — Testing, Quality & Observability

Part of [AUDIT-2026-10-17](README.md).

## QA-01 — Tests depend on physical audio hardware · P0 for CI
**Evidence:** `test_device_enumeration_and_fallback` asserts at least one output device exists (`crates/amp-adapter-cpal/src/lib.rs:239`). `test_repeated_start_pause_stop_lifecycle` calls `CpalAudioSink::new()` with `expect`. Every GUI and TUI app test constructs a real `AudioPlaybackDriver`. GitHub-hosted runners have no audio devices, so these fail or hang on Linux and Windows.
**Fix:** A `NullAudioSink`/`CaptureSink` in tests through the engine's sink port. Hardware tests go behind `#[ignore]` + `make test-hw`. → TASK-121

## QA-02 — Tests are not hermetic · P1
**Evidence:**
- They write the developer's real session file (DAT-07).
- Constructing `MultiplayGuiApp` in tests starts the remote daemon on `0.0.0.0:9002` (SEC-01). Parallel tests race for the port, and errors are swallowed.
- They mutate process-global env (`std::env::set_var("CI", ...)` in `file_dialog.rs` tests), which races with parallel tests and is `unsafe` in edition 2024.
- `PureYouTubeAdapter::new()` in tests builds a real HTTP client. Several `amp-app` integration tests bind real sockets.
**Fix:** → TASK-122

## QA-03 — Tests validate mocks, not behaviour · P1
**Evidence:** The Soulseek, P2P and Spotify tests assert that a value inserted into a `HashMap` can be read back. The TUI app test asserts demo-track titles. Mutation testing (`.cargo/mutants.toml`) has never been run in CI, and no score is recorded.
**Fix:** Contract tests per port that every adapter (real or fake) must pass. Recorded-fixture tests for network adapters. Real mutation runs on `amp-domain`/`dsp`/`playback` with a recorded threshold. → TASK-187

## QA-04 — No logging or diagnostics · P1
**Evidence:** 30+ `eprintln!` calls. Nothing is persisted, there are no levels and no panic hook. In a macOS `.app` or Windows GUI subsystem, stderr goes nowhere, so field bug reports will have zero data.
**Fix:** `tracing` + `tracing-subscriber` + `tracing-appender` (rolling file in the log dir), a panic hook writing a crash report, and a "Copy diagnostics" button. → TASK-177, TASK-178

## QA-05 — Resource budgets are claimed, never measured · P2
**Evidence:** The policy says idle CPU < 0.2%, active < 1.0%, RSS ≤ 35 MB. Busy loops (AUD-14), unbounded buffers (AUD-10), eframe/wgpu baseline (typically 60–120 MB RSS) and the 60 FPS repaint while playing make the 35 MB target unrealistic for the GUI as built.
**Fix:** A benchmark harness (headless engine playing a fixture through the null sink) measuring CPU and RSS in CI. Set realistic per-front-end budgets (headless/TUI vs GUI). → TASK-179

## QA-06 — No real-audio fixture corpus · P1
**Evidence:** Every decode test uses a synthetic 16-bit WAV generated in code. MP3 (encoder delay/padding, VBR/Xing), FLAC 24-bit/96k, AAC/M4A (gapless iTunSMPB), Vorbis (chained), multichannel and corrupt files are never tested. Gapless "sample-accurate" claims have no sample-accurate test.
**Fix:** A small, CC0-licensed fixture set (generated with known content, for example sine sweeps and clicks at known sample positions) and golden tests: gapless boundary continuity, seek accuracy ±1 frame, duration accuracy, and a null test for the transparent path. → TASK-184

## QA-07 — No fuzzing of untrusted parsers · P2
Targets: M3U/PLS/XSPF, `.wsz` zip + BMP/palette, the Soulseek frame codec, remote JSON actions, ICY metadata parsing, and HTTP header parsing. → TASK-185

## QA-08 — No soak, stress or failure-injection testing · P2
Needed: an 8-hour playback memory-growth check, 1,000 rapid skips, device unplug, network drop mid-stream, a slow server (1 KB/s), a resolver timeout, and a disk-full during download. → TASK-186
