//! Panic hook: a crash report in the log folder, then the front end's cleanup, then the default
//! panic message.

#![forbid(unsafe_code)]

use super::AppInfo;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Runs before the default hook prints, e.g. to leave the TUI's raw mode and alternate screen.
pub type CrashCleanup = Box<dyn Fn() + Send + Sync>;

/// The text of a crash report.
#[must_use]
pub fn crash_report(
    app: &AppInfo,
    message: &str,
    location: Option<&str>,
    backtrace: &str,
) -> String {
    let thread = std::thread::current();
    format!(
        "{} crashed.\n\n\
         Please attach this file to an issue: https://github.com/el-j/multiplay/issues\n\n\
         App:      {}\n\
         System:   {} {}\n\
         Thread:   {}\n\
         Message:  {message}\n\
         Location: {}\n\n\
         Backtrace:\n{backtrace}\n",
        app.name,
        app.version_line,
        std::env::consts::OS,
        std::env::consts::ARCH,
        thread.name().unwrap_or("unnamed"),
        location.unwrap_or("unknown"),
    )
}

/// Installs the crash hook. The previous hook still runs afterwards.
pub fn install_crash_hook(app: AppInfo, log_dir: PathBuf, cleanup: Option<CrashCleanup>) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if amp_ports::panic_is_contained() {
            // Caught by `contain_panic` (a damaged file): the caller turns it into an error.
            tracing::warn!("contained panic: {info}");
            return;
        }
        if let Some(cleanup) = &cleanup {
            cleanup();
        }
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(no message)".to_string());
        let location = info.location().map(ToString::to_string);
        let backtrace = std::backtrace::Backtrace::force_capture().to_string();
        let report = crash_report(&app, &message, location.as_deref(), &backtrace);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let file = log_dir.join(format!("crash-{stamp}.txt"));
        let written =
            std::fs::create_dir_all(&log_dir).and_then(|()| std::fs::write(&file, report));
        tracing::error!(
            message = %message,
            location = location.as_deref().unwrap_or("unknown"),
            "panic; crash report {}",
            if written.is_ok() { file.display().to_string() } else { "not written".to_string() }
        );
        previous(info);
    }));
}
