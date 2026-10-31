//! The add dialog lets people pick several files and folders at once. The dialogs themselves
//! can't run in tests, so this checks what they are built from: the macOS panel script (which
//! must compile) and the parsing of the paths they print.

#![forbid(unsafe_code)]

use amp_adapter_gui::file_dialog::{dialog_extensions, macos_open_panel_script, parse_paths};
use std::path::PathBuf;

#[test]
fn every_selected_path_is_returned() {
    let out = "/Music/Album A\n/Music/song one.flac\n\n  /Music/Album B/  \n";
    assert_eq!(
        parse_paths(out),
        vec![
            PathBuf::from("/Music/Album A"),
            PathBuf::from("/Music/song one.flac"),
            PathBuf::from("/Music/Album B/"),
        ]
    );
    assert!(
        parse_paths("").is_empty(),
        "a cancelled dialog adds nothing"
    );
}

#[test]
fn the_macos_panel_takes_several_files_and_folders() {
    let script = macos_open_panel_script(&dialog_extensions());
    for needle in [
        "canChooseFiles = true",
        "canChooseDirectories = true",
        "allowsMultipleSelection = true",
    ] {
        assert!(script.contains(needle), "missing {needle}");
    }
    for ext in dialog_extensions() {
        assert!(script.contains(&format!("'{ext}'")), "missing .{ext}");
    }
}

/// Compiles the panel script with the system's JavaScript for Automation compiler without
/// running it, so no dialog opens.
#[cfg(target_os = "macos")]
#[test]
fn the_macos_panel_script_compiles() {
    let out = std::env::temp_dir().join(format!("multiplay-panel-{}.scpt", uuid::Uuid::new_v4()));
    let status = std::process::Command::new("osacompile")
        .args(["-l", "JavaScript", "-o"])
        .arg(&out)
        .arg("-e")
        .arg(macos_open_panel_script(&dialog_extensions()))
        .output()
        .expect("osacompile runs");
    let _ = std::fs::remove_file(&out);
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
}
