//! Driven port for the operating system's media controls (TASK-173): hardware media keys,
//! headset buttons, and the "Now Playing" panels (macOS Control Center, Windows' volume
//! overlay, Linux MPRIS clients such as `playerctl` and desktop widgets).

#![forbid(unsafe_code)]

use std::time::Duration;

/// What the system shows as playing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Option<Duration>,
    /// An `http(s)` or `file://` image URL.
    pub cover_url: Option<String>,
}

/// Playback state as the system shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaPlayback {
    Stopped,
    Paused { position: Duration },
    Playing { position: Duration },
}

/// A request from a media key or a Now Playing panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKeyEvent {
    Play,
    Pause,
    Toggle,
    Stop,
    Next,
    Previous,
    /// Seek forward (`true`) or back by a step.
    SeekStep {
        forward: bool,
    },
    /// Seek by an exact offset (positive or negative), in seconds.
    SeekBy(i64),
    /// Jump to a position.
    SetPosition(Duration),
}

/// The system's media controls. Implementations must not block.
pub trait MediaControlsPort {
    /// Shows `now` as the playing track.
    fn set_now_playing(&mut self, now: &NowPlaying);
    /// Shows the playback state.
    fn set_playback(&mut self, playback: MediaPlayback);
    /// Requests that arrived since the last call.
    fn take_events(&mut self) -> Vec<MediaKeyEvent>;
}
