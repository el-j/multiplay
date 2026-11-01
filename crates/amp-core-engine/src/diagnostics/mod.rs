//! Field diagnostics for beta testers (TASK-177, TASK-178): log files, crash reports and a
//! redacted diagnostics bundle. Shared by the GUI and TUI composition roots.

#![forbid(unsafe_code)]

mod crash;
mod logging;
mod report;

pub use crash::{crash_report, install_crash_hook, CrashCleanup};
pub use logging::{file_subscriber, init_logging, LoggingGuard, LOG_FILE_PREFIX};
pub use report::{diagnostics_text, redact_config, write_diagnostics_zip};

/// The settings as they are saved in `config.toml`, for the diagnostics.
#[must_use]
pub fn config_toml(config: &amp_domain::AppConfig) -> String {
    toml::to_string(config).unwrap_or_else(|e| format!("# settings could not be written: {e}"))
}

/// Which front end is running, for logs, crash reports and diagnostics.
#[derive(Debug, Clone)]
pub struct AppInfo {
    /// `multiplay` or `multiplay-tui`.
    pub name: &'static str,
    /// `multiplay 0.1.0-beta.1 (1a2b3c4)`.
    pub version_line: String,
}
