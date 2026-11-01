//! The panic hook (TASK-178) writes a crash report and runs the front end's cleanup (the TUI
//! restores the terminal) before the default hook prints. Own test binary: hooks are global.

#![forbid(unsafe_code)]

use amp_core_engine::diagnostics::{install_crash_hook, AppInfo};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[test]
fn a_panic_writes_a_report_and_runs_the_cleanup() {
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
