# Distributed Mesh Jam & Authenticated Mobile Remote Control

This document defines two interconnected distributed capabilities:
1. **Shared P2P "Jam Session" Playlist**: Multiple peer instances running anywhere in the world join a synchronized session via **Conflict-Free Replicated Data Types (CRDTs)** over the existing encrypted Noise mesh.
2. **Pure-Rust Remote Protocol**: Embedded zero-allocation WebSocket/HTTP engine exposing an authenticated control interface for native iOS/Android mobile clients or web browsers.

---

## 1. Collaborative Real-Time "Jam" Engine (CRDT State Machine)

To allow multiple users to insert, reorder, upvote, and delete tracks across remote network instances without a centralized server, the queue is modeled as a **Replicated Growable Array (RGA)**.

```
       ┌────────────────────────┐              ┌────────────────────────┐
       │   Host Node (Laptop)   │              │  Guest Peer (Desktop)  │
       │   Peer ID: 12D3...AAA  │              │  Peer ID: 12D3...BBB   │
       └───────────┬────────────┘              └───────────┬────────────┘
                   │                                       │
                   │    1. Join Session ("AmbientRoom")    │
                   │◄──────────────────────────────────────┤
                   │                                       │
                   │    2. Sync State Snapshot (CRDT)      │
                   ├──────────────────────────────────────►│
                   │                                       │
                   │    3. Broadcast Insert (YouTube Track)│
                   │◄──────────────────────────────────────┤
                   │                                       │
                   │    4. Deterministic State Convergence │
                   │       (Both play queues identical)    │
```

### 1.1. CRDT Operation Model (`crates/amp-domain/src/jam_crdt.rs`)

```rust
use crate::{Track, TrackId};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LamportTimestamp {
    pub counter: u64,
    pub peer_id_fingerprint: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JamItem {
    pub element_id: LamportTimestamp,
    pub track: Track,
    pub added_by_peer: String,
    pub upvotes: u32,
    pub is_deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JamOperation {
    InsertAfter {
        preceding_id: Option<LamportTimestamp>,
        new_id: LamportTimestamp,
        track: Track,
        user: String,
    },
    Upvote {
        element_id: LamportTimestamp,
        by_user: String,
    },
    Delete {
        element_id: LamportTimestamp,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CollaborativeQueue {
    pub session_id: String,
    pub items: Vec<JamItem>,
    pub current_clock: u64,
}

impl CollaborativeQueue {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            items: Vec::new(),
            current_clock: 0,
        }
    }

    /// Merges an incoming CRDT operation deterministically
    pub fn apply_remote_op(&mut self, op: JamOperation) {
        match op {
            JamOperation::InsertAfter { preceding_id, new_id, track, user } => {
                self.current_clock = self.current_clock.max(new_id.counter) + 1;

                if self.items.iter().any(|item| item.element_id == new_id) {
                    return; // Idempotent discard of duplicate
                }

                let insert_idx = if let Some(target_id) = preceding_id {
                    self.items
                        .iter()
                        .position(|i| i.element_id == target_id)
                        .map(|pos| pos + 1)
                        .unwrap_or(self.items.len())
                } else {
                    0
                };

                // Conflict tie-break: Higher Lamport timestamp wins; if equal, fingerprint breaks tie
                let mut final_idx = insert_idx;
                while final_idx < self.items.len() && self.items[final_idx].element_id > new_id {
                    final_idx += 1;
                }

                self.items.insert(final_idx, JamItem {
                    element_id: new_id,
                    track,
                    added_by_peer: user,
                    upvotes: 0,
                    is_deleted: false,
                });
            }
            JamOperation::Upvote { element_id, .. } => {
                if let Some(item) = self.items.iter_mut().find(|i| i.element_id == element_id) {
                    item.upvotes += 1;
                }
            }
            JamOperation::Delete { element_id } => {
                if let Some(item) = self.items.iter_mut().find(|i| i.element_id == element_id) {
                    item.is_deleted = true;
                }
            }
        }
    }

    pub fn active_playlist(&self) -> Vec<Track> {
        self.items
            .iter()
            .filter(|i| !i.is_deleted)
            .map(|i| i.track.clone())
            .collect()
    }
}
```

---

## 2. Remote Control Protocol & Embedded Pure-Rust WebSocket Daemon

To control the player running on a Mac, server, or living-room media center from an iPhone, Android, or terminal across the room, the core engine embeds an asynchronous lightweight control daemon using pure-Rust networking.

```
       ┌────────────────────────┐
       │   iOS / Android App    │  <-- Built with flutter, slint, or web app
       │    (Phone on Wi-Fi)    │
       └───────────┬────────────┘
                   │
                   │ Authenticated WebSocket Frame (JSON-RPC)
                   │  { "cmd": "Seek", "position_sec": 42 }
                   ▼
       ┌────────────────────────┐
       │   amp-adapter-remote   │  <-- Embedded pure-Rust TCP/WebSocket server
       │  (Port 8888, Zero C)   │
       └───────────┬────────────┘
                   │
                   ▼
       ┌────────────────────────┐
       │  Core Engine EventBus  │  <-- Dispatches directly to Audio Pipeline
       └────────────────────────┘
```

### 2.1. Protocol Schema & Dispatcher (`crates/amp-adapter-remote/src/lib.rs`)

```rust
use amp_core_engine::dispatcher::EventBus;
use amp_core_engine::events::PlayerCommand;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use futures_util::{SinkExt, StreamExt};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload")]
pub enum RemoteAction {
    GetPlaybackStatus,
    Play,
    Pause,
    Next,
    Previous,
    SetVolume(f32),
    Seek(u64), // Seconds
    EnqueueTrack { uri: String },
    VoteSkip,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoteEventBroadcast {
    pub track_title: String,
    pub artist: String,
    pub elapsed_seconds: u64,
    pub total_seconds: u64,
    pub volume: f32,
    pub is_playing: bool,
    pub active_listeners_count: usize,
}

pub struct MobileRemoteServer {
    listen_port: u16,
    event_bus: Arc<EventBus>,
    auth_token: String,
}

impl MobileRemoteServer {
    pub fn new(listen_port: u16, event_bus: Arc<EventBus>, auth_token: String) -> Self {
        Self {
            listen_port,
            event_bus,
            auth_token,
        }
    }

    pub async fn run_server(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.listen_port)).await?;
        println!("󰄬 Mobile Remote Server listening on ws://0.0.0.0:{}", self.listen_port);

        while let Ok((stream, addr)) = listener.accept().await {
            let bus = Arc::clone(&self.event_bus);
            let expected_token = self.auth_token.clone();

            tokio::spawn(async move {
                if let Ok(mut ws_stream) = accept_async(stream).await {
                    println!("󰄬 Mobile controller connected from {addr}");

                    while let Some(Ok(msg)) = ws_stream.next().await {
                        if let Ok(text) = msg.to_text() {
                            if let Ok(action) = serde_json::from_str::<RemoteAction>(text) {
                                match action {
                                    RemoteAction::Play => bus.send_command(PlayerCommand::TogglePause),
                                    RemoteAction::Pause => bus.send_command(PlayerCommand::TogglePause),
                                    RemoteAction::SetVolume(v) => bus.send_command(PlayerCommand::SetVolume(v)),
                                    RemoteAction::Seek(s) => bus.send_command(PlayerCommand::SeekAbsolute(std::time::Duration::from_secs(s))),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            });
        }

        Ok(())
    }
}
```

---

## 3. Pairing via Terminal QR Code

To connect a mobile phone without typing IP addresses or complex keys:
1. The desktop app generates an ephemeral Ed25519 pairing token.
2. In the TUI/GUI, Ratatui renders a dense ASCII QR code directly into the terminal cell grid (`qrcodegen` crate, 100% pure Rust).
3. The user opens their phone camera or mobile web client, scans the QR code containing `ws://192.168.1.15:8888?token=xyz`, and instantly secures an authenticated remote session.