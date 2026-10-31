//! Shared by the binaries' build scripts: exports `MULTIPLAY_VERSION` and `MULTIPLAY_COMMIT` so
//! `--version` and the About tab show what was released. Release CI sets `MULTIPLAY_VERSION` to
//! the tag (`0.1.0-beta.2`); local builds use the Cargo version and the checked-out commit.

use std::process::Command;

pub fn export() {
    println!("cargo:rerun-if-env-changed=MULTIPLAY_VERSION");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    // On a branch HEAD only names the ref; the ref file is what moves on each commit.
    if let Some(r) = std::fs::read_to_string("../../.git/HEAD")
        .ok()
        .and_then(|head| head.strip_prefix("ref: ").map(|r| r.trim().to_string()))
    {
        println!("cargo:rerun-if-changed=../../.git/{r}");
    }
    let version = std::env::var("MULTIPLAY_VERSION")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| std::env::var("CARGO_PKG_VERSION").unwrap_or_default());
    let commit = std::env::var("GITHUB_SHA")
        .ok()
        .or_else(|| {
            let out = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .ok()?;
            out.status
                .success()
                .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        })
        .map(|sha| sha.chars().take(7).collect::<String>())
        .unwrap_or_default();
    println!("cargo:rustc-env=MULTIPLAY_VERSION={version}");
    println!("cargo:rustc-env=MULTIPLAY_COMMIT={commit}");
}
