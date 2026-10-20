//! Shared harness for the remote daemon integration tests: every test binds its own loopback
//! listener on an ephemeral port; nothing touches the LAN.

#![allow(dead_code)] // each test binary uses a different subset

use amp_adapter_remote::{HostRequest, RemoteDaemon, RemoteDaemonConfig};
use amp_core_engine::events::PlayerCommand;
use amp_core_engine::EventBus;
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tokio_tungstenite::WebSocketStream;

pub type Ws = WebSocketStream<TcpStream>;

pub struct Harness {
    pub daemon: Arc<RemoteDaemon>,
    pub addr: SocketAddr,
    pub cmd_rx: UnboundedReceiver<PlayerCommand>,
    /// Track requests and searches the daemon hands to the host for moderation.
    pub requests: UnboundedReceiver<HostRequest>,
}

pub async fn start_with(mut config: RemoteDaemonConfig) -> Harness {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    config.bind_addr = addr;
    let (bus, cmd_rx) = EventBus::new(64);
    let daemon = Arc::new(RemoteDaemon::new(config, Arc::new(bus)));
    let requests = daemon
        .take_request_receiver()
        .expect("request receiver is handed out once");
    let runner = Arc::clone(&daemon);
    tokio::spawn(async move {
        let _ = runner.run_with_listener(listener).await;
    });
    Harness {
        daemon,
        addr,
        cmd_rx,
        requests,
    }
}

pub async fn start() -> Harness {
    start_with(RemoteDaemonConfig::loopback(0)).await
}

/// Opens a WebSocket to `/ws?{query}` with optional Origin and Host overrides.
#[allow(clippy::result_large_err)] // tungstenite's error type
pub async fn connect(
    addr: SocketAddr,
    query: &str,
    origin: Option<&str>,
    host: Option<&str>,
) -> Result<Ws, WsError> {
    connect_as(addr, query, origin, host, None).await
}

/// Like [`connect`], also sending a `User-Agent` (the host names devices after it).
#[allow(clippy::result_large_err)]
pub async fn connect_as(
    addr: SocketAddr,
    query: &str,
    origin: Option<&str>,
    host: Option<&str>,
    user_agent: Option<&str>,
) -> Result<Ws, WsError> {
    let url = format!("ws://{addr}/ws?{query}");
    let mut req = url.into_client_request()?;
    let default_origin = format!("http://{addr}");
    let origin = origin.unwrap_or(&default_origin);
    if !origin.is_empty() {
        req.headers_mut()
            .insert("Origin", HeaderValue::from_str(origin).unwrap());
    }
    if let Some(host) = host {
        req.headers_mut()
            .insert("Host", HeaderValue::from_str(host).unwrap());
    }
    if let Some(ua) = user_agent {
        req.headers_mut()
            .insert("User-Agent", HeaderValue::from_str(ua).unwrap());
    }
    let tcp = TcpStream::connect(addr).await.expect("tcp connect");
    let (ws, _) = tokio_tungstenite::client_async(req, tcp).await?;
    Ok(ws)
}

pub fn http_status(err: WsError) -> u16 {
    match err {
        WsError::Http(resp) => resp.status().as_u16(),
        other => panic!("expected HTTP rejection, got {other:?}"),
    }
}

pub async fn next_text(ws: &mut Ws) -> String {
    let msg = tokio::time::timeout(Duration::from_secs(2), ws.next())
        .await
        .expect("timeout waiting for server message")
        .expect("stream ended")
        .expect("ws error");
    match msg {
        Message::Text(t) => t.to_string(),
        other => panic!("expected text frame, got {other:?}"),
    }
}

/// Next server message of type `kind`, skipping status broadcasts and other events.
pub async fn next_of(ws: &mut Ws, kind: &str) -> serde_json::Value {
    for _ in 0..32 {
        let v: serde_json::Value = serde_json::from_str(&next_text(ws).await).unwrap();
        if v["type"] == kind {
            return v;
        }
    }
    panic!("no {kind} message received");
}

/// Sends one protocol message.
pub async fn send(ws: &mut Ws, json: &str) {
    ws.send(Message::Text(json.to_string().into()))
        .await
        .expect("send");
}

/// Pairs with a fresh token and returns the socket plus the granted session secret.
pub async fn pair(h: &Harness) -> (Ws, String) {
    pair_as(h, None).await
}

/// Pairs while presenting `user_agent`.
pub async fn pair_as(h: &Harness, user_agent: Option<&str>) -> (Ws, String) {
    let token = h.daemon.issue_pairing_token(Duration::from_secs(60));
    let mut ws = connect_as(h.addr, &format!("token={token}"), None, None, user_agent)
        .await
        .expect("pairing with a valid token must succeed");
    let granted: serde_json::Value = serde_json::from_str(&next_text(&mut ws).await).unwrap();
    assert_eq!(granted["type"], "SessionGranted");
    let session = granted["payload"]["session"].as_str().unwrap().to_string();
    assert_eq!(
        session.len(),
        32,
        "session secret must carry 128 bits as hex"
    );
    (ws, session)
}

/// True once the server closes `ws` (close frame, error or end of stream) within 2 s.
pub async fn closes(ws: &mut Ws) -> bool {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match ws.next().await {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return true,
                Some(Ok(_)) => continue,
            }
        }
    })
    .await
    .unwrap_or(false)
}

pub async fn expect_no_command(rx: &mut UnboundedReceiver<PlayerCommand>) {
    let res = tokio::time::timeout(Duration::from_millis(300), rx.recv()).await;
    assert!(
        res.is_err(),
        "no command expected, got {:?}",
        res.ok().flatten()
    );
}

pub async fn expect_command(rx: &mut UnboundedReceiver<PlayerCommand>) -> PlayerCommand {
    tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for command")
        .expect("bus closed")
}

pub async fn expect_request(rx: &mut UnboundedReceiver<HostRequest>) -> HostRequest {
    tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for a host request")
        .expect("request channel closed")
}

pub async fn expect_no_request(rx: &mut UnboundedReceiver<HostRequest>) {
    let res = tokio::time::timeout(Duration::from_millis(300), rx.recv()).await;
    assert!(
        res.is_err(),
        "no host request expected, got {:?}",
        res.ok().flatten()
    );
}
