//! What this device shares with its mesh: chosen files only, looked up by an id derived from
//! the path and size, so a peer can never ask for a path.

#![forbid(unsafe_code)]

use crate::protocol::SharedTrack;
use amp_domain::TrackMetadata;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

/// A file offered to the mesh.
#[derive(Debug, Clone)]
pub struct SharedFile {
    pub path: PathBuf,
    pub metadata: TrackMetadata,
}

/// Results returned for one search.
const MAX_RESULTS: usize = 50;

/// The shared files by id.
#[derive(Debug, Default)]
pub struct Catalog {
    files: HashMap<String, (SharedFile, u64)>,
}

fn file_id(path: &std::path::Path, size: u64) -> String {
    let mut h = Sha256::new();
    h.update(path.to_string_lossy().as_bytes());
    h.update(size.to_le_bytes());
    h.finalize()[..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl Catalog {
    /// Replaces the shared files (missing files are left out).
    pub fn set(&mut self, files: Vec<SharedFile>) {
        self.files = files
            .into_iter()
            .filter_map(|f| {
                let size = std::fs::metadata(&f.path)
                    .ok()
                    .filter(|m| m.is_file())?
                    .len();
                Some((file_id(&f.path, size), (f, size)))
            })
            .collect();
    }

    /// Shared files matching `query` (every word in title, artist or album).
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<SharedTrack> {
        let words: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if words.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<SharedTrack> = self
            .files
            .iter()
            .filter(|(_, (f, _))| {
                let m = &f.metadata;
                let hay = format!(
                    "{} {} {}",
                    m.title,
                    m.artist,
                    m.album.as_deref().unwrap_or("")
                )
                .to_lowercase();
                words.iter().all(|w| hay.contains(w))
            })
            .map(|(id, (f, size))| SharedTrack::from_metadata(id.clone(), &f.metadata, *size))
            .collect();
        hits.sort_by(|a, b| (&a.artist, &a.title).cmp(&(&b.artist, &b.title)));
        hits.truncate(MAX_RESULTS);
        hits
    }

    /// The path of shared file `id`.
    #[must_use]
    pub fn path(&self, id: &str) -> Option<(PathBuf, u64)> {
        self.files.get(id).map(|(f, size)| (f.path.clone(), *size))
    }
}
