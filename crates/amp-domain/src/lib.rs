//! Zero-dependency pure domain core for multiplay.
//! All types are 100% pure Rust without unsafe code.

#![forbid(unsafe_code)]

pub mod about;
pub mod analysis;
pub mod config;
pub mod deck_assign;
pub mod formats;
pub mod jam_crdt;
pub mod source_badge;
pub mod source_url;

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
pub use source_url::{
    is_valid_youtube_id, parse_source_url, parse_time_param, parse_youtube_url, ParsedSourceUrl,
    YouTubeReference,
};

/// Unique identifier for an audio track in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackId(pub Uuid);

impl TrackId {
    /// Generates a new random TrackId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Computes a deterministic [`TrackId`] from the canonical URI of `source` (`TASK-148`).
    /// Rescanning the same local path or remote source always yields the exact same [`TrackId`].
    pub fn from_source(source: &AudioSource) -> Self {
        Self::from_key(&source.canonical_uri())
    }

    /// Computes a deterministic [`TrackId`] from a library key ([`Track::library_key`]).
    pub fn from_key(key: &str) -> Self {
        let bytes = key.as_bytes();
        // Dual 64-bit FNV-1a with distinct seeds + avalanche mixing to form 128 deterministic bits
        let mut h1: u64 = 0xcbf2_9ce4_8422_2325;
        let mut h2: u64 = 0x8422_2325_cbf2_9ce4;
        for (i, &b) in bytes.iter().enumerate() {
            h1 ^= b as u64;
            h1 = h1.wrapping_mul(0x0000_0100_0000_01b3);
            h2 ^= (b as u64).wrapping_add(i as u64);
            h2 = h2.wrapping_mul(0x0000_0100_0000_01b3).rotate_left(5);
        }
        h1 ^= h1 >> 33;
        h1 = h1.wrapping_mul(0xff51_afd7_ed55_8ccd);
        h1 ^= h1 >> 33;
        h2 ^= h2 >> 33;
        h2 = h2.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
        h2 ^= h2 >> 33;

        let mut uuid_bytes = [0u8; 16];
        uuid_bytes[..8].copy_from_slice(&h1.to_be_bytes());
        uuid_bytes[8..].copy_from_slice(&h2.to_be_bytes());
        // RFC 4122 variant + v5 version bits
        uuid_bytes[6] = (uuid_bytes[6] & 0x0f) | 0x50;
        uuid_bytes[8] = (uuid_bytes[8] & 0x3f) | 0x80;
        Self(Uuid::from_bytes(uuid_bytes))
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
    LocalFile {
        path: String,
    },
    NetworkSmb {
        share_url: String,
    },
    DirectHttp {
        url: String,
    },
    YouTube {
        video_id: String,
    },
    /// Kept so stored playlists and libraries still load; not playable (ADR-007).
    Spotify {
        track_uri: String,
    },
    Bandcamp {
        stream_url: String,
    },
    SoundCloud {
        track_url: String,
    },
    Soulseek {
        username: String,
        filename: String,
    },
    P2PShared {
        peer_id: String,
        file_hash: String,
    },
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

    /// Formats the source into a lossless canonical URI (`multiplay:` scheme for non-local sources,
    /// `TASK-148`, `TASK-152`).
    pub fn canonical_uri(&self) -> String {
        match self {
            Self::LocalFile { path } => format!("multiplay:file:{path}"),
            Self::NetworkSmb { share_url } => format!("multiplay:smb:{share_url}"),
            Self::DirectHttp { url } => format!("multiplay:http:{url}"),
            Self::YouTube { video_id } => format!("multiplay:youtube:{video_id}"),
            Self::Spotify { track_uri } => format!("multiplay:spotify:{track_uri}"),
            Self::Bandcamp { stream_url } => format!("multiplay:bandcamp:{stream_url}"),
            Self::SoundCloud { track_url } => format!("multiplay:soundcloud:{track_url}"),
            Self::Soulseek { username, filename } => {
                format!(
                    "multiplay:soulseek:{}:{}:{username}{filename}",
                    username.len(),
                    filename.len()
                )
            }
            Self::P2PShared { peer_id, file_hash } => {
                format!(
                    "multiplay:p2p:{}:{}:{peer_id}{file_hash}",
                    peer_id.len(),
                    file_hash.len()
                )
            }
        }
    }

    /// Parses a `multiplay:` canonical URI back into its exact [`AudioSource`] variant (`TASK-152`).
    pub fn from_canonical_uri(uri: &str) -> Option<Self> {
        let rest = uri.strip_prefix("multiplay:")?;
        if let Some(path) = rest.strip_prefix("file:") {
            Some(Self::LocalFile {
                path: path.to_string(),
            })
        } else if let Some(share_url) = rest.strip_prefix("smb:") {
            Some(Self::NetworkSmb {
                share_url: share_url.to_string(),
            })
        } else if let Some(url) = rest.strip_prefix("http:") {
            Some(Self::DirectHttp {
                url: url.to_string(),
            })
        } else if let Some(video_id) = rest.strip_prefix("youtube:") {
            Some(Self::YouTube {
                video_id: video_id.to_string(),
            })
        } else if let Some(track_uri) = rest.strip_prefix("spotify:") {
            Some(Self::Spotify {
                track_uri: track_uri.to_string(),
            })
        } else if let Some(stream_url) = rest.strip_prefix("bandcamp:") {
            Some(Self::Bandcamp {
                stream_url: stream_url.to_string(),
            })
        } else if let Some(track_url) = rest.strip_prefix("soundcloud:") {
            Some(Self::SoundCloud {
                track_url: track_url.to_string(),
            })
        } else if let Some(slsk) = rest.strip_prefix("soulseek:") {
            let (u_len, r1) = slsk.split_once(':')?;
            let (f_len, payload) = r1.split_once(':')?;
            let u_len: usize = u_len.parse().ok()?;
            let f_len: usize = f_len.parse().ok()?;
            if payload.len() != u_len + f_len {
                return None;
            }
            Some(Self::Soulseek {
                username: payload[..u_len].to_string(),
                filename: payload[u_len..].to_string(),
            })
        } else if let Some(p2p) = rest.strip_prefix("p2p:") {
            let (p_len, r1) = p2p.split_once(':')?;
            let (h_len, payload) = r1.split_once(':')?;
            let p_len: usize = p_len.parse().ok()?;
            let h_len: usize = h_len.parse().ok()?;
            if payload.len() != p_len + h_len {
                return None;
            }
            Some(Self::P2PShared {
                peer_id: payload[..p_len].to_string(),
                file_hash: payload[p_len..].to_string(),
            })
        } else {
            None
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
    /// Album artist (e.g. "Various Artists" on compilations). Missing in older data.
    #[serde(default)]
    pub album_artist: Option<String>,
    /// Disc number within a multi-disc release.
    #[serde(default)]
    pub disc_number: Option<u32>,
    /// Release or recording year.
    #[serde(default)]
    pub year: Option<u16>,
    #[serde(default)]
    pub genre: Option<String>,
    /// The part of the source this track covers, e.g. one song of a single-file album with a
    /// CUE sheet. `None` plays the whole source.
    #[serde(default)]
    pub segment: Option<TrackSegment>,
}

/// A time range within an audio source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackSegment {
    pub start: Duration,
    /// `None`: until the end of the source.
    pub end: Option<Duration>,
}

/// ReplayGain's reference loudness: a track with 0 dB ReplayGain gain is about -18 LUFS.
const REPLAY_GAIN_REFERENCE_LUFS: f32 = -18.0;

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
            album_artist: None,
            disc_number: None,
            year: None,
            genre: None,
            segment: None,
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

    /// Returns EBU R128 integrated loudness in LUFS (e.g. -14.0 LUFS): measured or from an R128
    /// tag, otherwise derived from the ReplayGain track gain.
    pub fn integrated_lufs(&self) -> Option<f32> {
        self.integrated_lufs_mb
            .map(|mb| mb as f32 / 100.0)
            .or_else(|| {
                self.replay_gain_db()
                    .map(|g| REPLAY_GAIN_REFERENCE_LUFS - g)
            })
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

    /// What makes this a distinct library entry: the source, plus the start of the part it
    /// plays for CUE tracks (several songs in one file). Equal to the source's canonical URI
    /// for whole files, so their keys never changed.
    #[must_use]
    pub fn library_key(&self) -> String {
        let uri = self.source.canonical_uri();
        match self.metadata.segment {
            Some(seg) => format!("{uri}#t={}", seg.start.as_millis()),
            None => uri,
        }
    }

    /// Creates a [`Track`] whose [`TrackId`] is deterministically derived from `source` (`TASK-148`).
    pub fn with_stable_id(source: AudioSource, metadata: TrackMetadata) -> Self {
        let id = TrackId::from_source(&source);
        Self {
            id,
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
    Loading,
    Buffering { progress_percent: u8 },
    Playing,
    Paused,
    Error,
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
