//! The opt-in update check in the desktop app (TASK-183): when switched on in Preferences →
//! About, at most once a day, on a background thread; a newer release shows as "NEW VERSION"
//! on the faceplate.

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use amp_core_engine::update_check::{check_is_due, newer_release};
use amp_domain::about::ReleaseInfo;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{SystemTime, UNIX_EPOCH};

/// A running check.
pub type UpdateJob = Receiver<Result<Vec<ReleaseInfo>, String>>;

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl MultiplayGuiApp {
    /// Starts a check with GitHub's release list when it is switched on and due.
    pub fn maybe_check_for_updates(&mut self) {
        self.maybe_check_for_updates_with(amp_adapter_http::releases::fetch_releases);
    }

    /// Like [`Self::maybe_check_for_updates`] with another source of releases (tests).
    pub fn maybe_check_for_updates_with(
        &mut self,
        fetch: impl FnOnce() -> Result<Vec<ReleaseInfo>, String> + Send + 'static,
    ) {
        let settings = &self.config.updates;
        if !settings.check_for_updates
            || self.update_job.is_some()
            || !check_is_due(settings.last_checked_epoch, now_epoch())
        {
            return;
        }
        // Remembered before asking, so a crash or an outage does not make it ask again soon.
        self.config.updates.last_checked_epoch = now_epoch();
        self.save_and_apply_config();
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("multiplay-update-check".into())
            .spawn(move || {
                let _ = tx.send(fetch());
            });
        if spawned.is_ok() {
            self.update_job = Some(rx);
        }
    }

    /// True while a check runs.
    #[must_use]
    pub fn update_checking(&self) -> bool {
        self.update_job.is_some()
    }

    /// Picks up a finished check. Failures are only logged: the next try is tomorrow.
    pub fn poll_update_check(&mut self) {
        let Some(job) = &self.update_job else {
            return;
        };
        let result = match job.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err("update check stopped".into()),
        };
        self.update_job = None;
        match result {
            Ok(releases) => {
                let current = env!("MULTIPLAY_VERSION");
                self.update_available =
                    newer_release(current, &releases, self.config.updates.include_betas);
                if let Some(r) = &self.update_available {
                    tracing::info!("update available: {} ({})", r.tag, r.url);
                }
            }
            Err(e) => tracing::info!("update check failed: {e}"),
        }
    }
}
