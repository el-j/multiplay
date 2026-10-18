//! Remote Daemon and Web Client Integration Test Harness.
//! Validates: embedded HTTP web serving, paired WebSocket control session,
//! versioned command dispatch, and server-to-client broadcast.
//! Negative security cases live in `amp-adapter-remote/tests/security.rs`.

#![forbid(unsafe_code)]

use amp_adapter_remote::protocol::{
    ClientMessage, RemoteAction, RemoteEventBroadcast, ServerMessage,
};
use amp_adapter_remote::{RemoteDaemon, RemoteDaemonConfig};
use amp_core_engine::events::PlayerCommand;
use amp_core_engine::EventBus;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

async fn next_server_message<S>(rx: &mut S) -> RemoteEventBroadcast
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    let frame = tokio::time::timeout(Duration::from_millis(1000), rx.next())
        .await
        .expect("timeout waiting for server message")
        .expect("stream ended")
        .expect("ws frame");
    let Message::Text(txt) = frame else {
        panic!("expected text frame");
    };
    let msg: ServerMessage = serde_json::from_str(&txt).expect("deserialize server message");
    assert_eq!(msg.v, 1);
    msg.event
}

#[tokio::test]
async fn test_remote_daemon_e2e_pipeline() {
    let (bus, mut cmd_rx) = EventBus::new(32);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral");
    let addr = listener.local_addr().expect("local addr");
    let daemon = Arc::new(RemoteDaemon::new(
        RemoteDaemonConfig::loopback(addr.port()),
        Arc::new(bus),
    ));
    let daemon_runner = Arc::clone(&daemon);
    tokio::spawn(async move {
        let _ = daemon_runner.run_with_listener(listener).await;
    });

    // 1. HTTP GET / returns the mobile web controller HTML
    let mut http_client = TcpStream::connect(addr).await.expect("http connect");
    http_client
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .expect("write http get");
    let mut http_resp = Vec::new();
    http_client
        .read_to_end(&mut http_resp)
        .await
        .expect("read http get");
    let resp_str = String::from_utf8_lossy(&http_resp);
    assert!(resp_str.starts_with("HTTP/1.1 200 OK"));
    assert!(resp_str.contains("Content-Type: text/html"));
    assert!(resp_str.contains("MULTIPLAY REMOTE"));

    // 2. HTTP GET /api/status returns JSON
    let mut status_client = TcpStream::connect(addr).await.expect("status connect");
    status_client
        .write_all(b"GET /api/status HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .expect("write status get");
    let mut status_resp = Vec::new();
    status_client
        .read_to_end(&mut status_resp)
        .await
        .expect("read status");
    let status_str = String::from_utf8_lossy(&status_resp);
    assert!(status_str.starts_with("HTTP/1.1 200 OK"));
    assert!(status_str.contains("\"service\":\"multiplay-remote\""));

    // 3. Pair with a host-issued token, then dispatch versioned commands
    let token = daemon.issue_pairing_token(Duration::from_secs(60));
    let ws_url = format!("ws://{addr}/ws?token={token}");
    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("ws connect failed");
    let (mut ws_tx, mut ws_rx) = ws_stream.split();
    assert!(matches!(
        next_server_message(&mut ws_rx).await,
        RemoteEventBroadcast::SessionGranted { .. }
    ));

    let vol = serde_json::to_string(&ClientMessage::new(RemoteAction::SetVolume(0.72))).unwrap();
    ws_tx
        .send(Message::Text(vol))
        .await
        .expect("send ws volume");
    let received_cmd = tokio::time::timeout(Duration::from_millis(500), cmd_rx.recv())
        .await
        .expect("timeout waiting for cmd")
        .expect("channel recv");
    assert!(matches!(received_cmd, PlayerCommand::SetVolume(v) if (v - 0.72).abs() < 0.01));

    let skip = serde_json::to_string(&ClientMessage::new(RemoteAction::VoteSkip)).unwrap();
    ws_tx.send(Message::Text(skip)).await.expect("send ws skip");
    let received_skip = tokio::time::timeout(Duration::from_millis(500), cmd_rx.recv())
        .await
        .expect("timeout waiting for skip")
        .expect("channel recv");
    assert!(matches!(received_skip, PlayerCommand::Next));

    // 4. Server-to-client broadcast
    daemon.broadcast(RemoteEventBroadcast::Status {
        current_track: None,
        is_playing: true,
        volume: 0.85,
        position_secs: 42,
        active_peers: 1,
    });
    match next_server_message(&mut ws_rx).await {
        RemoteEventBroadcast::Status {
            is_playing,
            volume,
            position_secs,
            ..
        } => {
            assert!(is_playing);
            assert!((volume - 0.85).abs() < 0.01);
            assert_eq!(position_secs, 42);
        }
        other => panic!("Expected Status broadcast, got {other:?}"),
    }
}
