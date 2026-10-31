//! `docs/privacy.md` has to match the code (TASK-193): every host the code names in a constant,
//! and the remote-control defaults, appear in the statement.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn statement() -> String {
    std::fs::read_to_string(workspace().join("docs/privacy.md")).expect("docs/privacy.md exists")
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Hosts from `const NAME: &str = "https://host/..."` or `"host:port"` in production sources.
fn hosts_named_in_constants() -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    for krate in std::fs::read_dir(workspace().join("crates"))
        .unwrap()
        .flatten()
    {
        let src = krate.path().join("src");
        if src.is_dir() {
            rust_sources(&src, &mut files);
        }
    }
    let mut hosts = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        // Unit tests sit at the end of a file; their URLs are fixtures, not connections.
        let code = text.split("#[cfg(test)]").next().unwrap_or("");
        for line in code.lines() {
            let line = line.trim();
            let Some(rest) = line
                .strip_prefix("pub const ")
                .or_else(|| line.strip_prefix("const "))
            else {
                continue;
            };
            let Some(value) = rest.split("&str = \"").nth(1) else {
                continue;
            };
            let value = value.split('"').next().unwrap_or("");
            let bare = value
                .strip_prefix("https://")
                .or_else(|| value.strip_prefix("http://"))
                .unwrap_or(value);
            let host = bare.split(['/', ':']).next().unwrap_or("");
            if host.contains('.') && !host.ends_with(".html") && !host.ends_with(".md") {
                hosts.push((host.to_string(), file.clone()));
            }
        }
    }
    hosts
}

#[test]
fn every_host_in_the_code_is_in_the_privacy_statement() {
    let doc = statement();
    let hosts = hosts_named_in_constants();
    assert!(
        hosts.iter().any(|(h, _)| h == "bandcamp.com"),
        "the scan finds known hosts: {hosts:?}"
    );
    for (host, file) in hosts {
        assert!(
            doc.contains(&host),
            "{host} (from {}) is missing from docs/privacy.md",
            file.display()
        );
    }
}

#[test]
fn the_statement_covers_providers_remote_control_and_telemetry() {
    let doc = statement();
    let remote = amp_domain::config::RemoteControlConfig::default();
    assert!(!remote.enabled && !remote.allow_lan);
    for needle in [
        "youtube.com",
        "googlevideo.com",
        "bcbits.com",
        &remote.port.to_string(),
        "No telemetry",
        "User-Agent",
    ] {
        assert!(
            doc.contains(needle),
            "docs/privacy.md should mention {needle}"
        );
    }
}
