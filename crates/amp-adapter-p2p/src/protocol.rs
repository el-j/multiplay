//! Messages between mesh peers (request-response over `/multiplay/mesh/1`, CBOR).

#![forbid(unsafe_code)]

use amp_domain::TrackMetadata;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Bytes per fetch reply.
pub const CHUNK: u64 = 256 * 1024;
/// Longest search text accepted.
pub const MAX_QUERY: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    Search { query: String },
    Fetch { id: String, offset: u64, len: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Results(Vec<SharedTrack>),
    Chunk { data: Vec<u8>, total: u64 },
    Error(String),
}

/// A shared file as peers see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_secs: Option<u64>,
    pub size: u64,
}

impl SharedTrack {
    #[must_use]
    pub fn from_metadata(id: String, m: &TrackMetadata, size: u64) -> Self {
        Self {
            id,
            title: m.title.clone(),
            artist: m.artist.clone(),
            album: m.album.clone(),
            duration_secs: m.duration.map(|d| d.as_secs()),
            size,
        }
    }

    #[must_use]
    pub fn metadata(&self) -> TrackMetadata {
        let mut m = TrackMetadata::new(self.title.clone(), self.artist.clone());
        m.album = self.album.clone();
        m.duration = self.duration_secs.map(Duration::from_secs);
        m
    }
}
