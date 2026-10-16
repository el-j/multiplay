//! JSON WebSocket protocol definitions for remote control and mobile clients.

#![forbid(unsafe_code)]

use amp_domain::Track;
use serde::{Deserialize, Serialize};

/// Inbound control action received from a connected remote client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum RemoteAction {
    /// Request the current playback and playlist status.
    GetPlaybackStatus,
    /// Resume or begin playback.
    Play,
    /// Pause ongoing playback.
    Pause,
    /// Skip to the next track in the queue.
    Next,
    /// Return to the previous track.
    Previous,
    /// Adjust player output volume (0.0 to 1.0).
    SetVolume(f32),
    /// Seek to a specified position in seconds.
    Seek(u64),
    /// Enqueue a track to the shared collaborative playlist.
    EnqueueTrack(Box<Track>),
    /// Cast a vote to skip the current track.
    VoteSkip,
}

/// Outbound broadcast event pushed from the player engine to remote clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum RemoteEventBroadcast {
    /// Comprehensive snapshot of the player's active state.
    Status {
        current_track: Option<Box<Track>>,
        is_playing: bool,
        volume: f32,
        position_secs: u64,
        active_peers: usize,
    },
    /// Acknowledgment of an applied action.
    ActionAck { action: String },
    /// Operational error notification.
    Error { message: String },
}
