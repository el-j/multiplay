//! File dialogs on their own thread (TASK-175). The platform dialogs are separate processes
//! (osascript, zenity, PowerShell) that block until the user picks; run on the UI thread they
//! froze every window, including the visualizer, for as long as the dialog was open.

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// A dialog that is open; its paths arrive when the user picks or cancels.
pub type DialogJob = Receiver<Vec<PathBuf>>;

impl MultiplayGuiApp {
    /// Runs `picker` on a background thread; its paths are added by [`Self::poll_dialog`]. Ignored
    /// while another dialog is open.
    pub fn start_dialog(&mut self, picker: impl FnOnce() -> Vec<PathBuf> + Send + 'static) {
        if self.dialog_open() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("multiplay-file-dialog".into())
            .spawn(move || {
                let _ = tx.send(picker());
            });
        if spawned.is_ok() {
            self.dialog_job = Some(rx);
        }
    }

    /// True while a dialog is open.
    #[must_use]
    pub fn dialog_open(&self) -> bool {
        self.dialog_job.is_some()
    }

    /// Adds what the user picked, once they have. A cancelled dialog adds nothing and scans
    /// nothing (audit UX-07).
    pub fn poll_dialog(&mut self) {
        let Some(job) = &self.dialog_job else {
            return;
        };
        let picked = match job.try_recv() {
            Ok(paths) => paths,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Vec::new(),
        };
        self.dialog_job = None;
        if std::mem::take(&mut self.dialog_for_onboarding) {
            if let Some(ob) = self.onboarding.as_mut() {
                for p in picked {
                    if !ob.folders.contains(&p) {
                        ob.folders.push(p);
                    }
                }
            }
            return;
        }
        if !picked.is_empty() {
            self.ingest_paths(&picked);
        }
    }

    /// The add dialog: audio files (and folders on macOS), playlists, CUE sheets, skins.
    pub fn open_file_dialog(&mut self) {
        self.start_dialog(crate::file_dialog::pick_audio_files);
    }

    /// The folder dialog for the first-run setup: the folders go into its list, not the playlist.
    pub fn start_dialog_for_onboarding(&mut self) {
        if !self.dialog_open() {
            self.start_dialog(crate::file_dialog::pick_folders);
            self.dialog_for_onboarding = true;
        }
    }

    /// The folder dialog: whole music folders.
    pub fn open_folder_dialog(&mut self) {
        self.start_dialog(crate::file_dialog::pick_folders);
    }
}
