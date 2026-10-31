//! The diagnostics a tester pastes or attaches: version, system, redacted config, log tail and
//! the latest crash report.

#![forbid(unsafe_code)]

use super::{AppInfo, LOG_FILE_PREFIX};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Lines of the newest log included in the pasted text.
const LOG_TAIL_LINES: usize = 200;

/// Key fragments whose values never leave the computer.
const SECRET_KEYS: [&str; 6] = [
    "password",
    "secret",
    "token",
    "credential",
    "api_key",
    "cookie",
];

/// The config with every secret-looking value replaced by `"<redacted>"`.
#[must_use]
pub fn redact_config(toml: &str) -> String {
    toml.lines()
        .map(|line| {
            let Some((key, _)) = line.split_once('=') else {
                return line.to_string();
            };
            let lower = key.trim().to_ascii_lowercase();
            if SECRET_KEYS.iter().any(|s| lower.contains(s)) {
                format!("{} = \"<redacted>\"", key.trim_end())
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn files_starting_with(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with(prefix))
        })
        .collect();
    // Dates and Unix timestamps in the names sort chronologically.
    files.sort();
    files
}

fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

/// Plain text for an issue: version, system, the latest crash, the redacted config and the end
/// of the newest log.
#[must_use]
pub fn diagnostics_text(app: &AppInfo, config_toml: &str, log_dir: &Path) -> String {
    let mut out = format!(
        "{}\nSystem: {} {}\nLogs: {}\n",
        app.version_line,
        std::env::consts::OS,
        std::env::consts::ARCH,
        log_dir.display()
    );
    if let Some(crash) = files_starting_with(log_dir, "crash-").last() {
        let text = std::fs::read_to_string(crash).unwrap_or_default();
        out.push_str(&format!("\n--- Latest crash ({}) ---\n", crash.display()));
        out.push_str(&tail(&text, 60));
        out.push('\n');
    }
    out.push_str("\n--- Settings (secrets redacted) ---\n");
    out.push_str(&redact_config(config_toml));
    out.push('\n');
    if let Some(log) = files_starting_with(log_dir, LOG_FILE_PREFIX)
        .into_iter()
        .rfind(|p| p.extension().is_some_and(|e| e == "log"))
    {
        let text = std::fs::read_to_string(&log).unwrap_or_default();
        out.push_str(&format!("\n--- Log, last {LOG_TAIL_LINES} lines ---\n"));
        out.push_str(&tail(&text, LOG_TAIL_LINES));
        out.push('\n');
    }
    out
}

/// Writes `multiplay-diagnostics-<time>.zip` into the log folder: `diagnostics.txt`, every log
/// file and every crash report. Returns its path.
///
/// # Errors
/// When the zip cannot be written.
pub fn write_diagnostics_zip(
    app: &AppInfo,
    config_toml: &str,
    log_dir: &Path,
) -> std::io::Result<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let path = log_dir.join(format!("multiplay-diagnostics-{stamp}.zip"));
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path)?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("diagnostics.txt", options)
        .map_err(std::io::Error::other)?;
    zip.write_all(diagnostics_text(app, config_toml, log_dir).as_bytes())?;
    let logs = files_starting_with(log_dir, LOG_FILE_PREFIX)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "log"));
    for file in logs.chain(files_starting_with(log_dir, "crash-")) {
        let name = file.file_name().unwrap_or_default().to_string_lossy();
        zip.start_file(name, options)
            .map_err(std::io::Error::other)?;
        zip.write_all(&std::fs::read(&file)?)?;
    }
    zip.finish().map_err(std::io::Error::other)?;
    Ok(path)
}
