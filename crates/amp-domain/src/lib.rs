//! Zero-dependency pure domain core for multiplay.
//! All types are 100% pure Rust without unsafe code.

#![forbid(unsafe_code)]

pub mod analysis;
pub mod config;
pub mod deck_assign;
pub mod jam_crdt;
pub mod source_badge;

use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

pub use analysis::{analysis_key, HotCues, TrackAnalysis, ANALYSIS_VERSION, HOT_CUE_SLOTS};
pub use config::{
    AppConfig, AudioHardwareConfig, LibraryConfig, P2PNetworkConfig, RemoteControlConfig,
    StreamingProvidersConfig, UiAndThemeConfig,
};
pub use deck_assign::{next_for_deck, next_markers, DeckAssignment};
pub use jam_crdt::{CollaborativeQueue, JamItem, JamOperation, LamportTimestamp};
pub use source_badge::{SourceFilterMask, SourceKind};

/// Unique identifier for an audio track in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackId(pub Uuid);

impl TrackId {
    /// Generates a new random TrackId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TrackId {
    fn default() -> Self {
        Self::new()
    }
}

/// The origin source descriptor for resolving audio stream bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioSource {
    LocalFile { path: String },
    NetworkSmb { share_url: String },
    DirectHttp { url: String },
    YouTube { video_id: String },
    Spotify { track_uri: String },
    Bandcamp { stream_url: String },
    SoundCloud { track_url: String },
    Soulseek { username: String, filename: String },
    P2PShared { peer_id: String, file_hash: String },
}

impl AudioSource {
    /// Maps the audio source variant to its corresponding SourceKind badge.
    pub fn kind(&self) -> SourceKind {
        match self {
            Self::LocalFile { .. } => SourceKind::LocalFile,
            Self::NetworkSmb { .. } => SourceKind::NetworkSmb,
            Self::DirectHttp { .. } => SourceKind::DirectHttp,
            Self::YouTube { .. } => SourceKind::YouTube,
            Self::Spotify { .. } => SourceKind::Spotify,
            Self::Bandcamp { .. } => SourceKind::Bandcamp,
            Self::SoundCloud { .. } => SourceKind::SoundCloud,
            Self::Soulseek { .. } => SourceKind::Soulseek,
            Self::P2PShared { .. } => SourceKind::P2PShared,
        }
    }
}

/// Metadata attributes associated with an audio track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackMetadata {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub track_number: Option<u32>,
    pub duration: Option<Duration>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub bit_depth: Option<u16>,
    pub bitrate_kbps: Option<u32>,
    pub cover_art_uri: Option<String>,
    pub replay_gain_mb: Option<i32>,
    pub integrated_lufs_mb: Option<i32>,
}

impl TrackMetadata {
    pub fn new(title: impl Into<String>, artist: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            artist: artist.into(),
            album: None,
            track_number: None,
            duration: None,
            sample_rate: None,
            channels: None,
            bit_depth: None,
            bitrate_kbps: None,
            cover_art_uri: None,
            replay_gain_mb: None,
            integrated_lufs_mb: None,
        }
    }

    /// Returns ReplayGain track gain in decibels (e.g. -6.5 dB).
    pub fn replay_gain_db(&self) -> Option<f32> {
        self.replay_gain_mb.map(|mb| mb as f32 / 100.0)
    }

    /// Sets ReplayGain track gain in decibels.
    pub fn set_replay_gain_db(&mut self, gain_db: f32) {
        self.replay_gain_mb = Some((gain_db * 100.0).round() as i32);
    }

    /// Returns EBU R128 integrated loudness in LUFS (e.g. -14.0 LUFS).
    pub fn integrated_lufs(&self) -> Option<f32> {
        self.integrated_lufs_mb.map(|mb| mb as f32 / 100.0)
    }

    /// Sets EBU R128 integrated loudness in LUFS.
    pub fn set_integrated_lufs(&mut self, lufs: f32) {
        self.integrated_lufs_mb = Some((lufs * 100.0).round() as i32);
    }
}

/// A playable track entity with unique ID, source locator, metadata, and timestamps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub source: AudioSource,
    pub metadata: TrackMetadata,
    pub date_added_epoch: u64,
}

impl Track {
    pub fn new(source: AudioSource, metadata: TrackMetadata) -> Self {
        Self {
            id: TrackId::new(),
            source,
            metadata,
            date_added_epoch: 0,
        }
    }
}

/// High-level transport state for playback session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackState {
    Stopped,
    Buffering { progress_percent: u8 },
    Playing,
    Paused,
}

/// Normalized interleaved audio frame chunk (-1.0 to 1.0).
#[derive(Debug, Clone)]
pub struct AudioFrameChunk {
    pub samples: Vec<f32>,
    pub channels: u16,
    pub sample_rate: u32,
}

/// Configuration settings for memory precaching and hardware ring buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferConfiguration {
    pub precache_capacity_bytes: usize,
    pub ring_buffer_samples: usize,
    pub high_watermark_ratio: f32,
    pub low_watermark_ratio: f32,
}

impl Default for BufferConfiguration {
    fn default() -> Self {
        Self {
            precache_capacity_bytes: 64 * 1024 * 1024, // 64 MB
            ring_buffer_samples: 65536,
            high_watermark_ratio: 0.90,
            low_watermark_ratio: 0.20,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_creation_and_serialization() {
        let meta = TrackMetadata::new("Windowlicker", "Aphex Twin");
        let source = AudioSource::LocalFile {
            path: "/music/aphex.flac".to_string(),
        };
        let track = Track::new(source.clone(), meta);

        assert_eq!(track.source.kind(), SourceKind::LocalFile);
        assert_eq!(track.metadata.title, "Windowlicker");

        let json = serde_json::to_string(&track).expect("serialization failed");
        let deserialized: Track = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(track.id, deserialized.id);
        assert_eq!(track.metadata.artist, deserialized.metadata.artist);
    }

    #[test]
    fn test_playback_state_equality() {
        let s1 = PlaybackState::Buffering {
            progress_percent: 42,
        };
        let s2 = PlaybackState::Buffering {
            progress_percent: 42,
        };
        let s3 = PlaybackState::Playing;

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
    }

    #[test]
    fn test_audio_source_kind_mapping() {
        let sources = [
            (
                AudioSource::LocalFile {
                    path: "/a.flac".into(),
                },
                SourceKind::LocalFile,
            ),
            (
                AudioSource::NetworkSmb {
                    share_url: "smb://a/b".into(),
                },
                SourceKind::NetworkSmb,
            ),
            (
                AudioSource::DirectHttp {
                    url: "https://a.com/b.mp3".into(),
                },
                SourceKind::DirectHttp,
            ),
            (
                AudioSource::YouTube {
                    video_id: "dQw4w9WgXcQ".into(),
                },
                SourceKind::YouTube,
            ),
            (
                AudioSource::Spotify {
                    track_uri: "spotify:track:123".into(),
                },
                SourceKind::Spotify,
            ),
            (
                AudioSource::Bandcamp {
                    stream_url: "https://bc.com/a".into(),
                },
                SourceKind::Bandcamp,
            ),
            (
                AudioSource::SoundCloud {
                    track_url: "https://sc.com/a".into(),
                },
                SourceKind::SoundCloud,
            ),
            (
                AudioSource::Soulseek {
                    username: "u".into(),
                    filename: "f".into(),
                },
                SourceKind::Soulseek,
            ),
            (
                AudioSource::P2PShared {
                    peer_id: "p".into(),
                    file_hash: "h".into(),
                },
                SourceKind::P2PShared,
            ),
        ];

        for (source, expected_kind) in sources {
            assert_eq!(source.kind(), expected_kind);
        }
    }
}
