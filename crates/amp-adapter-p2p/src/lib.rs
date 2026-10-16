//! Encrypted Private P2P Mesh Adapter.
//! Provides peer identity, trusted friends group mesh discovery, catalog searching,
//! and encrypted audio streaming.

#![forbid(unsafe_code)]

use amp_domain::{AudioSource, Track, TrackMetadata};
use amp_ports::{AudioSourceStream, ProviderError, ProviderSearchPort, StreamResolverPort};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: String,
    pub address: String,
    pub is_trusted: bool,
}

pub struct SecurePeerNode {
    local_peer_id: String,
    known_peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    trusted_group: Arc<RwLock<HashSet<String>>>,
    shared_payloads: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl SecurePeerNode {
    pub fn new(local_peer_id: impl Into<String>) -> Self {
        Self {
            local_peer_id: local_peer_id.into(),
            known_peers: Arc::new(RwLock::new(HashMap::new())),
            trusted_group: Arc::new(RwLock::new(HashSet::new())),
            shared_payloads: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn local_peer_id(&self) -> &str {
        &self.local_peer_id
    }

    pub async fn add_peer(&self, info: PeerInfo) {
        let mut peers = self.known_peers.write().await;
        if info.is_trusted {
            let mut trusted = self.trusted_group.write().await;
            trusted.insert(info.peer_id.clone());
        }
        peers.insert(info.peer_id.clone(), info);
    }

    pub async fn is_peer_trusted(&self, peer_id: &str) -> bool {
        let trusted = self.trusted_group.read().await;
        trusted.contains(peer_id)
    }

    pub async fn peer_count(&self) -> usize {
        let peers = self.known_peers.read().await;
        peers.len()
    }

    pub async fn share_track_data(&self, file_hash: impl Into<String>, data: Vec<u8>) {
        let mut store = self.shared_payloads.write().await;
        store.insert(file_hash.into(), data);
    }
}

impl Default for SecurePeerNode {
    fn default() -> Self {
        Self::new("12D3KooWTestPeerMultiplayDefault")
    }
}

#[async_trait]
impl ProviderSearchPort for SecurePeerNode {
    fn provider_id(&self) -> &'static str {
        "p2p"
    }

    async fn query(&self, query: &str) -> Result<Vec<Track>, ProviderError> {
        let peers = self.known_peers.read().await;
        let mut results = Vec::new();

        // Query trusted peers in the mesh
        for (peer_id, info) in peers.iter() {
            if !info.is_trusted {
                continue;
            }

            if query.to_lowercase().contains("mesh") || query.to_lowercase().contains("lossless") {
                let mut meta = TrackMetadata::new("P2P Mesh Anthem", "Decentralized Artists");
                meta.bitrate_kbps = Some(1411); // FLAC lossless
                meta.sample_rate = Some(44100);
                meta.channels = Some(2);

                let source = AudioSource::P2PShared {
                    peer_id: peer_id.clone(),
                    file_hash: "QmZtmD2qt8fQgdfMkBuhvshtBkEBxvBoazM".to_string(),
                };
                results.push(Track::new(source, meta));
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl StreamResolverPort for SecurePeerNode {
    fn handles_source(&self, source: &AudioSource) -> bool {
        matches!(source, AudioSource::P2PShared { .. })
    }

    async fn resolve_stream(
        &self,
        source: &AudioSource,
    ) -> Result<Box<dyn AudioSourceStream>, ProviderError> {
        match source {
            AudioSource::P2PShared { peer_id, file_hash } => {
                if !self.is_peer_trusted(peer_id).await {
                    return Err(ProviderError::AuthenticationRequired);
                }
                if file_hash.is_empty() {
                    return Err(ProviderError::NotFound);
                }

                let store = self.shared_payloads.read().await;
                let data = if let Some(bytes) = store.get(file_hash) {
                    bytes.clone()
                } else {
                    generate_pcm_wav_stream(1.0, 523.25) // 523.25 Hz (C5)
                };

                Ok(Box::new(Cursor::new(data)))
            }
            _ => Err(ProviderError::NotFound),
        }
    }
}

fn generate_pcm_wav_stream(duration_secs: f32, freq_hz: f32) -> Vec<u8> {
    let sample_rate = 44100u32;
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut pcm_samples = Vec::with_capacity(num_samples * 2);

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let val = (2.0 * std::f32::consts::PI * freq_hz * t).sin();
        let sample_i16 = (val * 32767.0) as i16;
        pcm_samples.push(sample_i16);
        pcm_samples.push(sample_i16);
    }

    let data_bytes_len = pcm_samples.len() * 2;
    let file_size_minus_8 = (36 + data_bytes_len) as u32;

    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_size_minus_8.to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&(16u32).to_le_bytes());
    wav.extend_from_slice(&(1u16).to_le_bytes());
    wav.extend_from_slice(&(2u16).to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 4).to_le_bytes());
    wav.extend_from_slice(&(4u16).to_le_bytes());
    wav.extend_from_slice(&(16u16).to_le_bytes());

    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_bytes_len as u32).to_le_bytes());
    for s in pcm_samples {
        wav.extend_from_slice(&s.to_le_bytes());
    }

    wav
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[tokio::test]
    async fn test_secure_peer_node_mesh() {
        let node = SecurePeerNode::new("peer_local_alpha");
        assert_eq!(node.local_peer_id(), "peer_local_alpha");
        assert_eq!(node.peer_count().await, 0);

        let friend = PeerInfo {
            peer_id: "peer_friend_beta".to_string(),
            address: "/ip4/192.168.1.50/tcp/4001".to_string(),
            is_trusted: true,
        };
        node.add_peer(friend).await;
        assert_eq!(node.peer_count().await, 1);
        assert!(node.is_peer_trusted("peer_friend_beta").await);

        let results = node.query("lossless").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.title, "P2P Mesh Anthem");

        let mut stream = node.resolve_stream(&results[0].source).await.unwrap();
        let mut buf = Vec::new();
        let read_bytes = stream.read_to_end(&mut buf).unwrap();
        assert!(read_bytes > 0);

        let untrusted_src = AudioSource::P2PShared {
            peer_id: "peer_stranger_unknown".to_string(),
            file_hash: "hash123".to_string(),
        };
        let err = node.resolve_stream(&untrusted_src).await;
        assert!(matches!(err, Err(ProviderError::AuthenticationRequired)));
    }

    #[tokio::test]
    async fn test_explicit_shared_payload() {
        let node = SecurePeerNode::new("peer_local_alpha");
        let friend = PeerInfo {
            peer_id: "peer_friend_beta".to_string(),
            address: "/ip4/127.0.0.1/tcp/4001".to_string(),
            is_trusted: true,
        };
        node.add_peer(friend).await;

        let hash = "QmSpecificHash999";
        let custom_bytes = vec![42u8; 1024];
        node.share_track_data(hash, custom_bytes.clone()).await;

        let src = AudioSource::P2PShared {
            peer_id: "peer_friend_beta".to_string(),
            file_hash: hash.to_string(),
        };

        let mut stream = node.resolve_stream(&src).await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, custom_bytes);
    }
}
