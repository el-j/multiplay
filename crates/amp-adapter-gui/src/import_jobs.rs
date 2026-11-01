//! Large imports (the first-run music folder, "Scan ~/Music") on a background thread with a
//! progress count, so the window keeps drawing while thousands of files are read (TASK-171).

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use amp_adapter_storage::import::{import_paths_with_progress, ImportReport};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;

/// An import that is running.
pub struct ImportJob {
    result: Receiver<ImportReport>,
    found: Arc<AtomicUsize>,
}

impl MultiplayGuiApp {
    /// Imports `paths` in the background; [`Self::poll_import`] adds the result.
    pub fn start_import(&mut self, paths: Vec<PathBuf>) {
        if self.importing() || paths.is_empty() {
            return;
        }
        let factory = self.audio_driver.decoder_factory();
        let found = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&found);
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("multiplay-import".into())
            .spawn(move || {
                let fallback = |path: &Path| {
                    let dec = factory.open_file(&path.to_string_lossy()).ok()?;
                    Some(amp_adapter_storage::file_name_track(
                        path,
                        dec.duration(),
                        Some(dec.sample_rate()),
                    ))
                };
                let report = import_paths_with_progress(&paths, &fallback, &|n| {
                    counter.store(n, Ordering::Relaxed);
                });
                let _ = tx.send(report);
            });
        if spawned.is_ok() {
            self.import_job = Some(ImportJob { result: rx, found });
            self.import_notice = Some("Scanning your music…".into());
        }
    }

    /// True while a background import runs.
    #[must_use]
    pub fn importing(&self) -> bool {
        self.import_job.is_some()
    }

    /// Updates the progress line and, once the import is done, adds what it found.
    pub fn poll_import(&mut self) {
        let Some(job) = &self.import_job else {
            return;
        };
        let report = match job.result.try_recv() {
            Ok(report) => report,
            Err(TryRecvError::Empty) => {
                let n = job.found.load(Ordering::Relaxed);
                self.import_notice = Some(format!("Scanning your music… {n} tracks found"));
                return;
            }
            Err(TryRecvError::Disconnected) => ImportReport::default(),
        };
        self.import_job = None;
        let added = report.tracks.len();
        self.remember_in_library(&report.tracks);
        self.playlist.extend(report.tracks);
        let skipped = crate::playback_notice::skipped_summary(&report.skipped);
        self.import_notice = Some(match skipped {
            Some(s) => format!("Added {added} tracks. {s}"),
            None => format!("Added {added} tracks"),
        });
        self.save_session();
    }
}
