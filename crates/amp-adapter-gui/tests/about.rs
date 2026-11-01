//! The About tab and `--version` (TASK-180, TASK-182, TASK-193).

#![forbid(unsafe_code)]

use amp_adapter_gui::about::{licenses_file_near, version_line};
use std::process::Command;

#[test]
fn version_flag_prints_the_version_and_exits_without_a_window() {
    let out = Command::new(env!("CARGO_BIN_EXE_multiplay-gui"))
        .arg("--version")
        .output()
        .expect("run multiplay-gui");
    assert!(out.status.success(), "{out:?}");
    let line = String::from_utf8(out.stdout).unwrap();
    assert_eq!(line.trim_end(), version_line());
    assert!(line.starts_with(&format!("multiplay {}", env!("CARGO_PKG_VERSION"))));
}

#[test]
fn the_license_bundle_is_found_next_to_the_executable_or_in_the_app_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    // Linux tarball / Windows zip: next to the executable.
    std::fs::write(root.join("THIRD_PARTY_LICENSES.html"), "x").unwrap();
    assert_eq!(
        licenses_file_near(&root.join("multiplay")),
        Some(root.join("THIRD_PARTY_LICENSES.html"))
    );
    // macOS: Multiplay.app/Contents/MacOS/Multiplay -> Contents/Resources.
    let contents = root.join("Multiplay.app/Contents");
    std::fs::create_dir_all(contents.join("MacOS")).unwrap();
    std::fs::create_dir_all(contents.join("Resources")).unwrap();
    std::fs::write(contents.join("Resources/THIRD_PARTY_LICENSES.html"), "x").unwrap();
    assert_eq!(
        licenses_file_near(&contents.join("MacOS/Multiplay")),
        Some(contents.join("Resources/THIRD_PARTY_LICENSES.html"))
    );
    // A development build has no bundle.
    let bare = tempfile::tempdir().unwrap();
    assert_eq!(licenses_file_near(&bare.path().join("multiplay-gui")), None);
}

#[test]
fn diagnostics_carry_the_version_and_the_settings_and_save_as_a_zip() {
    use amp_adapter_gui::about::{diagnostics_for, save_diagnostics};
    let logs = tempfile::tempdir().unwrap();
    std::fs::write(
        logs.path().join("multiplay.2026-10-31.log"),
        "INFO track started\n",
    )
    .unwrap();
    let config = amp_domain::AppConfig::default();

    let text = diagnostics_for(&config, logs.path());
    assert!(text.starts_with(&version_line()), "{text}");
    assert!(text.contains("port = 9002"), "settings as TOML: {text}");
    assert!(text.contains("INFO track started"), "{text}");

    let zip = save_diagnostics(&config, logs.path()).unwrap();
    assert!(zip.starts_with(logs.path()) && zip.extension().is_some_and(|e| e == "zip"));
}
