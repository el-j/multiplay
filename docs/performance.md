# Performance and resource budgets

multiplay's policy (`.nexus/policy.json`) sets these targets:

| | Target | Status |
|---|---|---|
| Steady-state idle | idle CPU < 0.2% of one core | **not met yet** (0.25–0.33%) |
| Active playback | playback CPU < 1.0% of one core | **not met yet** (1.6–2.2%) |
| Memory | base memory footprint <= 35 MB | met (8–11 MiB) |

The numbers below are measured, not estimated. CI re-measures them on every run and fails when
memory goes over budget or CPU use regresses more than 20% past the measured numbers.

## How it is measured

`crates/amp-adapter-gui/tests/resource_budget.rs` runs the real player engine (decoder, resampler,
DSP chain, output stage) with the real-time null output, which drains audio at sound-card pace
without a sound card. It reads the process's own CPU time and resident memory as the operating
system reports them (`/proc` on Linux, `ps` on macOS, `Get-Process` on Windows):

1. 2 s warm-up, then 12 s **idle** (engine running, nothing playing).
2. **FLAC** 44.1 kHz/16-bit stereo, 2 s warm-up, 12 s measured.
3. **MP3** 256 kbps stereo, same.

The output runs at 48 kHz, as most Macs and PCs do by default, so 44.1 kHz music is resampled: the
realistic case. The two 20-second files are music-like (a chord plus quiet pink noise), made by
`scripts/make_bench_fixtures.sh`; a pure sine would compress to almost nothing and flatter FLAC.

```sh
cargo test --release -p amp-adapter-gui --test resource_budget -- --ignored --nocapture
```

Release builds only: debug builds decode about ten times slower. CPU is in % of **one** core, so
2% means a fiftieth of one core.

## Measurements

2026-10-31, Apple Silicon (aarch64) MacBook, macOS, while the machine was busy (load average
10–16), several runs:

| Phase | Before | After | Resident memory |
|---|---|---|---|
| Idle | 0.75% | 0.25–0.33% | 7.7–10.3 MiB |
| FLAC 44.1 kHz | 2.3–3.3% | 1.6–1.8% | 9.1 MiB |
| MP3 256 kbps | 2.7–3.7% | 1.9–2.2% | 9.4–9.7 MiB |

CI adds a table for each runner to the workflow run's summary page.

### What changed (TASK-179)

Profiling the engine (macOS `sample`) while it played FLAC showed where the time went:

- **Resampling, ~60%.** The 256-tap sinc interpolator ran two 256-tap dot products per output
  sample. Track sample rates are fixed, so it is now rubato's synchronous FFT resampler with the
  same BlackmanHarris2 window. `crates/amp-core-engine/tests/resampler_quality.rs` checks it stays
  clean: a 1 kHz tone keeps its level within 0.1 dB, its image is more than 100 dB down, and the
  passband is flat to 20 kHz.
- **Spectrum analyzer, ~30%.** Two FFTs ran for every few milliseconds of audio, one of them never
  shown. Both now run at most once per 60 Hz display frame (`SPECTRUM_INTERVAL`).
- **Wake-ups.** The null output woke every 2 ms and the engine every 2 ms while the 1.36 s output
  buffer was full. Both now wait 10 ms, about one sound-card period; commands still wake the
  engine at once.

### Still to do for the CPU targets

The profile is now flat: no single cost stands out. The rest is the output stage's per-sample
work (fade gain and visualizer tap per frame), moving samples into the output ring, the loudness
meter and the async runtime's timers. The next steps are to batch the output stage per period
instead of per sample and to stop the visualizer tap when no window shows it.

## Desktop app (GUI)

The GUI draws with the GPU and its cost depends mostly on which windows are open and the screen's
refresh rate, so it is measured by hand per release: Activity Monitor (macOS), `top` (Linux) or
Task Manager (Windows), main window and playlist open, idle and playing. Record the numbers in the
release checklist.
