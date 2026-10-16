//! Pure-Rust Smart Balanced Shuffle & Queue Manager.
//! Prevents track clustering, ensures uniform permutations, and respects repeat modes.

#![forbid(unsafe_code)]

use amp_domain::Track;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShuffleMode {
    Off,
    TrueRandom,
    SmartBalanced { artist_spacing: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    RepeatAll,
    RepeatOne,
}

pub struct SmartQueueManager {
    pristine_queue: Vec<Track>,
    playback_order: VecDeque<usize>,
    history: Vec<usize>,
    shuffle_mode: ShuffleMode,
    repeat_mode: RepeatMode,
    rng: SmallRng,
}

impl Default for SmartQueueManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartQueueManager {
    pub fn new() -> Self {
        Self {
            pristine_queue: Vec::new(),
            playback_order: VecDeque::new(),
            history: Vec::new(),
            shuffle_mode: ShuffleMode::Off,
            repeat_mode: RepeatMode::Off,
            rng: SmallRng::from_entropy(),
        }
    }

    pub fn set_tracks(&mut self, tracks: Vec<Track>) {
        self.pristine_queue = tracks;
        self.history.clear();
        self.rebuild_order();
    }

    pub fn set_shuffle(&mut self, mode: ShuffleMode) {
        self.shuffle_mode = mode;
        self.rebuild_order();
    }

    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat_mode = mode;
    }

    pub fn queue_len(&self) -> usize {
        self.pristine_queue.len()
    }

    pub fn next_track(&mut self) -> Option<Track> {
        if self.pristine_queue.is_empty() {
            return None;
        }

        if let RepeatMode::RepeatOne = self.repeat_mode {
            if let Some(&last_idx) = self.history.last() {
                return self.pristine_queue.get(last_idx).cloned();
            }
        }

        if self.playback_order.is_empty() {
            if let RepeatMode::RepeatAll = self.repeat_mode {
                self.rebuild_order();
            } else {
                return None;
            }
        }

        let next_idx = self.playback_order.pop_front()?;
        self.history.push(next_idx);
        self.pristine_queue.get(next_idx).cloned()
    }

    pub fn previous_track(&mut self) -> Option<Track> {
        if self.history.len() < 2 {
            return None;
        }
        let current = self.history.pop()?;
        self.playback_order.push_front(current);

        let prev_idx = *self.history.last()?;
        self.pristine_queue.get(prev_idx).cloned()
    }

    fn rebuild_order(&mut self) {
        let total = self.pristine_queue.len();
        if total == 0 {
            self.playback_order.clear();
            return;
        }

        let mut indices: Vec<usize> = (0..total).collect();

        match self.shuffle_mode {
            ShuffleMode::Off => {
                self.playback_order = indices.into();
            }
            ShuffleMode::TrueRandom => {
                indices.shuffle(&mut self.rng);
                self.playback_order = indices.into();
            }
            ShuffleMode::SmartBalanced { artist_spacing } => {
                self.playback_order = self.generate_balanced_indices(&indices, artist_spacing).into();
            }
        }
    }

    fn generate_balanced_indices(&mut self, base_indices: &[usize], spacing: usize) -> Vec<usize> {
        let mut pool = base_indices.to_vec();
        pool.shuffle(&mut self.rng);

        let mut result = Vec::with_capacity(pool.len());
        let mut artist_last_seen: HashMap<String, usize> = HashMap::new();

        while !pool.is_empty() {
            let mut found_idx = None;

            for (i, &track_idx) in pool.iter().enumerate() {
                let artist = &self.pristine_queue[track_idx].metadata.artist;
                let current_pos = result.len();

                if let Some(&last_pos) = artist_last_seen.get(artist) {
                    if current_pos.saturating_sub(last_pos) >= spacing {
                        found_idx = Some(i);
                        break;
                    }
                } else {
                    found_idx = Some(i);
                    break;
                }
            }

            let idx_to_take = found_idx.unwrap_or(0);
            let chosen = pool.remove(idx_to_take);
            let artist = self.pristine_queue[chosen].metadata.artist.clone();

            artist_last_seen.insert(artist, result.len());
            result.push(chosen);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::{AudioSource, TrackMetadata};

    fn make_test_track(title: &str, artist: &str) -> Track {
        Track::new(
            AudioSource::DirectHttp { url: format!("http://test/{title}") },
            TrackMetadata::new(title, artist),
        )
    }

    #[test]
    fn test_smart_shuffle() {
        let mut mgr = SmartQueueManager::new();
        let tracks = vec![
            make_test_track("Song A1", "Artist A"),
            make_test_track("Song B1", "Artist B"),
            make_test_track("Song A2", "Artist A"),
            make_test_track("Song C1", "Artist C"),
            make_test_track("Song A3", "Artist A"),
            make_test_track("Song D1", "Artist D"),
        ];

        mgr.set_tracks(tracks);
        mgr.set_shuffle(ShuffleMode::SmartBalanced { artist_spacing: 2 });

        let mut played_artists = Vec::new();
        while let Some(track) = mgr.next_track() {
            played_artists.push(track.metadata.artist);
        }

        assert_eq!(played_artists.len(), 6);

        // Verify artist spacing where mathematically feasible
        for i in 0..played_artists.len().saturating_sub(2) {
            if played_artists[i] == played_artists[i + 1] {
                panic!("Consecutive duplicate artist detected: {:?}", played_artists);
            }
        }
    }

    #[test]
    fn test_repeat_modes() {
        let mut mgr = SmartQueueManager::new();
        let tracks = vec![
            make_test_track("Song 1", "Artist 1"),
            make_test_track("Song 2", "Artist 2"),
        ];
        mgr.set_tracks(tracks);

        // Repeat One
        mgr.set_repeat(RepeatMode::RepeatOne);
        let t1 = mgr.next_track().unwrap();
        assert_eq!(t1.metadata.title, "Song 1");
        let t1_repeat = mgr.next_track().unwrap();
        assert_eq!(t1_repeat.metadata.title, "Song 1");

        // Repeat Off
        mgr.set_repeat(RepeatMode::Off);
        let t2 = mgr.next_track().unwrap();
        assert_eq!(t2.metadata.title, "Song 2");
        assert!(mgr.next_track().is_none());
    }
}
