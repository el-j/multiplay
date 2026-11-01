//! Installing slskd on request: the official release zip for this system from slskd's GitHub
//! releases, checked against the SHA-256 GitHub publishes for it (pinned here), unpacked into the
//! app's own folder. slskd is AGPL-3.0 and runs as a separate program; multiplay neither bundles
//! nor links it (ADR-008).

#![forbid(unsafe_code)]

use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

/// The slskd release multiplay installs.
pub const SLSKD_VERSION: &str = "0.26.2";

/// One release zip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub name: String,
    /// Lower-case hex SHA-256, from the release's published asset digests.
    pub sha256: &'static str,
}

/// The release for `os` / `arch` (as in `std::env::consts`), if slskd publishes one.
#[must_use]
pub fn asset_for(os: &str, arch: &str) -> Option<ReleaseAsset> {
    let (platform, sha256) = match (os, arch) {
        ("macos", "aarch64") => (
            "osx-arm64",
            "18d3ebf7760cd93296b61f640c87b383b41a142cd445a04a633d3b596313ef85",
        ),
        ("macos", "x86_64") => (
            "osx-x64",
            "1df5c0cbe3b44dd6fae78f2461b5dd3353930a3c5b4128a8309d63b6bd435d95",
        ),
        ("linux", "x86_64") => (
            "linux-x64",
            "53102e91e0123f62d17627d5e02a24d9dcd08a165e75182c0f1d76d087510f37",
        ),
        ("linux", "aarch64") => (
            "linux-arm64",
            "b8acb302df08f7751f2068c779380dec136882adb5af82a081cd0ead8f028a51",
        ),
        ("windows", "x86_64") => (
            "win-x64",
            "ffe186ecbedce15bf19b6a690c6213c669f34acc793ef53aa92390a5cd9c48f0",
        ),
        _ => return None,
    };
    Some(ReleaseAsset {
        name: format!("slskd-{SLSKD_VERSION}-{platform}.zip"),
        sha256,
    })
}

/// The release for the running system.
#[must_use]
pub fn asset_for_this_system() -> Option<ReleaseAsset> {
    asset_for(std::env::consts::OS, std::env::consts::ARCH)
}

/// Where the zip is downloaded from.
#[must_use]
pub fn download_url(asset: &ReleaseAsset) -> String {
    format!(
        "https://github.com/slskd/slskd/releases/download/{SLSKD_VERSION}/{}",
        asset.name
    )
}

fn executable_name() -> &'static str {
    if cfg!(windows) {
        "slskd.exe"
    } else {
        "slskd"
    }
}

/// Checks `archive` against `sha256` and unpacks it into `dir`; returns the slskd executable.
///
/// # Errors
/// A checksum mismatch, a damaged zip, an entry pointing outside `dir`, or no executable.
pub fn install_archive(dir: &Path, archive: &[u8], sha256: &str) -> Result<PathBuf, String> {
    let actual: String = Sha256::digest(archive)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if actual != sha256 {
        return Err(format!(
            "the download's checksum does not match ({actual}); not installed"
        ));
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive)).map_err(|e| e.to_string())?;
    // Check every name first, so nothing is written from an archive that is refused.
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|e| e.to_string())?;
        if entry.enclosed_name().is_none() {
            return Err(format!(
                "{} would be written outside the install folder",
                entry.name().map(|n| n.into_owned()).unwrap_or_default()
            ));
        }
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = entry.enclosed_name() else {
            continue;
        };
        let target = dir.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        std::fs::write(&target, bytes).map_err(|e| e.to_string())?;
    }
    let exe = dir.join(executable_name());
    if !exe.is_file() {
        return Err("the archive has no slskd executable".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    Ok(exe)
}

/// Downloads the release for this system and installs it into `dir` (blocking; call it off the
/// UI thread).
///
/// # Errors
/// No release for this system, a network failure, or an install failure.
pub fn install(dir: &Path) -> Result<PathBuf, String> {
    let asset = asset_for_this_system().ok_or("slskd has no release for this system")?;
    amp_adapter_http::install_ring_provider();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let bytes = runtime.block_on(async {
        let client = reqwest::Client::builder()
            .user_agent(amp_domain::about::user_agent())
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client
            .get(download_url(&asset))
            .send()
            .await
            .map_err(|e| format!("download failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("download failed: {}", resp.status()));
        }
        resp.bytes()
            .await
            .map_err(|e| format!("download failed: {e}"))
    })?;
    install_archive(&dir.join(SLSKD_VERSION), &bytes, asset.sha256)
}
