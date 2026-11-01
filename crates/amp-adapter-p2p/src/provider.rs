//! The mesh as a search provider and stream resolver. Starts empty (mesh off); the app joins a
//! mesh with [`MeshProvider::join`] and leaves with [`MeshProvider::leave`].

#![forbid(unsafe_code)]

use crate::node::{MeshConfig, MeshNode};
use amp_domain::{AudioSource, Track};
use amp_ports::{AudioSourceStream, ProviderError, ProviderSearchPort, StreamResolverPort};
use async_trait::async_trait;
use std::time::Duration;
use tokio::sync::RwLock;

/// How long a search waits for members to answer.
const SEARCH_WAIT: Duration = Duration::from_secs(3);

/// The current mesh, if joined.
#[derive(Default)]
pub struct MeshProvider {
    node: RwLock<Option<MeshNode>>,
}

impl MeshProvider {
    /// Joins a mesh, leaving any previous one.
    ///
    /// # Errors
    /// When the node cannot start.
    pub async fn join(&self, config: MeshConfig) -> Result<MeshNode, String> {
        self.leave().await;
        let node = MeshNode::start(config).await?;
        *self.node.write().await = Some(node.clone());
        Ok(node)
    }

    /// Leaves the mesh (the node stops when its last handle goes).
    pub async fn leave(&self) {
        self.node.write().await.take();
    }

    /// The joined node.
    pub async fn node(&self) -> Option<MeshNode> {
        self.node.read().await.clone()
    }
}

#[async_trait]
impl ProviderSearchPort for MeshProvider {
    fn provider_id(&self) -> &'static str {
        "mesh"
    }

    async fn query(&self, query: &str) -> Result<Vec<Track>, ProviderError> {
        match self.node().await {
            Some(node) => Ok(node.search(query, SEARCH_WAIT).await),
            None => Ok(Vec::new()),
        }
    }
}

#[async_trait]
impl StreamResolverPort for MeshProvider {
    fn handles_source(&self, source: &AudioSource) -> bool {
        matches!(source, AudioSource::P2PShared { .. })
    }

    async fn resolve_stream(
        &self,
        source: &AudioSource,
    ) -> Result<Box<dyn AudioSourceStream>, ProviderError> {
        let AudioSource::P2PShared { peer_id, file_hash } = source else {
            return Err(ProviderError::NotFound);
        };
        let node = self
            .node()
            .await
            .ok_or_else(|| ProviderError::Unavailable("not in a mesh".into()))?;
        node.fetch(peer_id, file_hash)
            .await
            .map_err(ProviderError::Network)
    }
}
