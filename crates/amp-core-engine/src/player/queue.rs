//! Engine-owned playback queue, repeat/shuffle state, and Next/Previous navigation (`TASK-131`).
//!
//! Keeps playlist ordering, `SmartQueueManager`, and `EnqueueExisting` lookup inside
//! `amp-core-engine` so GUI, TUI, and remote daemon share one source of truth.

#![forbid(unsafe_code)]

use crate::playback::{RepeatMode, ShuffleMode, SmartQueueManager};
use amp_domain::{Track, TrackId};
use amp_ports::PlaylistError;
use std::collections::HashMap;
use std::time::Duration;

/// Threshold above which `PlayerCommand::Previous` restarts the current track instead of
/// stepping back to the previous track.
pub const PREVIOUS_RESTART_THRESHOLD: Duration = Duration::from_secs(3);

/// Outcome of a `Previous` navigation command.
#[derive(Debug, Clone, PartialEq)]
pub enum PreviousAction {
    /// Restart the currently playing track from `0:00`.
    RestartCurrent(Track),
    /// Load and play the previous track in the queue/history.
    PlayTrack(Track),
    /// Queue is empty and nothing is playing.
    None,
}

/// Engine-owned playback queue with repeat, shuffle, and host-catalog lookup.
pub struct EngineQueue {
    tracks: Vec<Track>,
    known_tracks: HashMap<TrackId, Track>,
    active_index: Option<usize>,
    repeat_mode: RepeatMode,
    shuffle_mode: ShuffleMode,
    smart_queue: SmartQueueManager,
}

impl Default for EngineQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineQueue {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            known_tracks: HashMap::new(),
            active_index: None,
            repeat_mode: RepeatMode::Off,
            shuffle_mode: ShuffleMode::Off,
            smart_queue: SmartQueueManager::new(),
        }
    }

    /// Registers a track in the host catalog so untrusted remotes can enqueue it by `TrackId`
    /// via `PlayerCommand::EnqueueExisting(TrackId)`.
    pub fn register_known_track(&mut self, track: Track) {
        self.known_tracks.insert(track.id, track);
    }

    /// Bulk-registers tracks in the host catalog.
    pub fn register_known_tracks(&mut self, tracks: impl IntoIterator<Item = Track>) {
        for track in tracks {
            self.register_known_track(track);
        }
    }

    /// Replaces the entire queue while preserving the catalog of known tracks.
    pub fn set_tracks(&mut self, tracks: Vec<Track>) {
        for t in &tracks {
            self.known_tracks.insert(t.id, t.clone());
        }
        self.tracks = tracks.clone();
        self.active_index = if self.tracks.is_empty() {
            None
        } else {
            Some(0)
        };
        self.smart_queue.set_tracks(tracks);
    }

    /// Appends a track to the end of the queue and registers it as a known host track.
    pub fn add_track(&mut self, track: Track) -> usize {
        self.known_tracks.insert(track.id, track.clone());
        self.tracks.push(track);
        if self.active_index.is_none() {
            self.active_index = Some(0);
        }
        self.smart_queue.set_tracks(self.tracks.clone());
        self.tracks.len() - 1
    }

    /// Reorders a track in the active queue.
    pub fn move_track(&mut self, from_idx: usize, to_idx: usize) -> Result<(), PlaylistError> {
        let len = self.tracks.len();
        if from_idx >= len {
            return Err(PlaylistError::IndexOutOfBounds {
                index: from_idx,
                len,
            });
        }
        if to_idx >= len {
            return Err(PlaylistError::IndexOutOfBounds { index: to_idx, len });
        }
        let item = self.tracks.remove(from_idx);
        self.tracks.insert(to_idx, item);
        self.smart_queue.set_tracks(self.tracks.clone());
        Ok(())
    }

    /// Enqueues an already-known host track by `TrackId`.
    pub fn enqueue_existing(&mut self, id: TrackId) -> Result<Track, PlaylistError> {
        let track = self
            .known_tracks
            .get(&id)
            .cloned()
            .or_else(|| self.tracks.iter().find(|t| t.id == id).cloned())
            .ok_or(PlaylistError::TrackNotFound(id))?;
        self.add_track(track.clone());
        Ok(track)
    }

    /// Removes a track by `TrackId` from the active queue.
    pub fn remove_track(&mut self, id: TrackId) -> Result<(), PlaylistError> {
        let pos = self
            .tracks
            .iter()
            .position(|t| t.id == id)
            .ok_or(PlaylistError::TrackNotFound(id))?;
        self.tracks.remove(pos);
        if self.tracks.is_empty() {
            self.active_index = None;
        } else if let Some(active) = self.active_index {
            if pos < active {
                self.active_index = Some(active - 1);
            } else if active >= self.tracks.len() {
                self.active_index = Some(self.tracks.len() - 1);
            }
        }
        self.smart_queue.set_tracks(self.tracks.clone());
        Ok(())
    }

    /// Clears all tracks from the active queue (known catalog remains registered).
    pub fn clear(&mut self) {
        self.tracks.clear();
        self.active_index = None;
        self.smart_queue.set_tracks(Vec::new());
    }

    /// Snapshot of the current queue tracks.
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// Marks `track` as the currently active track in the queue (inserting it if absent).
    pub fn select_track(&mut self, track: &Track) {
        self.known_tracks.insert(track.id, track.clone());
        if let Some(pos) = self
            .tracks
            .iter()
            .position(|t| t.id == track.id || t.source == track.source)
        {
            self.active_index = Some(pos);
        } else {
            self.tracks.push(track.clone());
            self.active_index = Some(self.tracks.len() - 1);
            self.smart_queue.set_tracks(self.tracks.clone());
        }
    }

    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat_mode = mode;
        self.smart_queue.set_repeat(mode);
    }

    pub fn repeat_mode(&self) -> RepeatMode {
        self.repeat_mode
    }

    pub fn set_shuffle(&mut self, mode: ShuffleMode) {
        self.shuffle_mode = mode;
        self.smart_queue.set_shuffle(mode);
    }

    pub fn shuffle_mode(&self) -> ShuffleMode {
        self.shuffle_mode
    }

    /// Advances to the next track according to repeat and shuffle modes.
    /// When `manual_skip` is false (natural track end), `RepeatOne` replays the active track.
    pub fn next_track(&mut self, manual_skip: bool) -> Option<Track> {
        if self.tracks.is_empty() {
            return None;
        }

        if !manual_skip && self.repeat_mode == RepeatMode::RepeatOne {
            if let Some(idx) = self.active_index {
                return self.tracks.get(idx).cloned();
            }
        }

        match self.shuffle_mode {
            ShuffleMode::Off => {
                let next_idx = match self.active_index {
                    Some(idx) if idx + 1 < self.tracks.len() => Some(idx + 1),
                    Some(_) => match self.repeat_mode {
                        RepeatMode::RepeatAll | RepeatMode::RepeatOne => Some(0),
                        RepeatMode::Off => None,
                    },
                    None => Some(0),
                }?;
                self.active_index = Some(next_idx);
                self.tracks.get(next_idx).cloned()
            }
            ShuffleMode::TrueRandom | ShuffleMode::SmartBalanced { .. } => {
                let next = self.smart_queue.next_track()?;
                if let Some(pos) = self.tracks.iter().position(|t| t.id == next.id) {
                    self.active_index = Some(pos);
                }
                Some(next)
            }
        }
    }

    /// Peeks the upcoming track for gapless/crossfade preloading without advancing state.
    pub fn peek_upcoming(&self) -> Option<Track> {
        if self.tracks.is_empty() {
            return None;
        }
        if self.repeat_mode == RepeatMode::RepeatOne {
            return self.active_index.and_then(|i| self.tracks.get(i).cloned());
        }
        match self.shuffle_mode {
            ShuffleMode::Off => {
                let next_idx = match self.active_index {
                    Some(idx) if idx + 1 < self.tracks.len() => Some(idx + 1),
                    Some(_) if self.repeat_mode == RepeatMode::RepeatAll => Some(0),
                    None => Some(0),
                    _ => None,
                }?;
                self.tracks.get(next_idx).cloned()
            }
            ShuffleMode::TrueRandom | ShuffleMode::SmartBalanced { .. } => {
                let idx = self.active_index.unwrap_or(0);
                let next_idx = (idx + 1) % self.tracks.len();
                self.tracks.get(next_idx).cloned()
            }
        }
    }

    /// Computes the outcome of `PlayerCommand::Previous`.
    /// Restarts the active track if `current_elapsed > 3 s`, otherwise moves to the previous track.
    pub fn previous_track(
        &mut self,
        current_track: Option<&Track>,
        current_elapsed: Duration,
    ) -> PreviousAction {
        if current_elapsed > PREVIOUS_RESTART_THRESHOLD {
            if let Some(cur) = current_track {
                return PreviousAction::RestartCurrent(cur.clone());
            }
        }

        if self.tracks.is_empty() {
            return current_track
                .cloned()
                .map_or(PreviousAction::None, PreviousAction::RestartCurrent);
        }

        match self.shuffle_mode {
            ShuffleMode::Off => {
                if let Some(idx) = self.active_index {
                    if idx > 0 {
                        self.active_index = Some(idx - 1);
                        return PreviousAction::PlayTrack(self.tracks[idx - 1].clone());
                    } else if self.repeat_mode == RepeatMode::RepeatAll && self.tracks.len() > 1 {
                        let last = self.tracks.len() - 1;
                        self.active_index = Some(last);
                        return PreviousAction::PlayTrack(self.tracks[last].clone());
                    }
                }
                if let Some(cur) = current_track {
                    PreviousAction::RestartCurrent(cur.clone())
                } else {
                    self.active_index = Some(0);
                    PreviousAction::PlayTrack(self.tracks[0].clone())
                }
            }
            ShuffleMode::TrueRandom | ShuffleMode::SmartBalanced { .. } => {
                if let Some(prev) = self.smart_queue.previous_track() {
                    if let Some(pos) = self.tracks.iter().position(|t| t.id == prev.id) {
                        self.active_index = Some(pos);
                    }
                    PreviousAction::PlayTrack(prev)
                } else if let Some(cur) = current_track {
                    PreviousAction::RestartCurrent(cur.clone())
                } else {
                    PreviousAction::None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::{AudioSource, TrackMetadata};

    fn sample_track(title: &str) -> Track {
        Track::new(
            AudioSource::LocalFile {
                path: format!("/music/{title}.wav"),
            },
            TrackMetadata::new(title, "Artist"),
        )
    }

    #[test]
    fn queue_next_and_previous_respect_3s_restart_and_repeat_modes() {
        let mut q = EngineQueue::new();
        let t1 = sample_track("One");
        let t2 = sample_track("Two");
        q.add_track(t1.clone());
        q.add_track(t2.clone());
        q.select_track(&t1);

        // Previous when elapsed > 3s restarts current track
        assert_eq!(
            q.previous_track(Some(&t1), Duration::from_secs(5)),
            PreviousAction::RestartCurrent(t1.clone())
        );

        // Next advances to t2
        assert_eq!(q.next_track(true).unwrap().id, t2.id);

        // Previous when elapsed <= 3s steps back to t1
        assert_eq!(
            q.previous_track(Some(&t2), Duration::from_secs(1)),
            PreviousAction::PlayTrack(t1.clone())
        );

        // Advance to t2 again; at end with RepeatOff -> None
        assert_eq!(q.next_track(true).unwrap().id, t2.id);
        assert!(q.next_track(false).is_none());

        // RepeatAll wraps around to t1
        q.set_repeat(RepeatMode::RepeatAll);
        assert_eq!(q.next_track(false).unwrap().id, t1.id);

        // RepeatOne replays active track on natural track end
        q.set_repeat(RepeatMode::RepeatOne);
        assert_eq!(q.next_track(false).unwrap().id, t1.id);
    }

    #[test]
    fn queue_enqueue_existing_resolves_known_track_by_id() {
        let mut q = EngineQueue::new();
        let host_track = sample_track("CatalogSong");
        let id = host_track.id;
        q.register_known_track(host_track.clone());
        assert_eq!(q.tracks().len(), 0);

        let enqueued = q.enqueue_existing(id).expect("must resolve known track");
        assert_eq!(enqueued.id, id);
        assert_eq!(q.tracks().len(), 1);
    }
}
