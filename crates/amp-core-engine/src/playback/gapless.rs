//! Pure-Rust Gapless Playback Double-Buffering & Underrun Resilience Engine.
//! Handles next-track pre-buffering, PTS timestamp mapping, and click-free comfort silence.

#![forbid(unsafe_code)]

use amp_domain::Track;
use std::time::Duration;

/// Configuration for gapless track transitions and pre-roll buffers.
#[derive(Debug, Clone)]
pub struct GaplessConfig {
    pub prefetch_lead_time: Duration,
    pub crossfade_duration: Duration,
    pub underrun_fade_samples: usize,
}

impl Default for GaplessConfig {
    fn default() -> Self {
        Self {
            prefetch_lead_time: Duration::from_secs(2),
            crossfade_duration: Duration::from_millis(50),
            underrun_fade_samples: 256,
        }
    }
}

/// Double-buffering track transition scheduler.
pub struct GaplessTrackScheduler {
    config: GaplessConfig,
    active_track: Option<Track>,
    next_track: Option<Track>,
    next_track_primed: bool,
    elapsed: Duration,
    total_duration: Option<Duration>,
}

impl GaplessTrackScheduler {
    /// Creates a new gapless scheduler with given configuration.
    pub fn new(config: GaplessConfig) -> Self {
        Self {
            config,
            active_track: None,
            next_track: None,
            next_track_primed: false,
            elapsed: Duration::ZERO,
            total_duration: None,
        }
    }

    /// Sets the currently playing track and its total duration.
    pub fn set_active_track(&mut self, track: Track, duration: Option<Duration>) {
        self.active_track = Some(track);
        self.total_duration = duration;
        self.elapsed = Duration::ZERO;
        self.next_track_primed = false;
    }

    /// Queues the upcoming track for gapless double-buffering.
    pub fn queue_next_track(&mut self, track: Track) {
        self.next_track = Some(track);
        self.next_track_primed = false;
    }

    /// Updates playback progress and checks if the upcoming track should be primed.
    pub fn update_progress(&mut self, current_pos: Duration) -> bool {
        self.elapsed = current_pos;
        if let Some(total) = self.total_duration {
            if total > self.config.prefetch_lead_time {
                let trigger_time = total - self.config.prefetch_lead_time;
                if current_pos >= trigger_time
                    && !self.next_track_primed
                    && self.next_track.is_some()
                {
                    self.next_track_primed = true;
                    return true; // Prime next track now
                }
            }
        }
        false
    }

    /// Advances to the primed next track, returning the new active track.
    pub fn advance_to_next(&mut self, next_duration: Option<Duration>) -> Option<Track> {
        let upcoming = self.next_track.take()?;
        self.set_active_track(upcoming.clone(), next_duration);
        Some(upcoming)
    }

    /// Checks whether the upcoming track has been primed into memory.
    pub fn is_next_primed(&self) -> bool {
        self.next_track_primed
    }
}

/// Generates smooth, click-free comfort silence envelopes during buffer underflows.
pub struct ComfortSilenceGenerator {
    fade_len: usize,
}

impl ComfortSilenceGenerator {
    /// Creates a comfort silence generator with specified fade envelope length in samples.
    pub fn new(fade_len: usize) -> Self {
        Self {
            fade_len: fade_len.max(16),
        }
    }

    /// Fades out the tail of a buffer to 0.0 using a smooth cosine window to prevent clicks.
    pub fn apply_fade_out(&self, buffer: &mut [f32]) {
        let len = buffer.len().min(self.fade_len);
        let start = buffer.len() - len;
        for (i, sample) in buffer[start..].iter_mut().enumerate() {
            let ratio = i as f32 / len as f32;
            let gain = 0.5 * (1.0 + (ratio * std::f32::consts::PI).cos());
            *sample *= gain;
        }
    }

    /// Fills an underflowing buffer slice with silence (0.0).
    pub fn fill_silence(&self, buffer: &mut [f32]) {
        buffer.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::{AudioSource, TrackMetadata};

    fn make_test_track(title: &str) -> Track {
        Track::new(
            AudioSource::LocalFile {
                path: format!("/music/{title}.flac"),
            },
            TrackMetadata::new(title, "Test Artist"),
        )
    }

    #[test]
    fn test_gapless_prefetch_trigger() {
        let mut scheduler = GaplessTrackScheduler::new(GaplessConfig::default());
        let t1 = make_test_track("Song 1");
        let t2 = make_test_track("Song 2");

        scheduler.set_active_track(t1, Some(Duration::from_secs(180)));
        scheduler.queue_next_track(t2);

        // Before prefetch window (at 170s of 180s track with 2s lead): should not prime
        assert!(!scheduler.update_progress(Duration::from_secs(170)));
        assert!(!scheduler.is_next_primed());

        // At 178s (within 2s lead): should trigger priming
        assert!(scheduler.update_progress(Duration::from_secs(178)));
        assert!(scheduler.is_next_primed());

        // Subsequent updates in the window should not re-trigger
        assert!(!scheduler.update_progress(Duration::from_secs(179)));

        // Advance to next track
        let advanced = scheduler
            .advance_to_next(Some(Duration::from_secs(200)))
            .unwrap();
        assert_eq!(advanced.metadata.title, "Song 2");
        assert!(!scheduler.is_next_primed());
    }

    #[test]
    fn test_comfort_silence_fade_out() {
        let gen = ComfortSilenceGenerator::new(64);
        let mut buffer = vec![1.0f32; 128];

        gen.apply_fade_out(&mut buffer);

        // Head unchanged
        assert_eq!(buffer[0], 1.0);
        assert_eq!(buffer[63], 1.0);

        // Tail smoothly faded down towards 0.0
        assert!(buffer[127] < 0.05);
        assert!(buffer[100] < buffer[80]);
    }
}
