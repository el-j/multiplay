//! The desktop app in the system's media controls (TASK-173): Now Playing shows the engine's
//! track and state; media keys and panel buttons go through the same transport actions as the
//! buttons and the keyboard.

#![forbid(unsafe_code)]

use crate::app::MultiplayGuiApp;
use crate::transport::TransportAction;
use amp_domain::PlaybackState;
use amp_ports::{MediaControlsPort, MediaKeyEvent, MediaPlayback, NowPlaying};

/// Seconds a bare "seek" key (no amount given) moves.
const SEEK_STEP: i32 = 5;

impl MultiplayGuiApp {
    /// Connects the system's media session (or a stand-in in tests).
    pub fn attach_media_controls(&mut self, controls: Box<dyn MediaControlsPort>) {
        self.media_controls = Some(controls);
    }

    /// Tells the system what plays and runs the media keys pressed since the last frame.
    pub fn sync_media_controls(&mut self) {
        let Some(mut controls) = self.media_controls.take() else {
            return;
        };
        let track = self
            .audio_driver
            .current_track
            .lock()
            .ok()
            .and_then(|t| t.clone());
        if let Some(track) = &track {
            let meta = &track.metadata;
            controls.set_now_playing(&NowPlaying {
                title: meta.title.clone(),
                artist: meta.artist.clone(),
                album: meta.album.clone(),
                duration: *self.audio_driver.total_duration.lock().unwrap(),
                cover_url: meta
                    .cover_art_uri
                    .clone()
                    .filter(|u| u.starts_with("http://") || u.starts_with("https://")),
            });
        }
        let position = *self.audio_driver.elapsed.lock().unwrap();
        controls.set_playback(match self.audio_driver.playback_state() {
            PlaybackState::Playing | PlaybackState::Buffering { .. } => {
                MediaPlayback::Playing { position }
            }
            PlaybackState::Paused if track.is_some() => MediaPlayback::Paused { position },
            _ => MediaPlayback::Stopped,
        });
        let events = controls.take_events();
        self.media_controls = Some(controls);

        let playing = self.audio_driver.playback_state() == PlaybackState::Playing;
        for event in events {
            let action = match event {
                MediaKeyEvent::Play if playing => continue,
                MediaKeyEvent::Pause if !playing => continue,
                MediaKeyEvent::Play | MediaKeyEvent::Pause | MediaKeyEvent::Toggle => {
                    TransportAction::PlayPause
                }
                MediaKeyEvent::Stop => TransportAction::Stop,
                MediaKeyEvent::Next => TransportAction::Next,
                MediaKeyEvent::Previous => TransportAction::Previous,
                MediaKeyEvent::SeekStep { forward } => {
                    TransportAction::SeekBy(if forward { SEEK_STEP } else { -SEEK_STEP })
                }
                MediaKeyEvent::SeekBy(secs) => {
                    TransportAction::SeekBy(i32::try_from(secs).unwrap_or(if secs < 0 {
                        i32::MIN
                    } else {
                        i32::MAX
                    }))
                }
                MediaKeyEvent::SetPosition(at) => {
                    self.audio_driver.seek(at);
                    continue;
                }
            };
            self.apply_transport(action);
        }
    }
}
