# Local Library Indexing, Pure-Rust Embedded DB & CQRS State Engine

This document defines the persistent metadata cache, local folder scanner, embedded pure-Rust transactional database (`redb`), and the unidirectional Command-Query Responsibility Segregation (CQRS) state machine connecting UI events, background tasks, and the real-time audio thread.

---

## 1. Storage & Indexing Architecture (`amp-adapter-storage`)

To eliminate any native C-compiler requirement (avoiding SQLite, LMDB, or RocksDB), the local library and persistent caches use **`redb`**, a 100% safe, pure-Rust embedded ACID key-value database.

```
       ┌────────────────────────────────────────────────────────┐
       │                Folder Crawler & Watcher                │
       │           (Async walkdir + notify filesystem)          │
       └──────────────────────────┬─────────────────────────────┘
                                  │
                                  ▼
       ┌────────────────────────────────────────────────────────┐
       │         Lofty Pure-Rust Metadata Tag Extractor         │
       │       (Extracts Title, Artist, Album, Bitrate, Art)    │
       └──────────────────────────┬─────────────────────────────┘
                                  │
                                  ▼
       ┌────────────────────────────────────────────────────────┐
       │                 redb Embedded Database                 │
       │  Tables:                                               │
       │  • TRACKS_BY_ID:      [UUID -> Bincode(Track)]         │
       │  • SEARCH_INDEX:      [String (prefix) -> Vec<UUID>]   │
       │  • PLAYLISTS:         [UUID -> Bincode(Playlist)]      │
       │  • CACHED_STREAM_URLS:[String (URL) -> CacheDescriptor]│
       └────────────────────────────────────────────────────────┘
```

### 1.1. Schema & `redb` Database Implementation

```rust
// crates/amp-adapter-storage/src/lib.rs
use amp_domain::{Track, TrackId};
use amp_ports::driven::LibraryStoragePort;
use async_trait::async_trait;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::Path;
use std::sync::Arc;

const TRACKS_TABLE: TableDefinition<&[u8; 16], &[u8]> = TableDefinition::new("tracks_v1");
const INDEX_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("search_index_v1");

pub struct RedbStorageAdapter {
    db: Arc<Database>,
}

impl RedbStorageAdapter {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let db = Database::create(path)?;
        
        // Initialize tables within a write transaction
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(TRACKS_TABLE)?;
            let _ = write_txn.open_table(INDEX_TABLE)?;
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
                .map_err(|e| format!("Serialization error: {e}"))?;
            
            let write_txn = db.begin_write()
                .map_err(|e| format!("DB Write transaction failure: {e}"))?;
            {
                let mut table = write_txn.open_table(TRACKS_TABLE)
                    .map_err(|e| e.to_string())?;
                table.insert(track_clone.id.0.as_bytes(), encoded_bytes.as_slice())
                    .map_err(|e| e.to_string())?;
            }
            write_txn.commit().map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e| format!("Task join panic: {e}"))?
    }

    async fn fetch_all(&self) -> Result<Vec<Track>, String> {
        let db = Arc::clone(&self.db);

        tokio::task::spawn_blocking(move || -> Result<Vec<Track>, String> {
            let read_txn = db.begin_read().map_err(|e| e.to_string())?;
            let table = read_txn.open_table(TRACKS_TABLE).map_err(|e| e.to_string())?;
            
            let mut tracks = Vec::new();
            for item in table.iter().map_err(|e| e.to_string())? {
                let (_, value) = item.map_err(|e| e.to_string())?;
                let track: Track = bincode::deserialize(value.value())
                    .map_err(|e| format!("Deserialization error: {e}"))?;
                tracks.push(track);
            }
            Ok(tracks)
        })
        .await
        .map_err(|e| format!("Task join panic: {e}"))?
    }

    async fn remove_track(&self, id: &TrackId) -> Result<(), String> {
        let db = Arc::clone(&self.db);
        let id_bytes = *id.0.as_bytes();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let write_txn = db.begin_write().map_err(|e| e.to_string())?;
            {
                let mut table = write_txn.open_table(TRACKS_TABLE).map_err(|e| e.to_string())?;
                table.remove(&id_bytes).map_err(|e| e.to_string())?;
            }
            write_txn.commit().map_err(|e| e.to_string())?;
            Ok(())
        })
        .await
        .map_err(|e| format!("Task join panic: {e}"))?
    }

    async fn search_cached(&self, text_query: &str) -> Result<Vec<Track>, String> {
        let tracks = self.fetch_all().await?;
        let q = text_query.to_lowercase();
        Ok(tracks
            .into_iter()
            .filter(|t| {
                t.metadata.title.to_lowercase().contains(&q)
                    || t.metadata.artist.to_lowercase().contains(&q)
                    || t.metadata.album.as_deref().unwrap_or("").to_lowercase().contains(&q)
            })
            .collect())
    }
}
```

---

## 2. Command-Query Event Bus (CQRS State Orchestrator)

To prevent UI locking, race conditions, or audio-thread starvation, communication between the driving UI and background services occurs via an asynchronous unidirectional command-event queue.

```
                  ┌─────────────────────────────────────────┐
                  │          UI Shell (Ratatui/egui)        │
                  └────────────┬────────────────────────────┘
                               │
                      Commands │ (MPSC Unbounded)
                               ▼
                  ┌─────────────────────────────────────────┐
                  │       Engine Event Orchestrator         │
                  │        (tokio central run loop)         │
                  └────────────┬────────────────────────────┘
                               │
                       Events  │ (broadcast channel: N-subscribers)
                               ▼
        ┌──────────────────────┼──────────────────────┐
        ▼                      ▼                      ▼
  [ UI Renderer ]     [ Playback Session ]     [ Downloader Queue ]
```

### 2.1. System Commands & Events Definition

```rust
// crates/amp-core-engine/src/events.rs
use amp_domain::{PlaybackState, Track, TrackId};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum PlayerCommand {
    PlayTrack(Box<Track>),
    TogglePause,
    Stop,
    SeekRelative(Duration, bool), // (Offset, Forward)
    SeekAbsolute(Duration),
    SetVolume(f32),              // 0.0 to 1.0
    AddToQueue(Box<Track>),
    RemoveFromQueue(TrackId),
    ClearQueue,
    TriggerDownload(Box<Track>, String), // (Track, Destination directory)
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    TrackStarted(Box<Track>),
    PlaybackStateChanged(PlaybackState),
    PositionUpdated { elapsed: Duration, total: Option<Duration> },
    SpectrumDataReady(Vec<f32>), // 16-band normalized magnitudes (0.0 .. 1.0)
    BufferHealth { percent_filled: u8, kbps: u32 },
    DownloadProgress { track_id: TrackId, percent: u8, completed: bool },
    ErrorNotification(String),
}
```

### 2.2. Command Processing Dispatcher

```rust
// crates/amp-core-engine/src/dispatcher.rs
use super::events::{PlayerCommand, PlayerEvent};
use tokio::sync::{broadcast, mpsc};

pub struct EventBus {
    pub cmd_sender: mpsc::UnboundedSender<PlayerCommand>,
    pub event_broadcaster: broadcast::Sender<PlayerEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> (Self, mpsc::UnboundedReceiver<PlayerCommand>) {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (evt_tx, _) = broadcast::channel(capacity);

        (
            Self {
                cmd_sender: cmd_tx,
                event_broadcaster: evt_tx,
            },
            cmd_rx,
        )
    }

    pub fn subscribe(&self) -> broadcast::Receiver<PlayerEvent> {
        self.event_broadcaster.subscribe()
    }

    pub fn send_command(&self, cmd: PlayerCommand) {
        let _ = self.cmd_sender.send(cmd);
    }

    pub fn emit_event(&self, event: PlayerEvent) {
        let _ = self.event_broadcaster.send(event);
    }
}
```