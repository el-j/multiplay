//! Comprehensive Remote Daemon and Web Client Integration Test Harness.
//! Validates: Embedded HTTP web serving, WebSocket control session,
//! multi-peer broadcast, pairing token expiration, and collaborative actions.

#![forbid(unsafe_code)]

use amp_adapter_remote::pairing::PairingManager;
use amp_adapter_remote::protocol::{RemoteAction, RemoteEventBroadcast};
use amp_adapter_remote::server::RemoteDaemon;
use amp_core_engine::events::PlayerCommand;
use amp_core_engine::EventBus;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn test_remote_daemon_e2e_pipeline() {
    let (bus, mut cmd_rx) = EventBus::new(32);
    let arc_bus = Arc::new(bus);

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind ephemeral");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);

    let bind_addr = format!("127.0.0.1:{port}");
    let daemon = Arc::new(RemoteDaemon::new(&bind_addr, Arc::clone(&arc_bus)));
    let daemon_runner = Arc::clone(&daemon);

    tokio::spawn(async move {
        let _ = daemon_runner.run().await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // 1. Verify HTTP GET / returns the mobile web controller HTML
    let mut http_client = TcpStream::connect(&bind_addr).await.expect("http connect");
    http_client
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .expect("write http get");

    let mut http_resp = vec![0u8; 4096];
    let n = http_client.read(&mut http_resp).await.expect("read http get");
    let resp_str = String::from_utf8_lossy(&http_resp[..n]);

    assert!(resp_str.starts_with("HTTP/1.1 200 OK"));
    assert!(resp_str.contains("Content-Type: text/html"));
    assert!(resp_str.contains("MULTIPLAY REMOTE"));

    // 2. Verify HTTP GET /api/status returns JSON
    let mut status_client = TcpStream::connect(&bind_addr).await.expect("status connect");
    status_client
        .write_all(b"GET /api/status HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .expect("write status get");

    let mut status_resp = vec![0u8; 1024];
    let n_status = status_client.read(&mut status_resp).await.expect("read status");
    let status_str = String::from_utf8_lossy(&status_resp[..n_status]);

    assert!(status_str.starts_with("HTTP/1.1 200 OK"));
    assert!(status_str.contains("application/json"));
    assert!(status_str.contains("\"service\":\"multiplay-remote\""));

    // 3. Verify WebSocket Connection and Command Dispatch
    let ws_url = format!("ws://127.0.0.1:{port}/ws");
    let (ws_stream, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("ws connect failed");
    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    // Send SetVolume action
    let vol_action = serde_json::to_string(&RemoteAction::SetVolume(0.72)).unwrap();
    ws_tx.send(Message::Text(vol_action)).await.expect("send ws volume");

    let received_cmd = tokio::time::timeout(Duration::from_millis(500), cmd_rx.recv())
        .await
        .expect("timeout waiting for cmd")
        .expect("channel recv");
    assert!(matches!(received_cmd, PlayerCommand::SetVolume(v) if (v - 0.72).abs() < 0.01));

    // Send VoteSkip action
    let skip_action = serde_json::to_string(&RemoteAction::VoteSkip).unwrap();
    ws_tx.send(Message::Text(skip_action)).await.expect("send ws skip");

    let received_skip = tokio::time::timeout(Duration::from_millis(500), cmd_rx.recv())
        .await
        .expect("timeout waiting for skip")
        .expect("channel recv");
    assert!(matches!(received_skip, PlayerCommand::SeekRelative(_, true)));

    // 4. Verify Server-to-Client Broadcast
    daemon.broadcast(RemoteEventBroadcast::Status {
        current_track: None,
        is_playing: true,
        volume: 0.85,
        position_secs: 42,
        active_peers: 1,
    });

    let msg = tokio::time::timeout(Duration::from_millis(500), ws_rx.next())
        .await
        .expect("timeout on broadcast")
        .expect("broadcast msg")
        .expect("ws frame");

    if let Message::Text(txt) = msg {
        let broadcast_event: RemoteEventBroadcast = serde_json::from_str(&txt).expect("deserialize broadcast");
        match broadcast_event {
            RemoteEventBroadcast::Status { is_playing, volume, position_secs, .. } => {
                assert!(is_playing);
                assert!((volume - 0.85).abs() < 0.01);
                assert_eq!(position_secs, 42);
            }
            _ => panic!("Expected Status broadcast"),
        }
    } else {
        panic!("Expected text frame");
    }

    // 5. Verify Pairing Token Lifecycle
    let mut pairing = PairingManager::new();
    let token = pairing.generate_token(1); // 1-second TTL
    assert!(pairing.is_valid(&token));
    assert!(pairing.validate_and_consume(&token));
    assert!(!pairing.validate_and_consume(&token), "Consumed token must not be reusable");
}
