//! Results of DJ track analysis: tempo, beat grid and waveforms (TASK-204).

#![forbid(unsafe_code)]

use crate::AudioSource;
use serde::{Deserialize, Serialize};

/// Bump when the analysis algorithm changes, so cached results are recomputed.
pub const ANALYSIS_VERSION: u32 = 1;

/// Hot cue slots per track.
pub const HOT_CUE_SLOTS: usize = 8;

/// A track's hot cues in seconds (`None` = empty slot).
pub type HotCues = [Option<f64>; HOT_CUE_SLOTS];

/// What the analyser found in a track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackAnalysis {
    /// Algorithm version that produced this ([`ANALYSIS_VERSION`]).
    pub version: u32,
    /// Track length in seconds.
    pub duration_secs: f64,
    /// Tempo, or `None` when no steady beat was found.
    pub bpm: Option<f64>,
    /// Time of a downbeat (the first beat of a bar) in seconds; the grid extends both ways.
    pub first_downbeat_secs: f64,
    /// Beats per bar (4 for nearly all dance music).
    pub beats_per_bar: u8,
    /// Whole-track overview: peak level per column, 0–255.
    pub overview: Vec<u8>,
    /// Detail waveform: low, mid and high band level per column, 0–255.
    pub detail: Vec<[u8; 3]>,
    /// Detail columns per second.
    pub detail_rate: f32,
}

impl TrackAnalysis {
    /// Seconds per beat, if the track has a tempo.
    pub fn beat_period(&self) -> Option<f64> {
        self.bpm.filter(|b| *b > 0.0).map(|b| 60.0 / b)
    }

    /// Time of beat `n` (beat 0 is the downbeat anchor; negative beats lie before it).
    pub fn beat_time(&self, n: i64) -> Option<f64> {
        self.beat_period()
            .map(|p| self.first_downbeat_secs + n as f64 * p)
    }

    /// The grid beat closest to `secs`.
    pub fn nearest_beat(&self, secs: f64) -> Option<f64> {
        let p = self.beat_period()?;
        let n = ((secs - self.first_downbeat_secs) / p).round();
        Some(self.first_downbeat_secs + n * p)
    }
}

/// Cache key for a track's analysis until stable track IDs exist (TASK-148): the source
/// itself, which names the same audio across sessions.
pub fn analysis_key(source: &AudioSource) -> String {
    match source {
        AudioSource::LocalFile { path } => format!("file:{path}"),
        AudioSource::NetworkSmb { share_url } => format!("smb:{share_url}"),
        AudioSource::DirectHttp { url } => format!("http:{url}"),
        AudioSource::YouTube { video_id } => format!("youtube:{video_id}"),
        AudioSource::Spotify { track_uri } => format!("spotify:{track_uri}"),
        AudioSource::Bandcamp { stream_url } => format!("bandcamp:{stream_url}"),
        AudioSource::SoundCloud { track_url } => format!("soundcloud:{track_url}"),
        AudioSource::Soulseek { username, filename } => format!("slsk:{username}/{filename}"),
        AudioSource::P2PShared { file_hash, .. } => format!("p2p:{file_hash}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(bpm: f64, first: f64) -> TrackAnalysis {
        TrackAnalysis {
            version: ANALYSIS_VERSION,
            duration_secs: 60.0,
            bpm: Some(bpm),
            first_downbeat_secs: first,
            beats_per_bar: 4,
            overview: Vec::new(),
            detail: Vec::new(),
            detail_rate: 100.0,
        }
    }

    #[test]
    fn beat_grid_extends_both_ways_from_the_downbeat() {
        let a = grid(120.0, 1.0);
        assert_eq!(a.beat_time(2), Some(2.0));
        assert_eq!(a.beat_time(-2), Some(0.0));
        assert_eq!(a.nearest_beat(2.2), Some(2.0));
        assert_eq!(a.nearest_beat(2.3), Some(2.5));
    }

    #[test]
    fn no_tempo_means_no_grid() {
        let mut a = grid(120.0, 0.0);
        a.bpm = None;
        assert_eq!(a.beat_time(1), None);
    }

    #[test]
    fn the_same_file_gets_the_same_key() {
        let a = AudioSource::LocalFile {
            path: "/m/a.flac".into(),
        };
        assert_eq!(analysis_key(&a), analysis_key(&a.clone()));
        assert_eq!(analysis_key(&a), "file:/m/a.flac");
    }
}
