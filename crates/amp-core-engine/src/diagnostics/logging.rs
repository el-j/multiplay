//! Structured logging to daily rolling files (`multiplay.YYYY-MM-DD.log`, seven kept).

#![forbid(unsafe_code)]

use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::{fmt, EnvFilter};

/// Log files are `<prefix>.<date>.log`.
pub const LOG_FILE_PREFIX: &str = "multiplay";

/// Days of logs kept in the log folder.
const KEPT_LOG_FILES: usize = 7;

/// Keeps the background log writer alive; dropping it flushes what is buffered.
pub struct LoggingGuard {
    _worker: WorkerGuard,
}

/// A subscriber that writes to the log folder at `default_level`, or at `RUST_LOG` when set.
///
/// # Errors
/// When the log folder cannot be created.
pub fn file_subscriber(
    dir: &Path,
    default_level: &str,
) -> std::io::Result<(impl tracing::Subscriber + Send + Sync, LoggingGuard)> {
    std::fs::create_dir_all(dir)?;
    let appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix(LOG_FILE_PREFIX)
        .filename_suffix("log")
        .max_log_files(KEPT_LOG_FILES)
        .build(dir)
        .map_err(std::io::Error::other)?;
    let (writer, worker) = tracing_appender::non_blocking(appender);
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));
    let subscriber = fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .with_thread_names(true)
        .finish();
    Ok((subscriber, LoggingGuard { _worker: worker }))
}

/// Installs the file subscriber for the whole process. Logging problems never stop the app: on
/// failure it runs without logs and says why on stderr.
#[must_use]
pub fn init_logging(dir: &Path, default_level: &str) -> Option<LoggingGuard> {
    match file_subscriber(dir, default_level) {
        Ok((subscriber, guard)) => {
            tracing::subscriber::set_global_default(subscriber).ok()?;
            Some(guard)
        }
        Err(e) => {
            // The log file is what failed, so stderr is the only place left to say so.
            eprintln!("multiplay: logging disabled ({}): {e}", dir.display());
            None
        }
    }
}
