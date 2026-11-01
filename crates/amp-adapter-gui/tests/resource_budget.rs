//! Measured resource budgets (TASK-179): the real player engine with the real-time null output,
//! idle and playing FLAC and MP3, measured from outside the code under test: the process's CPU
//! time and resident memory as the OS reports them.
//!
//! Release build only (debug decoding is ~10x slower and would measure the compiler, not us):
//! `cargo test --release -p amp-adapter-gui --test resource_budget -- --ignored --nocapture`
//! The numbers and how to read them are in `docs/performance.md`.

#![forbid(unsafe_code)]

use amp_adapter_gui::create_headless_engine;
use amp_domain::{AudioSource, Track, TrackMetadata};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// Policy targets (`.nexus/policy.json`), in % of one CPU core and MiB. Memory is met and
/// enforced; the CPU targets are not met yet and are reported against (docs/performance.md).
/// The guard test below keeps policy, doc and these numbers in step.
const IDLE_CPU_PERCENT: f64 = 0.2;
const PLAYING_CPU_PERCENT: f64 = 1.0;
const RESIDENT_MIB: f64 = 35.0;

/// Measured 2026-10-31/11-01 (docs/performance.md): the worst run on a laptop or a CI runner,
/// in % of one core (shared runners vary by 2-3x between runs). CI fails when a run is more than 20% above them, so regressions show up without
/// runner noise failing the build.
const MEASURED_IDLE_CPU_PERCENT: f64 = 0.5;
const MEASURED_PLAYING_CPU_PERCENT: f64 = 2.2;
const REGRESSION_MARGIN: f64 = 1.2;

const WARM_UP: Duration = Duration::from_secs(2);
const WINDOW: Duration = Duration::from_secs(12);

/// CPU seconds this process has used so far, and its resident memory in MiB.
fn process_usage() -> (f64, f64) {
    let pid = std::process::id().to_string();
    if cfg!(target_os = "linux") {
        let stat = std::fs::read_to_string("/proc/self/stat").unwrap();
        // Fields after the `(comm)` part; utime and stime are fields 14 and 15 (1-based).
        let after = &stat[stat.rfind(')').unwrap() + 2..];
        let fields: Vec<&str> = after.split_whitespace().collect();
        let ticks: f64 = fields[11].parse::<f64>().unwrap() + fields[12].parse::<f64>().unwrap();
        let hz: f64 = String::from_utf8(
            Command::new("getconf")
                .arg("CLK_TCK")
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .parse()
        .unwrap();
        let status = std::fs::read_to_string("/proc/self/status").unwrap();
        let rss_kib: f64 = status
            .lines()
            .find_map(|l| l.strip_prefix("VmRSS:"))
            .and_then(|v| v.split_whitespace().next())
            .unwrap()
            .parse()
            .unwrap();
        (ticks / hz, rss_kib / 1024.0)
    } else if cfg!(windows) {
        let script = format!(
            "$p = Get-Process -Id {pid}; \"$($p.TotalProcessorTime.TotalSeconds) $($p.WorkingSet64)\""
        );
        let out = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .unwrap();
        let text = String::from_utf8(out.stdout).unwrap();
        let mut it = text.split_whitespace();
        let cpu: f64 = it.next().unwrap().replace(',', ".").parse().unwrap();
        let bytes: f64 = it.next().unwrap().parse().unwrap();
        (cpu, bytes / 1024.0 / 1024.0)
    } else {
        // macOS / BSD: cumulative CPU time as [[hh:]mm:]ss.cc, RSS in KiB.
        let out = Command::new("ps")
            .args(["-o", "time=,rss=", "-p", &pid])
            .output()
            .unwrap();
        let text = String::from_utf8(out.stdout).unwrap();
        let mut it = text.split_whitespace();
        let cpu = it
            .next()
            .unwrap()
            .split(':')
            .fold(0.0, |acc, part| acc * 60.0 + part.parse::<f64>().unwrap());
        let rss_kib: f64 = it.next().unwrap().parse().unwrap();
        (cpu, rss_kib / 1024.0)
    }
}

/// CPU use over `window` as a percentage of one core, and resident memory at its end.
fn measure(window: Duration) -> (f64, f64) {
    let (cpu0, _) = process_usage();
    let start = Instant::now();
    std::thread::sleep(window);
    let (cpu1, rss) = process_usage();
    ((cpu1 - cpu0) / start.elapsed().as_secs_f64() * 100.0, rss)
}

fn bench_track(file: &str) -> Track {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/bench")
        .join(file);
    assert!(
        path.is_file(),
        "{} (run scripts/make_bench_fixtures.sh)",
        path.display()
    );
    Track::new(
        AudioSource::LocalFile {
            path: path.to_string_lossy().into_owned(),
        },
        TrackMetadata::new("Budget", "multiplay"),
    )
}

#[test]
#[ignore = "measures wall-clock CPU use; run in release with --ignored (see docs/performance.md)"]
fn engine_stays_within_cpu_and_memory_budgets() {
    if cfg!(debug_assertions) {
        panic!("run with --release: debug builds measure unoptimized decoding");
    }
    let engine = create_headless_engine().expect("headless engine");
    std::thread::sleep(WARM_UP);
    let (idle_cpu, idle_rss) = measure(WINDOW);

    let mut rows = vec![("idle", idle_cpu, idle_rss)];
    for (label, file) in [
        ("FLAC 44.1 kHz", "bench_20s.flac"),
        ("MP3 256 kbps", "bench_20s.mp3"),
    ] {
        engine.play(bench_track(file));
        std::thread::sleep(WARM_UP);
        let (cpu, rss) = measure(WINDOW);
        assert_eq!(
            engine.playback_state(),
            amp_domain::PlaybackState::Playing,
            "{label} still playing at the end of the window"
        );
        rows.push((label, cpu, rss));
        engine.stop();
    }

    let mut table = format!(
        "### Resource use: {} {}\n\n| Phase | CPU (% of one core) | Target | Resident memory (MiB) |\n|---|---|---|---|\n",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    for (label, cpu, rss) in &rows {
        let target = if *label == "idle" {
            IDLE_CPU_PERCENT
        } else {
            PLAYING_CPU_PERCENT
        };
        let mark = if *cpu <= target { "met" } else { "not met" };
        table.push_str(&format!(
            "| {label} | {cpu:.2} | < {target} ({mark}) | {rss:.1} (budget {RESIDENT_MIB}) |\n"
        ));
    }
    println!("\n{table}");
    // CI tracking: the table lands on the workflow run's summary page.
    if let Some(summary) = std::env::var_os("GITHUB_STEP_SUMMARY") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(summary) {
            let _ = writeln!(f, "{table}");
        }
    }

    for (label, cpu, rss) in &rows {
        let ceiling = REGRESSION_MARGIN
            * if *label == "idle" {
                MEASURED_IDLE_CPU_PERCENT
            } else {
                MEASURED_PLAYING_CPU_PERCENT
            };
        assert!(
            *cpu <= ceiling,
            "{label}: {cpu:.2}% CPU regressed past {ceiling:.2}% (measured baseline + 20%)"
        );
        assert!(
            *rss <= RESIDENT_MIB,
            "{label}: {rss:.1} MiB resident is over the {RESIDENT_MIB} MiB budget"
        );
    }
}

#[test]
fn the_budgets_here_match_the_policy_and_the_performance_doc() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let policy = std::fs::read_to_string(root.join(".nexus/policy.json")).unwrap();
    let doc = std::fs::read_to_string(root.join("docs/performance.md")).unwrap();
    for (text, name) in [(&policy, "policy.json"), (&doc, "performance.md")] {
        for needle in [
            format!("idle CPU < {IDLE_CPU_PERCENT}%"),
            format!("playback CPU < {PLAYING_CPU_PERCENT:.1}%"),
            format!("<= {RESIDENT_MIB:.0} MB"),
        ] {
            assert!(text.contains(&needle), "{name} should say \"{needle}\"");
        }
    }
}
