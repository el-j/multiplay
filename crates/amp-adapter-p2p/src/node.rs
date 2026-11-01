//! A mesh node: a libp2p swarm on a private network. TCP, then the pre-shared-key handshake
//! (pnet: a wrong mesh code fails here, before any protocol data), then Noise and Yamux.
//! Request-response carries search and file chunks; gossipsub carries jam operations.

#![forbid(unsafe_code)]

use crate::catalog::{Catalog, SharedFile};
use crate::code::MeshCode;
use crate::protocol::{Request, Response, CHUNK, MAX_QUERY};
use amp_domain::jam_crdt::JamOperation;
use amp_domain::{AudioSource, Track};
use amp_ports::{AudioSourceStream, SharedPrecacheBuffer};
use futures::StreamExt;
use libp2p::core::upgrade::Version;
use libp2p::identity::Keypair;
use libp2p::pnet::{PnetConfig, PreSharedKey};
use libp2p::request_response::{self, OutboundRequestId, ProtocolSupport};
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{gossipsub, Multiaddr, PeerId, StreamProtocol, Swarm, Transport};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot};

/// How a node starts.
#[derive(Debug, Clone)]
pub struct MeshConfig {
    pub code: MeshCode,
    /// Shown to other members.
    pub name: String,
    /// 0 picks a free port.
    pub listen_port: u16,
    /// Listen on the local network and find members with mDNS; `false` stays on 127.0.0.1.
    pub discover_on_lan: bool,
    pub shared: Vec<SharedFile>,
    /// A saved identity (protobuf-encoded keypair) so the peer id stays the same; `None` makes one.
    pub identity: Option<Vec<u8>>,
}

#[derive(NetworkBehaviour)]
struct Behaviour {
    rr: request_response::cbor::Behaviour<Request, Response>,
    gossip: gossipsub::Behaviour,
}

type ChunkReply = oneshot::Sender<Result<(Vec<u8>, u64), String>>;

pub(crate) enum Command {
    Dial(Multiaddr),
    Peers(oneshot::Sender<Vec<String>>),
    ListenAddrs(oneshot::Sender<Vec<Multiaddr>>),
    Search {
        query: String,
        timeout: Duration,
        reply: oneshot::Sender<Vec<Track>>,
    },
    Chunk {
        peer: PeerId,
        id: String,
        offset: u64,
        reply: ChunkReply,
    },
    Publish(JamOperation),
    SetShared(Vec<SharedFile>),
    Shutdown,
}

/// A dial command for the discovery thread.
pub(crate) fn dial_command(addr: Multiaddr) -> Command {
    Command::Dial(addr)
}

struct Stop(mpsc::UnboundedSender<Command>);

impl Drop for Stop {
    fn drop(&mut self) {
        let _ = self.0.send(Command::Shutdown);
    }
}

/// A running node; cloning shares it, dropping the last clone stops it.
#[derive(Clone)]
pub struct MeshNode {
    tx: mpsc::UnboundedSender<Command>,
    peer_id: PeerId,
    jam: broadcast::Sender<JamOperation>,
    /// The runtime the node runs on; downloads run there too, whoever asks.
    runtime: tokio::runtime::Handle,
    _stop: Arc<Stop>,
}

struct PendingSearch {
    waiting: usize,
    results: Vec<Track>,
    reply: oneshot::Sender<Vec<Track>>,
    deadline: tokio::time::Instant,
}

impl MeshNode {
    /// Starts a node on the current tokio runtime.
    ///
    /// # Errors
    /// When the transport, behaviour or listener cannot be set up.
    pub async fn start(config: MeshConfig) -> Result<Self, String> {
        let keypair = config
            .identity
            .as_deref()
            .and_then(|b| Keypair::from_protobuf_encoding(b).ok())
            .unwrap_or_else(Keypair::generate_ed25519);
        let psk = PreSharedKey::new(config.code.0);
        let mut swarm: Swarm<Behaviour> = libp2p::SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_other_transport(|key| {
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
                    libp2p::tcp::tokio::Transport::new(
                        libp2p::tcp::Config::default().nodelay(true),
                    )
                    .and_then(move |socket, _| PnetConfig::new(psk).handshake(socket))
                    .upgrade(Version::V1Lazy)
                    .authenticate(libp2p::noise::Config::new(key)?)
                    .multiplex(libp2p::yamux::Config::default())
                    .timeout(Duration::from_secs(10))
                    .boxed(),
                )
            })
            .map_err(|e| e.to_string())?
            .with_behaviour(|key| {
                let rr = request_response::cbor::Behaviour::new(
                    [(
                        StreamProtocol::new("/multiplay/mesh/1"),
                        ProtocolSupport::Full,
                    )],
                    request_response::Config::default()
                        .with_request_timeout(Duration::from_secs(20)),
                );
                let gossip = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub::ConfigBuilder::default()
                        .heartbeat_interval(Duration::from_millis(500))
                        .max_transmit_size(256 * 1024)
                        .build()
                        .map_err(|e| e.to_string())?,
                )
                .map_err(str::to_string)?;
                Ok(Behaviour { rr, gossip })
            })
            .map_err(|e| e.to_string())?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(300)))
            .build();

        let topic =
            gossipsub::IdentTopic::new(format!("multiplay-jam-{}", config.code.discovery_id()));
        swarm
            .behaviour_mut()
            .gossip
            .subscribe(&topic)
            .map_err(|e| e.to_string())?;
        let ip = if config.discover_on_lan {
            "0.0.0.0"
        } else {
            "127.0.0.1"
        };
        let addr: Multiaddr = format!("/ip4/{ip}/tcp/{}", config.listen_port)
            .parse()
            .map_err(|e| format!("{e}"))?;
        swarm.listen_on(addr).map_err(|e| e.to_string())?;

        let peer_id = *swarm.local_peer_id();
        let (tx, rx) = mpsc::unbounded_channel();
        let (jam, _) = broadcast::channel(256);
        let mut catalog = Catalog::default();
        catalog.set(config.shared.clone());
        tokio::spawn(run(swarm, rx, catalog, topic, jam.clone()));
        let node = Self {
            tx: tx.clone(),
            peer_id,
            jam,
            runtime: tokio::runtime::Handle::current(),
            _stop: Arc::new(Stop(tx)),
        };
        if config.discover_on_lan {
            crate::discovery::start(&node, &config).await;
        }
        tracing::info!(peer = %peer_id, lan = config.discover_on_lan, "mesh node started");
        Ok(node)
    }

    /// This node's peer id.
    #[must_use]
    pub fn peer_id(&self) -> String {
        self.peer_id.to_string()
    }

    pub(crate) fn dial_tx(&self) -> mpsc::UnboundedSender<Command> {
        self.tx.clone()
    }

    /// Connects to a member at `addr`.
    pub async fn dial(&self, addr: Multiaddr) {
        let _ = self.tx.send(Command::Dial(addr));
    }

    /// Connected members' peer ids.
    pub async fn peers(&self) -> Vec<String> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Command::Peers(reply));
        rx.await.unwrap_or_default()
    }

    /// Where this node listens (loopback first).
    pub async fn listen_addrs(&self) -> Vec<Multiaddr> {
        for _ in 0..50 {
            let (reply, rx) = oneshot::channel();
            let _ = self.tx.send(Command::ListenAddrs(reply));
            let addrs = rx.await.unwrap_or_default();
            if !addrs.is_empty() {
                return addrs;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        Vec::new()
    }

    /// Searches every connected member's shared files.
    pub async fn search(&self, query: &str, timeout: Duration) -> Vec<Track> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Command::Search {
            query: query.chars().take(MAX_QUERY).collect(),
            timeout,
            reply,
        });
        rx.await.unwrap_or_default()
    }

    async fn chunk(&self, peer: PeerId, id: &str, offset: u64) -> Result<(Vec<u8>, u64), String> {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(Command::Chunk {
            peer,
            id: id.to_string(),
            offset,
            reply,
        });
        rx.await.map_err(|_| "the mesh node stopped".to_string())?
    }

    /// Streams shared file `id` from `peer`: the first chunk arrives before this returns (so a
    /// refused file is an error here), the rest in the background.
    ///
    /// # Errors
    /// A bad peer id, a file that is not shared, or a lost connection.
    pub async fn fetch(&self, peer: &str, id: &str) -> Result<Box<dyn AudioSourceStream>, String> {
        let peer: PeerId = peer.parse().map_err(|_| "not a peer id".to_string())?;
        let (first, total) = self.chunk(peer, id, 0).await?;
        let buffer = SharedPrecacheBuffer::new_blocking(total as usize, Duration::from_secs(20));
        buffer.push_chunk(&first);
        let first_len = first.len() as u64;
        let writer = buffer.clone();
        let node = self.clone();
        let id = id.to_string();
        self.runtime.spawn(async move {
            let mut offset = first_len;
            while offset < total && !writer.is_cancelled() {
                match node.chunk(peer, &id, offset).await {
                    Ok((data, _)) if !data.is_empty() => {
                        offset += data.len() as u64;
                        writer.push_chunk(&data);
                    }
                    _ => break,
                }
            }
            writer.mark_finished();
        });
        Ok(Box::new(buffer.into_reader()))
    }

    /// Replaces what this node shares.
    pub fn set_shared(&self, files: Vec<SharedFile>) {
        let _ = self.tx.send(Command::SetShared(files));
    }

    /// Sends a jam operation to every member.
    pub async fn publish_jam(&self, op: JamOperation) {
        let _ = self.tx.send(Command::Publish(op));
    }

    /// Jam operations from other members.
    #[must_use]
    pub fn jam_events(&self) -> broadcast::Receiver<JamOperation> {
        self.jam.subscribe()
    }
}

fn read_chunk(catalog: &Catalog, id: &str, offset: u64, len: u64) -> Response {
    let Some((path, total)) = catalog.path(id) else {
        return Response::Error("that file is not shared".into());
    };
    let read = || -> std::io::Result<Vec<u8>> {
        let mut f = std::fs::File::open(&path)?;
        f.seek(SeekFrom::Start(offset.min(total)))?;
        let mut data = Vec::new();
        f.take(len.min(CHUNK)).read_to_end(&mut data)?;
        Ok(data)
    };
    match read() {
        Ok(data) => Response::Chunk { data, total },
        Err(e) => Response::Error(e.to_string()),
    }
}

async fn run(
    mut swarm: Swarm<Behaviour>,
    mut rx: mpsc::UnboundedReceiver<Command>,
    mut catalog: Catalog,
    topic: gossipsub::IdentTopic,
    jam: broadcast::Sender<JamOperation>,
) {
    let mut searches: HashMap<u64, PendingSearch> = HashMap::new();
    let mut search_of: HashMap<OutboundRequestId, u64> = HashMap::new();
    let mut chunks: HashMap<OutboundRequestId, ChunkReply> = HashMap::new();
    let mut next_search = 0u64;
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    loop {
        tokio::select! {
            cmd = rx.recv() => match cmd {
                None | Some(Command::Shutdown) => break,
                Some(Command::Dial(addr)) => {
                    if let Err(e) = swarm.dial(addr) {
                        tracing::debug!("mesh dial failed: {e}");
                    }
                }
                Some(Command::Peers(reply)) => {
                    let _ = reply.send(swarm.connected_peers().map(ToString::to_string).collect());
                }
                Some(Command::ListenAddrs(reply)) => {
                    let mut addrs: Vec<Multiaddr> = swarm.listeners().cloned().collect();
                    addrs.sort_by_key(|a| !a.to_string().contains("127.0.0.1"));
                    let _ = reply.send(addrs);
                }
                Some(Command::Search { query, timeout, reply }) => {
                    let peers: Vec<PeerId> = swarm.connected_peers().copied().collect();
                    if peers.is_empty() {
                        let _ = reply.send(Vec::new());
                        continue;
                    }
                    next_search += 1;
                    for p in &peers {
                        let id = swarm.behaviour_mut().rr.send_request(p, Request::Search { query: query.clone() });
                        search_of.insert(id, next_search);
                    }
                    searches.insert(next_search, PendingSearch {
                        waiting: peers.len(),
                        results: Vec::new(),
                        reply,
                        deadline: tokio::time::Instant::now() + timeout,
                    });
                }
                Some(Command::Chunk { peer, id, offset, reply }) => {
                    let req = swarm.behaviour_mut().rr.send_request(&peer, Request::Fetch { id, offset, len: CHUNK });
                    chunks.insert(req, reply);
                }
                Some(Command::Publish(op)) => {
                    if let Ok(data) = serde_json::to_vec(&op) {
                        if let Err(e) = swarm.behaviour_mut().gossip.publish(topic.clone(), data) {
                            tracing::debug!("jam publish: {e}");
                        }
                    }
                }
                Some(Command::SetShared(files)) => catalog.set(files),
            },
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(BehaviourEvent::Rr(request_response::Event::Message { peer, message, .. })) => match message {
                    request_response::Message::Request { request, channel, .. } => {
                        let response = match request {
                            Request::Search { query } => {
                                Response::Results(catalog.search(&query.chars().take(MAX_QUERY).collect::<String>()))
                            }
                            Request::Fetch { id, offset, len } => read_chunk(&catalog, &id, offset, len),
                        };
                        let _ = swarm.behaviour_mut().rr.send_response(channel, response);
                    }
                    request_response::Message::Response { request_id, response } => {
                        if let Some(search) = search_of.remove(&request_id) {
                            if let Some(s) = searches.get_mut(&search) {
                                if let Response::Results(found) = response {
                                    s.results.extend(found.into_iter().map(|t| {
                                        Track::new(
                                            AudioSource::P2PShared { peer_id: peer.to_string(), file_hash: t.id.clone() },
                                            t.metadata(),
                                        )
                                    }));
                                }
                                s.waiting = s.waiting.saturating_sub(1);
                            }
                        } else if let Some(reply) = chunks.remove(&request_id) {
                            let _ = reply.send(match response {
                                Response::Chunk { data, total } => Ok((data, total)),
                                Response::Error(e) => Err(e),
                                Response::Results(_) => Err("unexpected answer".into()),
                            });
                        }
                    }
                },
                SwarmEvent::Behaviour(BehaviourEvent::Rr(request_response::Event::OutboundFailure { request_id, error, .. })) => {
                    if let Some(search) = search_of.remove(&request_id) {
                        if let Some(s) = searches.get_mut(&search) {
                            s.waiting = s.waiting.saturating_sub(1);
                        }
                    } else if let Some(reply) = chunks.remove(&request_id) {
                        let _ = reply.send(Err(error.to_string()));
                    }
                }
                SwarmEvent::Behaviour(BehaviourEvent::Gossip(gossipsub::Event::Message { message, .. })) => {
                    if let Ok(op) = serde_json::from_slice::<JamOperation>(&message.data) {
                        let _ = jam.send(op);
                    }
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => tracing::info!(peer = %peer_id, "mesh member connected"),
                _ => {}
            },
            _ = tick.tick() => {
                let now = tokio::time::Instant::now();
                let done: Vec<u64> = searches
                    .iter()
                    .filter(|(_, s)| s.waiting == 0 || now >= s.deadline)
                    .map(|(k, _)| *k)
                    .collect();
                for k in done {
                    if let Some(s) = searches.remove(&k) {
                        let _ = s.reply.send(s.results);
                    }
                }
            }
        }
    }
    tracing::info!("mesh node stopped");
}
