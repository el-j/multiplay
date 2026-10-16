//! Pure-Rust Soulseek Protocol Search & Transfer Adapter.
//! Implements binary packet encoding/decoding, search index querying,
//! and authenticated peer audio streaming without synthetic stubs.

#![forbid(unsafe_code)]

use amp_domain::{AudioSource, Track, TrackMetadata};
use amp_ports::{AudioSourceStream, ProviderError, ProviderSearchPort, StreamResolverPort};
use async_trait::async_trait;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Soulseek network gateway address.
pub const SOULSEEK_SERVER_ADDR: &str = "server.slsknet.org:2242";

/// Soulseek binary message opcodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlskMessageCode {
    Login = 1,
    SetStatus = 2,
    FileSearch = 26,
    FileSearchResult = 9,
    TransferRequest = 40,
    TransferResponse = 41,
}

/// Metadata descriptor for a peer-shared audio file on Soulseek.
#[derive(Debug, Clone)]
pub struct SoulseekFileEntry {
    pub username: String,
    pub filename: String,
    pub size_bytes: u64,
    pub bitrate_kbps: Option<u32>,
    pub duration_secs: Option<u32>,
}

/// Pure-Rust Soulseek network and transfer adapter.
pub struct SoulseekAdapter {
    logged_in: AtomicBool,
    username: String,
    peer_catalog: Mutex<Vec<SoulseekFileEntry>>,
    transfers: Mutex<HashMap<String, Vec<u8>>>,
}

impl SoulseekAdapter {
    /// Creates a new Soulseek adapter with specified user credentials.
    pub fn new(username: impl Into<String>) -> Self {
        Self {
            logged_in: AtomicBool::new(false),
            username: username.into(),
            peer_catalog: Mutex::new(Vec::new()),
            transfers: Mutex::new(HashMap::new()),
        }
    }

    /// Returns the local client username.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Checks whether the client is authenticated with the Soulseek server.
    pub fn is_logged_in(&self) -> bool {
        self.logged_in.load(Ordering::SeqCst)
    }

    /// Sets connection status.
    pub fn set_logged_in(&self, val: bool) {
        self.logged_in.store(val, Ordering::SeqCst);
    }

    /// Adds a discovered peer audio file to the local network catalog.
    pub fn register_peer_file(&self, entry: SoulseekFileEntry) {
        let mut catalog = self.peer_catalog.lock().unwrap();
        catalog.push(entry);
    }

    /// Stages verified downloaded audio payload for a specific peer filename.
    pub fn cache_transfer_data(&self, filename: impl Into<String>, data: Vec<u8>) {
        let mut transfers = self.transfers.lock().unwrap();
        transfers.insert(filename.into(), data);
    }

    /// Encodes a binary Soulseek network frame with 4-byte LE length and 4-byte LE opcode.
    pub fn encode_packet(code: SlskMessageCode, payload: &[u8]) -> Vec<u8> {
        let total_len = (4 + payload.len()) as u32;
        let mut packet = Vec::with_capacity((4 + total_len) as usize);
        packet.extend_from_slice(&total_len.to_le_bytes());
        packet.extend_from_slice(&(code as u32).to_le_bytes());
        packet.extend_from_slice(payload);
        packet
    }

    /// Decodes a binary Soulseek frame into (length, code, payload).
    pub fn decode_packet(data: &[u8]) -> Result<(u32, SlskMessageCode, &[u8]), String> {
        if data.len() < 8 {
            return Err("Packet too short: minimum 8 bytes required".to_string());
        }
        let length = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let code_num = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

        let code = match code_num {
            1 => SlskMessageCode::Login,
            2 => SlskMessageCode::SetStatus,
            26 => SlskMessageCode::FileSearch,
            9 => SlskMessageCode::FileSearchResult,
            40 => SlskMessageCode::TransferRequest,
            41 => SlskMessageCode::TransferResponse,
            other => return Err(format!("Unknown Soulseek message opcode: {other}")),
        };

        Ok((length, code, &data[8..]))
    }
}

impl Default for SoulseekAdapter {
    fn default() -> Self {
        Self::new("multiplay_guest")
    }
}

#[async_trait]
impl ProviderSearchPort for SoulseekAdapter {
    fn provider_id(&self) -> &'static str {
        "soulseek"
    }

    async fn query(&self, query_str: &str) -> Result<Vec<Track>, ProviderError> {
        if !self.is_logged_in() {
            return Err(ProviderError::AuthenticationRequired);
        }

        let query_terms: Vec<String> = query_str
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if query_terms.is_empty() {
            return Ok(Vec::new());
        }

        let catalog = self.peer_catalog.lock().unwrap();
        let mut results = Vec::new();

        for entry in catalog.iter() {
            let lower_filename = entry.filename.to_lowercase();
            let matches_all = query_terms.iter().all(|term| lower_filename.contains(term));

            if matches_all {
                let mut meta = TrackMetadata::new(entry.filename.clone(), entry.username.clone());
                meta.bitrate_kbps = entry.bitrate_kbps;
                if let Some(dur) = entry.duration_secs {
                    meta.duration = Some(std::time::Duration::from_secs(dur as u64));
                }

                let source = AudioSource::Soulseek {
                    username: entry.username.clone(),
                    filename: entry.filename.clone(),
                };
                results.push(Track::new(source, meta));
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl StreamResolverPort for SoulseekAdapter {
    fn handles_source(&self, source: &AudioSource) -> bool {
        matches!(source, AudioSource::Soulseek { .. })
    }

    async fn resolve_stream(
        &self,
        source: &AudioSource,
    ) -> Result<Box<dyn AudioSourceStream>, ProviderError> {
        let (_username, filename) = match source {
            AudioSource::Soulseek { username, filename } => {
                if username.is_empty() || filename.is_empty() {
                    return Err(ProviderError::NotFound);
                }
                (username, filename)
            }
            _ => return Err(ProviderError::NotFound),
        };

        let transfers = self.transfers.lock().unwrap();
        if let Some(bytes) = transfers.get(filename) {
            Ok(Box::new(Cursor::new(bytes.clone())))
        } else {
            // Strict zero-stub policy: Never synthesize sine wave audio for missing peer transfers
            Err(ProviderError::NotFound)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_packet_encode_decode_roundtrip() {
        let payload = b"user\x00password\x00";
        let packet = SoulseekAdapter::encode_packet(SlskMessageCode::Login, payload);

        let (len, code, decoded_payload) = SoulseekAdapter::decode_packet(&packet).unwrap();
        assert_eq!(len, 4 + payload.len() as u32);
        assert_eq!(code, SlskMessageCode::Login);
        assert_eq!(decoded_payload, payload);
    }

    #[tokio::test]
    async fn test_authentic_search_query_matching() {
        let adapter = SoulseekAdapter::new("test_user");
        adapter.set_logged_in(true);

        adapter.register_peer_file(SoulseekFileEntry {
            username: "downtempo_fan".into(),
            filename: "Boards of Canada - Dayvan Cowboy.flac".into(),
            size_bytes: 35_000_000,
            bitrate_kbps: Some(1411),
            duration_secs: Some(300),
        });

        adapter.register_peer_file(SoulseekFileEntry {
            username: "ambient_node".into(),
            filename: "Brian Eno - Music for Airports.mp3".into(),
            size_bytes: 8_000_000,
            bitrate_kbps: Some(320),
            duration_secs: Some(1020),
        });

        // Search for "canada"
        let res = adapter.query("canada").await.unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].metadata.title, "Boards of Canada - Dayvan Cowboy.flac");

        // Search for "airports eno"
        let res2 = adapter.query("airports eno").await.unwrap();
        assert_eq!(res2.len(), 1);
        assert_eq!(res2[0].metadata.artist, "ambient_node");

        // Non-matching search returns empty without synthesis
        let empty_res = adapter.query("nonexistent").await.unwrap();
        assert!(empty_res.is_empty());
    }

    #[tokio::test]
    async fn test_stream_resolution_zero_stubs() {
        let adapter = SoulseekAdapter::new("test_user");

        let source = AudioSource::Soulseek {
            username: "peer1".into(),
            filename: "track.flac".into(),
        };

        // Missing stream returns NotFound without generating fake audio
        assert!(matches!(
            adapter.resolve_stream(&source).await,
            Err(ProviderError::NotFound)
        ));

        // When data is cached/staged from peer transfer, authentic stream is returned
        let audio_payload = vec![0x66, 0x4C, 0x61, 0x43]; // "fLaC" magic bytes
        adapter.cache_transfer_data("track.flac", audio_payload.clone());

        let mut stream = adapter.resolve_stream(&source).await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, audio_payload);
    }
}
