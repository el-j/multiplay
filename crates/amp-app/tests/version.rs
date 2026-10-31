//! `multiplay-tui --version` prints one line and exits without starting the terminal UI (TASK-180).

#![forbid(unsafe_code)]

use std::process::Command;

#[test]
fn version_flag_prints_the_version_and_exits() {
    for flag in ["--version", "-V"] {
        let out = Command::new(env!("CARGO_BIN_EXE_amp-app"))
            .arg(flag)
            .output()
            .expect("run amp-app");
        assert!(out.status.success(), "{flag}: {out:?}");
        let line = String::from_utf8(out.stdout).unwrap();
        assert!(
            line.starts_with(&format!("multiplay-tui {}", env!("CARGO_PKG_VERSION"))),
            "{flag}: {line}"
        );
        assert_eq!(line.lines().count(), 1, "{line}");
    }
}
