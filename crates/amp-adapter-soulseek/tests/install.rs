//! Installing and running slskd on request (TASK-162): the official release for this system,
//! checked against a pinned SHA-256, unpacked safely, started with settings in its environment
//! (the password never on the command line), stopped with multiplay.

#![forbid(unsafe_code)]

use amp_adapter_soulseek::daemon::generate_api_key;
#[cfg(unix)]
use amp_adapter_soulseek::daemon::{ManagedConfig, ManagedSlskd};
use amp_adapter_soulseek::install::{asset_for, download_url, install_archive, SLSKD_VERSION};
use sha2::{Digest, Sha256};
use std::io::Write;

fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut out);
        let opts = zip::write::SimpleFileOptions::default();
        for (name, data) in entries {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    out.into_inner()
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn every_supported_system_has_a_pinned_release() {
    for (os, arch, name) in [
        ("macos", "aarch64", "osx-arm64"),
        ("macos", "x86_64", "osx-x64"),
        ("linux", "x86_64", "linux-x64"),
        ("linux", "aarch64", "linux-arm64"),
        ("windows", "x86_64", "win-x64"),
    ] {
        let asset = asset_for(os, arch).unwrap_or_else(|| panic!("{os} {arch}"));
        assert!(
            asset.name.contains(name) && asset.name.contains(SLSKD_VERSION),
            "{}",
            asset.name
        );
        assert_eq!(asset.sha256.len(), 64);
        assert!(
            download_url(&asset).starts_with("https://github.com/slskd/slskd/releases/download/")
        );
    }
    assert!(asset_for("freebsd", "x86_64").is_none());
}

#[test]
fn a_verified_archive_is_unpacked_and_a_tampered_one_refused() {
    let dir = tempfile::tempdir().unwrap();
    let exe_name = if cfg!(windows) { "slskd.exe" } else { "slskd" };
    let archive = zip_of(&[
        (exe_name, b"#!/bin/sh\n"),
        ("wwwroot/index.html", b"<html>"),
    ]);
    let wrong = "0".repeat(64);
    let err = install_archive(dir.path(), &archive, &wrong).expect_err("refused");
    assert!(err.contains("checksum"), "{err}");
    assert!(!dir.path().join(exe_name).exists());

    let exe = install_archive(dir.path(), &archive, &hex(&archive)).expect("installed");
    assert!(exe.is_file() && exe.starts_with(dir.path()));
    assert!(dir.path().join("wwwroot/index.html").is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert!(
            std::fs::metadata(&exe).unwrap().permissions().mode() & 0o111 != 0,
            "executable"
        );
    }
}

#[test]
fn entries_that_escape_the_install_folder_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let exe_name = if cfg!(windows) { "slskd.exe" } else { "slskd" };
    let archive = zip_of(&[(exe_name, b"x"), ("../../escape.txt", b"x")]);
    let err = install_archive(dir.path(), &archive, &hex(&archive)).expect_err("refused");
    assert!(err.contains("outside"), "{err}");
    assert!(!dir.path().join("../../escape.txt").exists());
}

#[test]
fn api_keys_are_long_and_random() {
    let (a, b) = (generate_api_key(), generate_api_key());
    assert!(a.len() >= 32 && a.len() <= 255 && a != b);
}

#[cfg(unix)]
#[test]
fn the_managed_slskd_gets_its_settings_from_the_environment_and_stops_with_us() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("env.txt");
    let exe = dir.path().join("slskd");
    std::fs::write(
        &exe,
        format!(
            "#!/bin/sh\nenv > '{}'\necho \"$@\" > '{}.args'\nexec sleep 30\n",
            report.display(),
            report.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    let config = ManagedConfig {
        exe,
        app_dir: dir.path().join("app"),
        port: 50_399,
        username: "listener".into(),
        password: "hunter2".into(),
        api_key: generate_api_key(),
        downloads: dir.path().join("downloads"),
    };
    let managed = ManagedSlskd::start(&config).expect("starts");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !report.is_file() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    std::thread::sleep(std::time::Duration::from_millis(100));
    let env = std::fs::read_to_string(&report).unwrap();
    for needle in [
        "SLSKD_HTTP_IP_ADDRESS=127.0.0.1",
        "SLSKD_HTTP_PORT=50399",
        "SLSKD_NO_HTTPS=true",
        "SLSKD_SLSK_USERNAME=listener",
        "SLSKD_SLSK_PASSWORD=hunter2",
        &format!("SLSKD_API_KEY={}", config.api_key),
    ] {
        assert!(env.contains(needle), "missing {needle}");
    }
    let args = std::fs::read_to_string(format!("{}.args", report.display())).unwrap();
    assert!(
        !args.contains("hunter2"),
        "the password is not on the command line: {args}"
    );
    assert_eq!(managed.url(), "http://127.0.0.1:50399");
    let pid = managed.pid();
    drop(managed);
    std::thread::sleep(std::time::Duration::from_millis(200));
    let alive = std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .unwrap()
        .success();
    assert!(!alive, "slskd stops with multiplay");
}
