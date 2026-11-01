//! The library behind the browser (TASK-172): every imported track is stored in the library
//! database once, dated when it was first added, and indexed for the Library window. The stored
//! library loads on a background thread at start-up.

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use amp_core_engine::library_index::LibraryIndex;
use amp_domain::{Track, TrackId};
use std::collections::HashSet;
use std::sync::mpsc::{self, TryRecvError};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl MultiplayGuiApp {
    /// Starts loading the stored library in the background; [`Self::poll_library`] picks it up.
    pub fn load_library(&mut self) {
        if self.library_job.is_some() || self.library_index.is_some() {
            return;
        }
        let Some(db) = self.library() else {
            self.library_index = Some(LibraryIndex::build(Vec::new()));
            return;
        };
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("multiplay-library-load".into())
            .spawn(move || {
                let tracks = db.fetch_all_sync().unwrap_or_else(|e| {
                    tracing::warn!("library could not be read: {e}");
                    Vec::new()
                });
                let _ = tx.send(tracks);
            });
        if spawned.is_ok() {
            self.library_job = Some(rx);
        }
    }

    /// True while the stored library is still loading.
    #[must_use]
    pub fn library_loading(&self) -> bool {
        self.library_job.is_some()
    }

    /// Installs the loaded library, plus anything imported while it loaded.
    pub fn poll_library(&mut self) {
        let Some(job) = &self.library_job else {
            return;
        };
        let mut tracks = match job.try_recv() {
            Ok(tracks) => tracks,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Vec::new(),
        };
        self.library_job = None;
        let mut known: HashSet<String> = tracks.iter().map(Track::library_key).collect();
        tracks.extend(
            std::mem::take(&mut self.library_pending)
                .into_iter()
                .filter(|t| known.insert(t.library_key())),
        );
        tracing::info!(tracks = tracks.len(), "library loaded");
        self.set_library(tracks);
    }

    fn set_library(&mut self, tracks: Vec<Track>) {
        self.library_index = Some(LibraryIndex::build(tracks));
        self.library_generation += 1;
    }

    /// Stores tracks that are not in the library yet, dated now. Tracks already there keep the
    /// date they were first added.
    pub fn remember_in_library(&mut self, tracks: &[Track]) {
        let mut known: HashSet<String> = self
            .library_index
            .as_ref()
            .map(|i| i.tracks().iter().map(Track::library_key).collect())
            .unwrap_or_default();
        let now = now_epoch();
        let new: Vec<Track> = tracks
            .iter()
            .filter(|t| known.insert(t.library_key()))
            .map(|t| {
                let mut t = t.clone();
                t.id = TrackId::from_key(&t.library_key());
                if t.date_added_epoch == 0 {
                    t.date_added_epoch = now;
                }
                t
            })
            .collect();
        if new.is_empty() {
            return;
        }
        if let Some(db) = self.library() {
            if let Err(e) = db.store_tracks_batch_sync(&new) {
                tracing::warn!("library: {} tracks not stored: {e}", new.len());
            }
        }
        match self.library_index.take() {
            Some(index) => {
                let mut all = index.tracks().to_vec();
                all.extend(new);
                self.set_library(all);
            }
            None => self.library_pending.extend(new),
        }
    }

    /// Appends library rows (indices into the index's tracks) to the playlist, in that order.
    pub fn library_add_rows(&mut self, rows: &[usize]) {
        let Some(index) = &self.library_index else {
            return;
        };
        let tracks: Vec<Track> = rows
            .iter()
            .filter_map(|&i| index.tracks().get(i).cloned())
            .collect();
        self.playlist.extend(tracks);
        self.save_session();
    }

    /// Appends library rows and plays the first of them.
    pub fn library_play_rows(&mut self, rows: &[usize]) {
        let first = self.playlist.len();
        self.library_add_rows(rows);
        if first < self.playlist.len() {
            self.play_index(first);
        }
    }
}
