//! Hexagonal driving port implementations (`PlayerControlPort`, `PlaylistManagementPort`)
//! and CQRS `PlayerCommand` dispatch for [`PlayerEngine`].

#![forbid(unsafe_code)]

use super::PlayerEngine;
use crate::events::{PlayerCommand, PlayerErrorKind, PlayerEvent};
use amp_domain::{PlaybackState, Track, TrackId};
use amp_ports::{
    async_trait, PlayerControlError, PlayerControlPort, PlaylistError, PlaylistManagementPort,
};
use std::time::Duration;

impl PlayerEngine {
    /// Executes a [`PlayerCommand`] directly against the engine.
    pub fn dispatch_command(&self, cmd: PlayerCommand) {
        match cmd {
            PlayerCommand::PlayTrack(t) => self.play(*t),
            PlayerCommand::TogglePause => self.toggle_play_pause(),
            PlayerCommand::Stop => self.stop(),
            PlayerCommand::SeekRelative(offset, forward) => self.seek_relative(offset, forward),
            PlayerCommand::SeekAbsolute(pos) => self.seek(pos),
            PlayerCommand::SetVolume(v) => self.set_volume(v),
            PlayerCommand::AddToQueue(t) => {
                let _ = self.queue.lock().unwrap().add_track(*t);
            }
            PlayerCommand::Next => {
                let _ = self.next_in_queue();
            }
            PlayerCommand::Previous => {
                let _ = self.previous_in_queue();
            }
            PlayerCommand::EnqueueExisting(id) => {
                if let Err(e) = self.queue.lock().unwrap().enqueue_existing(id) {
                    self.emit(PlayerEvent::Error {
                        kind: PlayerErrorKind::Playlist,
                        source: None,
                        message: e.to_string(),
                        recoverable: true,
                    });
                }
            }
            PlayerCommand::RemoveFromQueue(id) => {
                let _ = self.queue.lock().unwrap().remove_track(id);
            }
            PlayerCommand::ClearQueue => self.queue.lock().unwrap().clear(),
            PlayerCommand::TriggerDownload(track, dest) => {
                let resolvers = self.resolvers.lock().unwrap().clone();
                let bus = self.event_bus.clone();
                let dest_path = std::path::PathBuf::from(dest);
                self.runtime_handle.spawn(async move {
                    if let Some(resolver) =
                        resolvers.iter().find(|r| r.handles_source(&track.source))
                    {
                        let event_tx = bus.as_ref().map(|b| &b.event_broadcaster);
                        let _ = crate::downloader::OfflineTrackDownloader::download_and_tag(
                            resolver.as_ref(),
                            &track,
                            &dest_path,
                            None,
                            event_tx,
                        )
                        .await;
                    }
                });
            }
        }
    }
}

#[async_trait]
impl PlayerControlPort for PlayerEngine {
    async fn play_track(&self, track: Track) -> Result<(), PlayerControlError> {
        self.play(track);
        Ok(())
    }

    async fn toggle_pause(&self) -> Result<(), PlayerControlError> {
        self.toggle_play_pause();
        Ok(())
    }

    async fn stop(&self) -> Result<(), PlayerControlError> {
        PlayerEngine::stop(self);
        Ok(())
    }

    async fn seek_relative(&self, off: Duration, fwd: bool) -> Result<(), PlayerControlError> {
        PlayerEngine::seek_relative(self, off, fwd);
        Ok(())
    }

    async fn seek_absolute(&self, timestamp: Duration) -> Result<(), PlayerControlError> {
        self.seek(timestamp);
        Ok(())
    }

    fn current_state(&self) -> PlaybackState {
        self.playback_state()
    }

    fn current_position(&self) -> Duration {
        *self.elapsed.lock().unwrap()
    }
}

#[async_trait]
impl PlaylistManagementPort for PlayerEngine {
    async fn add_to_playlist(&self, track: Track) -> Result<usize, PlaylistError> {
        Ok(self.queue.lock().unwrap().add_track(track))
    }

    async fn remove_from_playlist(&self, track_id: TrackId) -> Result<(), PlaylistError> {
        self.queue.lock().unwrap().remove_track(track_id)
    }

    async fn clear_playlist(&self) -> Result<(), PlaylistError> {
        self.queue.lock().unwrap().clear();
        Ok(())
    }

    async fn move_track(&self, from_idx: usize, to_idx: usize) -> Result<(), PlaylistError> {
        self.queue.lock().unwrap().move_track(from_idx, to_idx)
    }

    async fn get_playlist(&self) -> Vec<Track> {
        self.queue.lock().unwrap().tracks().to_vec()
    }
}
