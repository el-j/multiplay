//! Pure-Rust embedded ACID persistence adapter using Redb (`TASK-148`).
//! Replaces SQLite and RocksDB with zero C compiler or dynamic linkage dependencies.

#![forbid(unsafe_code)]

pub mod analysis;
pub mod artwork;
pub mod cue;
pub mod import;
pub mod keyring;
pub mod paths;
pub mod playlist;
pub mod scanner;
pub mod tags;

use amp_domain::{AudioSource, SourceKind, Track, TrackId};
use amp_ports::driven::LibraryStoragePort;
use amp_ports::StorageError;
use async_trait::async_trait;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

pub use cue::{cue_audio_files, import_cue, parse_cue, CueSheet};
pub use keyring::{KeyringStorageAdapter, SecretString};
pub use paths::{expand_tilde, AppPaths};
pub use playlist::{
    export_m3u8, export_pls, export_xspf, import_m3u8, import_m3u8_with_base, import_pls,
    import_xspf, SessionPlaylistSnapshot,
};
pub use scanner::{
    collect_audio_files, extract_track_from_file, is_playable_path, scan_audio_files,
    LibraryScanner, ScanReport,
};
pub use tags::{embedded_cover_art, file_name_track, CoverArt, EMBEDDED_COVER_PREFIX};

const TRACKS_TABLE: TableDefinition<&[u8; 16], &[u8]> = TableDefinition::new("tracks_v2");
const SOURCE_INDEX_TABLE: TableDefinition<&str, &[u8; 16]> =
    TableDefinition::new("source_index_v2");
const ARTIST_INDEX_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("artist_index_v2");
const ALBUM_INDEX_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("album_index_v2");
const QUARANTINE_TABLE: TableDefinition<&[u8; 16], &[u8]> = TableDefinition::new("quarantine_v1");
pub(crate) const SCAN_STATE_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("scan_state_v1");

/// Current schema version for [`RecordEnvelope`].
pub const CURRENT_RECORD_VERSION: u16 = 2;

/// Versioned record envelope stored in `TRACKS_TABLE` (`TASK-148`).
/// Wraps serialized track data so older records can be migrated seamlessly and
/// schema additions in [`amp_domain::TrackMetadata`] remain backward-compatible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordEnvelope {
    pub version: u16,
    pub payload: Vec<u8>,
}

impl RecordEnvelope {
    /// Encodes a [`Track`] into a v2 [`RecordEnvelope`].
    pub fn encode(track: &Track) -> Result<Vec<u8>, StorageError> {
        let json_payload =
            serde_json::to_vec(track).map_err(|e| StorageError::Serialization(e.to_string()))?;
        let env = Self {
            version: CURRENT_RECORD_VERSION,
            payload: json_payload,
        };
        postcard::to_allocvec(&env).map_err(|e| StorageError::Serialization(e.to_string()))
    }

    /// Decodes a [`Track`] from either a v2 [`RecordEnvelope`] or a legacy v1 postcard payload.
    pub fn decode(raw: &[u8]) -> Result<Track, StorageError> {
        if let Ok(env) = postcard::from_bytes::<Self>(raw) {
            match env.version {
                2 => {
                    if let Ok(track) = serde_json::from_slice::<Track>(&env.payload) {
                        return Ok(track);
                    }
                }
                1 => {
                    if let Ok(track) = postcard::from_bytes::<Track>(&env.payload) {
                        return Ok(track);
                    }
                }
                _ => {}
            }
        }
        // Fallback: legacy un-enveloped v1 postcard record
        postcard::from_bytes::<Track>(raw).map_err(|e| StorageError::Serialization(e.to_string()))
    }
}

/// Embedded ACID library storage adapter backed by pure-Rust `redb`.
pub struct RedbStorageAdapter {
    db: Arc<Database>,
}

impl RedbStorageAdapter {
    /// Opens or creates the persistent `library.redb` inside `paths.data_dir` (`TASK-148`).
    pub fn open_default(
        paths: &AppPaths,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        paths.ensure_dirs()?;
        Self::new(paths.library_db_file())
    }

    /// Creates or opens an existing pure-Rust redb database at the specified path.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(parent) = path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        let db = Database::create(path)?;

        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(TRACKS_TABLE)?;
            let _ = write_txn.open_table(SOURCE_INDEX_TABLE)?;
            let _ = write_txn.open_table(ARTIST_INDEX_TABLE)?;
            let _ = write_txn.open_table(ALBUM_INDEX_TABLE)?;
            let _ = write_txn.open_table(QUARANTINE_TABLE)?;
            let _ = write_txn.open_table(SCAN_STATE_TABLE)?;
            let _ = write_txn.open_table(analysis::ANALYSIS_TABLE)?;
            let _ = write_txn.open_table(analysis::HOT_CUE_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Synchronous upsert used by batch library scanning and tests.
    pub fn store_track_sync(&self, track: &Track) -> Result<(), StorageError> {
        let encoded_bytes = RecordEnvelope::encode(track)?;
        let canonical = track.library_key();
        let id_bytes = *track.id.0.as_bytes();

        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
        {
            let mut tracks = write_txn
                .open_table(TRACKS_TABLE)
                .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
            let mut source_idx = write_txn
                .open_table(SOURCE_INDEX_TABLE)
                .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;

            // If this canonical source was previously stored under another TrackId, remove old entry so rescans upsert cleanly
            let prev_id = source_idx
                .get(canonical.as_str())
                .map_err(|e| StorageError::TransactionFailed(e.to_string()))?
                .map(|g| *g.value());
            if let Some(old_id) = prev_id {
                if old_id != id_bytes {
                    let _ = tracks.remove(&old_id);
                }
            }

            tracks
                .insert(&id_bytes, encoded_bytes.as_slice())
                .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
            source_idx
                .insert(canonical.as_str(), &id_bytes)
                .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
        }
        write_txn
            .commit()
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
        Ok(())
    }

    /// Batch-stores multiple tracks in a single ACID write transaction (`TASK-148`).
    pub fn store_tracks_batch_sync(&self, batch: &[Track]) -> Result<usize, StorageError> {
        if batch.is_empty() {
            return Ok(0);
        }
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
        {
            let mut tracks = write_txn
                .open_table(TRACKS_TABLE)
                .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
            let mut source_idx = write_txn
                .open_table(SOURCE_INDEX_TABLE)
                .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;

            for track in batch {
                let encoded = RecordEnvelope::encode(track)?;
                let canonical = track.library_key();
                let id_bytes = *track.id.0.as_bytes();
                let prev_id = source_idx
                    .get(canonical.as_str())
                    .map_err(|e| StorageError::TransactionFailed(e.to_string()))?
                    .map(|g| *g.value());
                if let Some(old_id) = prev_id {
                    if old_id != id_bytes {
                        let _ = tracks.remove(&old_id);
                    }
                }
                tracks
                    .insert(&id_bytes, encoded.as_slice())
                    .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
                source_idx
                    .insert(canonical.as_str(), &id_bytes)
                    .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
            }
        }
        write_txn
            .commit()
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
        Ok(batch.len())
    }

    /// Synchronous fetch that tolerates and quarantines corrupt records (`TASK-148`).
    pub fn fetch_all_sync(&self) -> Result<Vec<Track>, StorageError> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
        let table = read_txn
            .open_table(TRACKS_TABLE)
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;

        let mut tracks = Vec::new();
        let mut corrupt_entries: Vec<([u8; 16], Vec<u8>)> = Vec::new();

        for item in table
            .iter()
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?
        {
            let (key, value) = item.map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
            match RecordEnvelope::decode(value.value()) {
                Ok(track) => tracks.push(track),
                Err(_) => {
                    corrupt_entries.push((*key.value(), value.value().to_vec()));
                }
            }
        }
        drop(table);
        drop(read_txn);

        if !corrupt_entries.is_empty() {
            if let Ok(write_txn) = self.db.begin_write() {
                if let (Ok(mut tracks_tbl), Ok(mut quarantine_tbl)) = (
                    write_txn.open_table(TRACKS_TABLE),
                    write_txn.open_table(QUARANTINE_TABLE),
                ) {
                    for (k, raw) in &corrupt_entries {
                        let _ = tracks_tbl.remove(k);
                        let _ = quarantine_tbl.insert(k, raw.as_slice());
                    }
                }
                let _ = write_txn.commit();
            }
        }

        Ok(tracks)
    }

    /// Returns the number of corrupt records moved to the quarantine table.
    pub fn quarantined_count(&self) -> Result<usize, StorageError> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
        let table = read_txn
            .open_table(QUARANTINE_TABLE)
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
        let count = table
            .iter()
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?
            .count();
        Ok(count)
    }

    /// Queries tracks by artist (case-insensitive match).
    pub fn tracks_by_artist(&self, artist: &str) -> Result<Vec<Track>, StorageError> {
        let needle = artist.to_lowercase();
        Ok(self
            .fetch_all_sync()?
            .into_iter()
            .filter(|t| t.metadata.artist.to_lowercase() == needle)
            .collect())
    }

    /// Queries tracks by album (case-insensitive match).
    pub fn tracks_by_album(&self, album: &str) -> Result<Vec<Track>, StorageError> {
        let needle = album.to_lowercase();
        Ok(self
            .fetch_all_sync()?
            .into_iter()
            .filter(|t| {
                t.metadata
                    .album
                    .as_deref()
                    .map(|a| a.to_lowercase() == needle)
                    .unwrap_or(false)
            })
            .collect())
    }

    /// Queries tracks by [`SourceKind`].
    pub fn tracks_by_source_kind(&self, kind: SourceKind) -> Result<Vec<Track>, StorageError> {
        Ok(self
            .fetch_all_sync()?
            .into_iter()
            .filter(|t| t.source.kind() == kind)
            .collect())
    }

    /// Looks up a track by its canonical [`AudioSource`] locator in O(1) via `SOURCE_INDEX_TABLE`.
    pub fn track_by_source(&self, source: &AudioSource) -> Result<Option<Track>, StorageError> {
        let canonical = source.canonical_uri();
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
        let source_idx = read_txn
            .open_table(SOURCE_INDEX_TABLE)
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
        let Some(id_guard) = source_idx
            .get(canonical.as_str())
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?
        else {
            return Ok(None);
        };
        let id_bytes = *id_guard.value();
        let tracks = read_txn
            .open_table(TRACKS_TABLE)
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
        let Some(val_guard) = tracks
            .get(&id_bytes)
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?
        else {
            return Ok(None);
        };
        RecordEnvelope::decode(val_guard.value()).map(Some)
    }
}

#[async_trait]
impl LibraryStoragePort for RedbStorageAdapter {
    async fn store_track(&self, track: &Track) -> Result<(), StorageError> {
        self.store_track_sync(track)
    }

    async fn fetch_all(&self) -> Result<Vec<Track>, StorageError> {
        self.fetch_all_sync()
    }

    async fn remove_track(&self, id: &TrackId) -> Result<(), StorageError> {
        let id_bytes = *id.0.as_bytes();
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(TRACKS_TABLE)
                .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
            let removed_canonical = table
                .get(&id_bytes)
                .ok()
                .flatten()
                .and_then(|g| RecordEnvelope::decode(g.value()).ok())
                .map(|t| t.source.canonical_uri());
            table
                .remove(&id_bytes)
                .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
            if let Some(uri) = removed_canonical {
                if let Ok(mut source_idx) = write_txn.open_table(SOURCE_INDEX_TABLE) {
                    let _ = source_idx.remove(uri.as_str());
                }
            }
        }
        write_txn
            .commit()
            .map_err(|e| StorageError::TransactionFailed(e.to_string()))?;
        Ok(())
    }

    async fn search_cached(&self, text_query: &str) -> Result<Vec<Track>, StorageError> {
        let all_tracks = self.fetch_all_sync()?;
        let q = text_query.to_lowercase();

        Ok(all_tracks
            .into_iter()
            .filter(|t| {
                t.metadata.title.to_lowercase().contains(&q)
                    || t.metadata.artist.to_lowercase().contains(&q)
                    || t.metadata
                        .album
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&q)
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::{AudioSource, TrackMetadata};
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_redb_persistence_roundtrip() {
        let tmp = NamedTempFile::new().expect("create temp file failed");
        let storage = RedbStorageAdapter::new(tmp.path()).expect("init redb failed");

        let track = Track::with_stable_id(
            AudioSource::LocalFile {
                path: "/music/selected_ambient_works.flac".to_string(),
            },
            TrackMetadata::new("Xtal", "Aphex Twin"),
        );
        let track_id = track.id;

        storage
            .store_track(&track)
            .await
            .expect("store track failed");

        let all = storage.fetch_all().await.expect("fetch all failed");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].metadata.title, "Xtal");

        let results = storage.search_cached("xtal").await.expect("search failed");
        assert_eq!(results.len(), 1);

        let empty = storage
            .search_cached("nonexistent")
            .await
            .expect("search failed");
        assert_eq!(empty.len(), 0);

        storage
            .remove_track(&track_id)
            .await
            .expect("remove track failed");
        let after_removal = storage
            .fetch_all()
            .await
            .expect("fetch after removal failed");
        assert_eq!(after_removal.len(), 0);
    }

    #[test]
    fn rescan_of_10k_unchanged_tracks_adds_zero_duplicates_and_quarantines_corrupt_records() {
        let tmp = NamedTempFile::new().unwrap();
        let storage = RedbStorageAdapter::new(tmp.path()).unwrap();

        let tracks: Vec<Track> = (0..10_000)
            .map(|i| {
                Track::with_stable_id(
                    AudioSource::LocalFile {
                        path: format!("/library/track_{i:05}.flac"),
                    },
                    TrackMetadata::new(format!("Song {i}"), "Artist"),
                )
            })
            .collect();

        storage.store_tracks_batch_sync(&tracks).unwrap();
        assert_eq!(storage.fetch_all_sync().unwrap().len(), 10_000);

        // Rescanning the exact same 10k files (even if constructed via Track::new with fresh UUIDs)
        // upserts by canonical source locator and adds 0 duplicate records!
        let rescanned: Vec<Track> = (0..10_000)
            .map(|i| {
                Track::new(
                    AudioSource::LocalFile {
                        path: format!("/library/track_{i:05}.flac"),
                    },
                    TrackMetadata::new(format!("Song {i}"), "Artist"),
                )
            })
            .collect();
        storage.store_tracks_batch_sync(&rescanned).unwrap();
        assert_eq!(storage.fetch_all_sync().unwrap().len(), 10_000);

        // Inject a corrupt record directly into TRACKS_TABLE and verify fetch_all_sync quarantines it
        let corrupt_id = [0xEEu8; 16];
        let write_txn = storage.db.begin_write().unwrap();
        {
            let mut tbl = write_txn.open_table(TRACKS_TABLE).unwrap();
            tbl.insert(&corrupt_id, b"corrupt-garbage-bytes".as_slice())
                .unwrap();
        }
        write_txn.commit().unwrap();

        let fetched = storage.fetch_all_sync().unwrap();
        assert_eq!(fetched.len(), 10_000);
        assert_eq!(storage.quarantined_count().unwrap(), 1);
    }
}
