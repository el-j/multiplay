//! Pure-Rust embedded ACID persistence adapter using Redb.
//! Replaces SQLite and RocksDB with zero C compiler or dynamic linkage dependencies.

#![forbid(unsafe_code)]

pub mod analysis;
pub mod keyring;
pub mod playlist;
pub mod scanner;

use amp_domain::{Track, TrackId};
use amp_ports::driven::LibraryStoragePort;
use async_trait::async_trait;

pub use keyring::{KeyringStorageAdapter, SecretString};
pub use playlist::{export_m3u8, import_m3u8, SessionPlaylistSnapshot};
use redb::{Database, ReadableTable, TableDefinition};
pub use scanner::{collect_audio_files, extract_track_from_file, LibraryScanner};
use std::path::Path;
use std::sync::Arc;

const TRACKS_TABLE: TableDefinition<&[u8; 16], &[u8]> = TableDefinition::new("tracks_v1");
const INDEX_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("search_index_v1");

pub struct RedbStorageAdapter {
    db: Arc<Database>,
}

impl RedbStorageAdapter {
    /// Creates or opens an existing pure-Rust redb database at the specified path.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let db = Database::create(path)?;

        // Ensure tables exist within an initial transaction
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(TRACKS_TABLE)?;
            let _ = write_txn.open_table(INDEX_TABLE)?;
            let _ = write_txn.open_table(analysis::ANALYSIS_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self { db: Arc::new(db) })
    }
}

#[async_trait]
impl LibraryStoragePort for RedbStorageAdapter {
    async fn store_track(&self, track: &Track) -> Result<(), String> {
        let db = Arc::clone(&self.db);
        let track_clone = track.clone();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let encoded_bytes = bincode::serialize(&track_clone)
                .map_err(|e| format!("Track serialization failure: {e}"))?;

            let write_txn = db
                .begin_write()
                .map_err(|e| format!("DB write transaction error: {e}"))?;
            {
                let mut table = write_txn
                    .open_table(TRACKS_TABLE)
                    .map_err(|e| e.to_string())?;
                table
                    .insert(track_clone.id.0.as_bytes(), encoded_bytes.as_slice())
                    .map_err(|e| e.to_string())?;
            }
            write_txn.commit().map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e| format!("Tokio task join panic: {e}"))?
    }

    async fn fetch_all(&self) -> Result<Vec<Track>, String> {
        let db = Arc::clone(&self.db);

        tokio::task::spawn_blocking(move || -> Result<Vec<Track>, String> {
            let read_txn = db
                .begin_read()
                .map_err(|e| format!("DB read transaction error: {e}"))?;
            let table = read_txn
                .open_table(TRACKS_TABLE)
                .map_err(|e| e.to_string())?;

            let mut tracks = Vec::new();
            for item in table.iter().map_err(|e| e.to_string())? {
                let (_, value) = item.map_err(|e| e.to_string())?;
                let track: Track = bincode::deserialize(value.value())
                    .map_err(|e| format!("Track deserialization failure: {e}"))?;
                tracks.push(track);
            }
            Ok(tracks)
        })
        .await
        .map_err(|e| format!("Tokio task join panic: {e}"))?
    }

    async fn remove_track(&self, id: &TrackId) -> Result<(), String> {
        let db = Arc::clone(&self.db);
        let id_bytes = *id.0.as_bytes();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let write_txn = db
                .begin_write()
                .map_err(|e| format!("DB write transaction error: {e}"))?;
            {
                let mut table = write_txn
                    .open_table(TRACKS_TABLE)
                    .map_err(|e| e.to_string())?;
                table.remove(&id_bytes).map_err(|e| e.to_string())?;
            }
            write_txn.commit().map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e| format!("Tokio task join panic: {e}"))?
    }

    async fn search_cached(&self, text_query: &str) -> Result<Vec<Track>, String> {
        let all_tracks = self.fetch_all().await?;
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

        let track = Track::new(
            AudioSource::LocalFile {
                path: "/music/selected_ambient_works.flac".to_string(),
            },
            TrackMetadata::new("Xtal", "Aphex Twin"),
        );
        let track_id = track.id;

        // Store track
        storage
            .store_track(&track)
            .await
            .expect("store track failed");

        // Fetch all
        let all = storage.fetch_all().await.expect("fetch all failed");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].metadata.title, "Xtal");

        // Search cached
        let results = storage.search_cached("xtal").await.expect("search failed");
        assert_eq!(results.len(), 1);

        let empty = storage
            .search_cached("nonexistent")
            .await
            .expect("search failed");
        assert_eq!(empty.len(), 0);

        // Remove track
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
}
