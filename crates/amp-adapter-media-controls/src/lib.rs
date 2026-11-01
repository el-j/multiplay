//! OS media controls adapter (TASK-173): hardware media keys, headset buttons and the "Now
//! Playing" panels, through `souvlaki` (MPRIS over pure-Rust zbus on Linux, the MediaPlayer
//! framework on macOS, SMTC on Windows).
//!
//! macOS needs an AppKit run loop (the desktop app has one) and Windows needs the window
//! handle, so the TUI only gets media controls on Linux.

#![forbid(unsafe_code)]

use amp_ports::{MediaControlsPort, MediaKeyEvent, MediaPlayback, NowPlaying};
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPosition, PlatformConfig, SeekDirection,
};
use std::ffi::c_void;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// How far the reported position may drift from the clock before the OS is told again.
const SEEK_TOLERANCE: Duration = Duration::from_millis(1500);

/// The player request for an OS event; `None` for events the player leaves to others (volume,
/// quit, raise, open URI).
#[must_use]
pub fn map_event(event: MediaControlEvent) -> Option<MediaKeyEvent> {
    let secs = |d: Duration| i64::try_from(d.as_secs()).unwrap_or(i64::MAX);
    Some(match event {
        MediaControlEvent::Play => MediaKeyEvent::Play,
        MediaControlEvent::Pause => MediaKeyEvent::Pause,
        MediaControlEvent::Toggle => MediaKeyEvent::Toggle,
        MediaControlEvent::Stop => MediaKeyEvent::Stop,
        MediaControlEvent::Next => MediaKeyEvent::Next,
        MediaControlEvent::Previous => MediaKeyEvent::Previous,
        MediaControlEvent::Seek(dir) => MediaKeyEvent::SeekStep {
            forward: dir == SeekDirection::Forward,
        },
        MediaControlEvent::SeekBy(SeekDirection::Forward, d) => MediaKeyEvent::SeekBy(secs(d)),
        MediaControlEvent::SeekBy(SeekDirection::Backward, d) => MediaKeyEvent::SeekBy(-secs(d)),
        MediaControlEvent::SetPosition(MediaPosition(at)) => MediaKeyEvent::SetPosition(at),
        _ => return None,
    })
}

/// Decides when the OS needs a playback update: on every change of state, and when the position
/// jumps (a seek). Between those the OS extrapolates the position from the last update.
#[derive(Debug, Default)]
pub struct PlaybackSync {
    last: Option<(MediaPlayback, Instant)>,
}

impl PlaybackSync {
    /// True when `now` should be sent; remembers it if so.
    pub fn should_send(&mut self, now: MediaPlayback, at: Instant) -> bool {
        let send = match (self.last, now) {
            (None, _) => true,
            (
                Some((MediaPlayback::Playing { position: was }, then)),
                MediaPlayback::Playing { position },
            ) => {
                let expected = was + at.saturating_duration_since(then);
                position.abs_diff(expected) > SEEK_TOLERANCE
            }
            (Some((last, _)), now) => last != now,
        };
        if send {
            self.last = Some((now, at));
        }
        send
    }
}

fn to_souvlaki(playback: MediaPlayback) -> souvlaki::MediaPlayback {
    let progress = |p| Some(MediaPosition(p));
    match playback {
        MediaPlayback::Stopped => souvlaki::MediaPlayback::Stopped,
        MediaPlayback::Paused { position } => souvlaki::MediaPlayback::Paused {
            progress: progress(position),
        },
        MediaPlayback::Playing { position } => souvlaki::MediaPlayback::Playing {
            progress: progress(position),
        },
    }
}

/// The system's media session for multiplay.
pub struct OsMediaControls {
    controls: MediaControls,
    events: Receiver<MediaKeyEvent>,
    now_playing: Option<NowPlaying>,
    playback: PlaybackSync,
}

impl OsMediaControls {
    /// Registers multiplay with the system. `hwnd` is the main window's handle on Windows
    /// (required there) and ignored elsewhere.
    ///
    /// # Errors
    /// When the system has no media session service (no D-Bus session on Linux) or refuses.
    /// `wake` runs on the OS's thread after each event so the app can handle it right away.
    pub fn new(hwnd: Option<*mut c_void>, wake: Box<dyn Fn() + Send>) -> Result<Self, String> {
        let config = PlatformConfig {
            display_name: "multiplay",
            dbus_name: "multiplay",
            hwnd,
        };
        let mut controls = MediaControls::new(config).map_err(|e| format!("{e:?}"))?;
        let (tx, rx) = mpsc::channel();
        controls
            .attach(move |event| {
                if let Some(e) = map_event(event) {
                    let _ = tx.send(e);
                    wake();
                }
            })
            .map_err(|e| format!("{e:?}"))?;
        Ok(Self {
            controls,
            events: rx,
            now_playing: None,
            playback: PlaybackSync::default(),
        })
    }
}

impl MediaControlsPort for OsMediaControls {
    fn set_now_playing(&mut self, now: &NowPlaying) {
        if self.now_playing.as_ref() == Some(now) {
            return;
        }
        let metadata = MediaMetadata {
            title: Some(&now.title),
            artist: Some(&now.artist),
            album: now.album.as_deref(),
            duration: now.duration,
            cover_url: now.cover_url.as_deref(),
        };
        if let Err(e) = self.controls.set_metadata(metadata) {
            tracing::debug!("media controls: metadata not accepted: {e:?}");
        }
        self.now_playing = Some(now.clone());
    }

    fn set_playback(&mut self, playback: MediaPlayback) {
        if self.playback.should_send(playback, Instant::now()) {
            if let Err(e) = self.controls.set_playback(to_souvlaki(playback)) {
                tracing::debug!("media controls: playback not accepted: {e:?}");
            }
        }
    }

    fn take_events(&mut self) -> Vec<MediaKeyEvent> {
        self.events.try_iter().collect()
    }
}
