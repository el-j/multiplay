//! CQRS command and event definitions for player state synchronization.

#![forbid(unsafe_code)]

use amp_domain::{PlaybackState, Track, TrackId};
use std::time::Duration;

/// Inbound commands dispatched from UI controllers or external interfaces to the core engine.
#[derive(Debug, Clone)]
pub enum PlayerCommand {
    PlayTrack(Box<Track>),
    TogglePause,
    Stop,
    SeekRelative(Duration, bool), // (Offset duration, forward)
    SeekAbsolute(Duration),
    SetVolume(f32), // Normalized volume (0.0 to 1.0)
    AddToQueue(Box<Track>),
    /// Skip to the next track in the queue.
    Next,
    /// Return to the previous track (or restart the current one).
    Previous,
    /// Enqueue a track the host already holds, by id. Used by untrusted remotes,
    /// which may never supply a path or URL themselves.
    EnqueueExisting(TrackId),
    RemoveFromQueue(TrackId),
    ClearQueue,
    TriggerDownload(Box<Track>, String), // (Track, Destination folder)
}

/// Outbound broadcast events emitted by the core engine to all active UI and background subscribers.
#[derive(Debug, Clone)]
pub enum PlayerEvent {
    TrackStarted(Box<Track>),
    PlaybackStateChanged(PlaybackState),
    PositionUpdated {
        elapsed: Duration,
        total: Option<Duration>,
    },
    SpectrumDataReady(Vec<f32>), // 16-band normalized magnitudes (0.0 to 1.0)
    BufferHealth {
        percent_filled: u8,
        kbps: u32,
    },
    DownloadProgress {
        track_id: TrackId,
        percent: u8,
        completed: bool,
    },
    ConfigChanged(Box<amp_domain::AppConfig>),
    ErrorNotification(String),
}
