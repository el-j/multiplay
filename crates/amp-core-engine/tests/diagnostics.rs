//! Field diagnostics (TASK-177, TASK-178): log files, redacted config, the diagnostics text a
//! beta tester pastes into an issue, and the zip they can attach.

#![forbid(unsafe_code)]

use amp_core_engine::diagnostics::{
    crash_report, diagnostics_text, file_subscriber, redact_config, write_diagnostics_zip, AppInfo,
};
use std::io::Read;

fn app() -> AppInfo {
    AppInfo {
        name: "multiplay",
        version_line: "multiplay 0.1.0-beta.1 (1a2b3c4)".to_string(),
    }
}

#[test]
fn log_lines_land_in_a_daily_file_in_the_log_folder() {
    let dir = tempfile::tempdir().unwrap();
    let (subscriber, guard) = file_subscriber(dir.path(), "info").unwrap();
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(target: "amp_test", "stream opened");
        tracing::debug!("below the level, not written");
    });
    drop(guard); // flushes the background writer
    let logs: Vec<_> = std::fs::read_dir(dir.path()).unwrap().flatten().collect();
    assert_eq!(logs.len(), 1, "{logs:?}");
    let name = logs[0].file_name().to_string_lossy().into_owned();
    assert!(
        name.starts_with("multiplay.") && name.ends_with(".log"),
        "{name}"
    );
    let text = std::fs::read_to_string(logs[0].path()).unwrap();
    assert!(text.contains("stream opened"), "{text}");
    assert!(!text.contains("below the level"), "{text}");
}

#[test]
fn secrets_are_redacted_from_the_config() {
    let toml =
        "[accounts]\nsoulseek_user = \"dj\"\nsoulseek_password = \"hunter2\"\napi_token = 'abc'\n\
                [remote]\nport = 9002\n";
    let out = redact_config(toml);
    assert!(!out.contains("hunter2") && !out.contains("abc"), "{out}");
    assert!(out.contains("soulseek_password = \"<redacted>\""), "{out}");
    assert!(
        out.contains("soulseek_user = \"dj\"") && out.contains("port = 9002"),
        "{out}"
    );
}

#[test]
fn a_crash_report_names_the_version_the_place_and_the_backtrace() {
    let report = crash_report(
        &app(),
        "index out of bounds",
        Some("src/deck.rs:42:7"),
        "  0: frame",
    );
    for needle in [
        "multiplay 0.1.0-beta.1 (1a2b3c4)",
        std::env::consts::OS,
        std::env::consts::ARCH,
        "index out of bounds",
        "src/deck.rs:42:7",
        "0: frame",
    ] {
        assert!(report.contains(needle), "missing {needle}:\n{report}");
    }
}

#[test]
fn diagnostics_have_the_version_redacted_config_log_tail_and_latest_crash() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("multiplay.2026-10-31.log");
    let lines: String = (0..500).map(|i| format!("line {i}\n")).collect();
    std::fs::write(&log, lines).unwrap();
    std::fs::write(
        dir.path().join("crash-1761900000.txt"),
        "thread panicked: boom",
    )
    .unwrap();

    let text = diagnostics_text(&app(), "password = \"x1\"\nport = 1\n", dir.path());
    assert!(text.contains("multiplay 0.1.0-beta.1 (1a2b3c4)"));
    assert!(
        text.contains("line 499") && !text.contains("line 100\n"),
        "only the tail"
    );
    assert!(text.contains("thread panicked: boom"));
    assert!(text.contains("<redacted>") && !text.contains("x1"));

    let zip_path = write_diagnostics_zip(&app(), "port = 1\n", dir.path()).unwrap();
    assert!(zip_path.starts_with(dir.path()));
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&zip_path).unwrap()).unwrap();
    let mut names: Vec<String> = zip.file_names().map(|n| n.unwrap().into_owned()).collect();
    names.sort();
    assert_eq!(
        names,
        [
            "crash-1761900000.txt",
            "diagnostics.txt",
            "multiplay.2026-10-31.log"
        ]
    );
    let mut summary = String::new();
    zip.by_name("diagnostics.txt")
        .unwrap()
        .read_to_string(&mut summary)
        .unwrap();
    assert!(summary.contains("multiplay 0.1.0-beta.1"));
}
