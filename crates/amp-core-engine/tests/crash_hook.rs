//! The panic hook (TASK-178) writes a crash report and runs the front end's cleanup (the TUI
//! restores the terminal) before the default hook prints. Own test binary: hooks are global.

#![forbid(unsafe_code)]

use amp_core_engine::diagnostics::{install_crash_hook, AppInfo};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// The panic hook is global: one test at a time.
static HOOK: Mutex<()> = Mutex::new(());

#[test]
fn a_panic_writes_a_report_and_runs_the_cleanup() {
    let _one_at_a_time = HOOK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    let cleaned = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cleaned);
    install_crash_hook(
        AppInfo {
            name: "multiplay-tui",
            version_line: "multiplay-tui 0.1.0".to_string(),
        },
        dir.path().to_path_buf(),
        Some(Box::new(move || flag.store(true, Ordering::SeqCst))),
    );
    let result = std::thread::spawn(|| panic!("forced test panic")).join();
    assert!(result.is_err());
    let _ = std::panic::take_hook();

    assert!(cleaned.load(Ordering::SeqCst), "cleanup ran");
    let reports: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("crash-"))
        .collect();
    assert_eq!(reports.len(), 1);
    let text = std::fs::read_to_string(reports[0].path()).unwrap();
    assert!(
        text.contains("forced test panic") && text.contains("crash_hook.rs"),
        "{text}"
    );
    assert!(text.contains("multiplay-tui 0.1.0"), "{text}");
}

#[test]
fn a_contained_panic_is_an_error_not_a_crash_report() {
    let _one_at_a_time = HOOK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    install_crash_hook(
        AppInfo {
            name: "multiplay",
            version_line: "multiplay 0.1.0".to_string(),
        },
        dir.path().to_path_buf(),
        None,
    );
    let result = amp_ports::contain_panic(|| -> u32 { panic!("damaged file") });
    let _ = std::panic::take_hook();
    let err = result.expect_err("the panic became an error");
    assert!(err.contains("damaged file"), "{err}");
    assert_eq!(amp_ports::contain_panic(|| 7).unwrap(), 7);
    let reports = std::fs::read_dir(dir.path())
        .map(|d| {
            d.flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with("crash-"))
                .count()
        })
        .unwrap_or(0);
    assert_eq!(reports, 0, "no crash report for a contained panic");
}
