//! Negative and positive security tests for the remote-control daemon (TASK-117/118/119).
//! Every test binds its own loopback listener on an ephemeral port; nothing touches the LAN.

#![forbid(unsafe_code)]

use amp_adapter_remote::{RemoteDaemon, RemoteDaemonConfig};
use amp_core_engine::events::PlayerCommand;
use amp_core_engine::EventBus;
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tokio_tungstenite::WebSocketStream;

type Ws = WebSocketStream<TcpStream>;

struct Harness {
    daemon: Arc<RemoteDaemon>,
    addr: SocketAddr,
    cmd_rx: UnboundedReceiver<PlayerCommand>,
}

async fn start_with(mut config: RemoteDaemonConfig) -> Harness {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    config.bind_addr = addr;
    let (bus, cmd_rx) = EventBus::new(64);
    let daemon = Arc::new(RemoteDaemon::new(config, Arc::new(bus)));
    let runner = Arc::clone(&daemon);
    tokio::spawn(async move {
        let _ = runner.run_with_listener(listener).await;
    });
    Harness {
        daemon,
        addr,
        cmd_rx,
    }
}

async fn start() -> Harness {
    start_with(RemoteDaemonConfig::loopback(0)).await
}

/// Opens a WebSocket to `/ws?{query}` with optional Origin and Host overrides.
#[allow(clippy::result_large_err)] // tungstenite's error type
async fn connect(
    addr: SocketAddr,
    query: &str,
    origin: Option<&str>,
    host: Option<&str>,
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
    let tcp = TcpStream::connect(addr).await.expect("tcp connect");
    let (ws, _) = tokio_tungstenite::client_async(req, tcp).await?;
    Ok(ws)
}

fn http_status(err: WsError) -> u16 {
    match err {
        WsError::Http(resp) => resp.status().as_u16(),
        other => panic!("expected HTTP rejection, got {other:?}"),
    }
}

async fn next_text(ws: &mut Ws) -> String {
    let msg = tokio::time::timeout(Duration::from_secs(2), ws.next())
        .await
        .expect("timeout waiting for server message")
        .expect("stream ended")
        .expect("ws error");
    match msg {
        Message::Text(t) => t,
        other => panic!("expected text frame, got {other:?}"),
    }
}

/// Pairs with a fresh token and returns the socket plus the granted session secret.
async fn pair(h: &Harness) -> (Ws, String) {
    let token = h.daemon.issue_pairing_token(Duration::from_secs(60));
    let mut ws = connect(h.addr, &format!("token={token}"), None, None)
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

async fn expect_no_command(rx: &mut UnboundedReceiver<PlayerCommand>) {
    let res = tokio::time::timeout(Duration::from_millis(300), rx.recv()).await;
    assert!(
        res.is_err(),
        "no command expected, got {:?}",
        res.ok().flatten()
    );
}

async fn expect_command(rx: &mut UnboundedReceiver<PlayerCommand>) -> PlayerCommand {
    tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for command")
        .expect("bus closed")
}

#[tokio::test]
async fn unauthenticated_upgrade_is_rejected_with_401() {
    let h = start().await;
    let err = connect(h.addr, "", None, None).await.unwrap_err();
    assert_eq!(http_status(err), 401);
}

#[tokio::test]
async fn unknown_token_is_rejected_with_401() {
    let h = start().await;
    let err = connect(h.addr, "token=DEADBEEFDEADBEEF", None, None)
        .await
        .unwrap_err();
    assert_eq!(http_status(err), 401);
}

#[tokio::test]
async fn expired_token_is_rejected() {
    let h = start().await;
    let token = h.daemon.issue_pairing_token(Duration::ZERO);
    tokio::time::sleep(Duration::from_millis(20)).await;
    let err = connect(h.addr, &format!("token={token}"), None, None)
        .await
        .unwrap_err();
    assert_eq!(http_status(err), 401);
}

#[tokio::test]
async fn paired_client_can_control_playback() {
    let mut h = start().await;
    let (mut ws, _) = pair(&h).await;
    ws.send(Message::Text(
        r#"{"v":1,"type":"SetVolume","payload":0.5}"#.into(),
    ))
    .await
    .unwrap();
    let cmd = expect_command(&mut h.cmd_rx).await;
    assert!(matches!(cmd, PlayerCommand::SetVolume(v) if (v - 0.5).abs() < 1e-6));

    ws.send(Message::Text(r#"{"v":1,"type":"Next"}"#.into()))
        .await
        .unwrap();
    assert!(matches!(
        expect_command(&mut h.cmd_rx).await,
        PlayerCommand::Next
    ));
    ws.send(Message::Text(r#"{"v":1,"type":"Previous"}"#.into()))
        .await
        .unwrap();
    assert!(matches!(
        expect_command(&mut h.cmd_rx).await,
        PlayerCommand::Previous
    ));
}

#[tokio::test]
async fn consumed_token_cannot_be_replayed() {
    let h = start().await;
    let token = h.daemon.issue_pairing_token(Duration::from_secs(60));
    let _first = connect(h.addr, &format!("token={token}"), None, None)
        .await
        .unwrap();
    let err = connect(h.addr, &format!("token={token}"), None, None)
        .await
        .unwrap_err();
    assert_eq!(http_status(err), 401);
}

#[tokio::test]
async fn session_secret_reconnects_until_revoked() {
    let h = start().await;
    let (ws, session) = pair(&h).await;
    drop(ws);
    let again = connect(h.addr, &format!("session={session}"), None, None).await;
    assert!(again.is_ok(), "a granted session must allow reconnecting");
    drop(again);

    h.daemon.revoke_all_sessions();
    let err = connect(h.addr, &format!("session={session}"), None, None)
        .await
        .unwrap_err();
    assert_eq!(http_status(err), 401);
}

#[tokio::test]
async fn cross_origin_upgrade_is_rejected_with_403() {
    let h = start().await;
    let token = h.daemon.issue_pairing_token(Duration::from_secs(60));
    let err = connect(
        h.addr,
        &format!("token={token}"),
        Some("http://evil.example"),
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(http_status(err), 403);
}

#[tokio::test]
async fn dns_rebinding_host_is_rejected_with_403() {
    let h = start().await;
    let token = h.daemon.issue_pairing_token(Duration::from_secs(60));
    let port = h.addr.port();
    let evil = format!("evil.example:{port}");
    let err = connect(
        h.addr,
        &format!("token={token}"),
        Some(&format!("http://{evil}")),
        Some(&evil),
    )
    .await
    .unwrap_err();
    assert_eq!(http_status(err), 403);
}

#[tokio::test]
async fn repeated_bad_attempts_are_throttled() {
    let h = start().await;
    let mut statuses = Vec::new();
    for _ in 0..20 {
        let err = connect(h.addr, "token=0000000000000000", None, None)
            .await
            .unwrap_err();
        statuses.push(http_status(err));
    }
    assert!(
        statuses.contains(&429),
        "expected throttling, got {statuses:?}"
    );

    // Even a valid token is refused while the address is backed off.
    let token = h.daemon.issue_pairing_token(Duration::from_secs(60));
    let err = connect(h.addr, &format!("token={token}"), None, None)
        .await
        .unwrap_err();
    assert_eq!(http_status(err), 429);
}

#[tokio::test]
async fn connection_cap_is_enforced() {
    let mut config = RemoteDaemonConfig::loopback(0);
    config.max_peers = 1;
    let h = start_with(config).await;
    let (_ws, _) = pair(&h).await;
    let token = h.daemon.issue_pairing_token(Duration::from_secs(60));
    let err = connect(h.addr, &format!("token={token}"), None, None)
        .await
        .unwrap_err();
    assert_eq!(http_status(err), 503);
}

#[tokio::test]
async fn oversized_frame_closes_the_socket() {
    let mut h = start().await;
    let (mut ws, _) = pair(&h).await;
    let huge = format!(
        r#"{{"v":1,"type":"Seek","payload":{}}}"#,
        "9".repeat(100 * 1024)
    );
    let _ = ws.send(Message::Text(huge)).await;
    let closed = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match ws.next().await {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return true,
                Some(Ok(_)) => continue,
            }
        }
    })
    .await
    .expect("socket should close after an oversized frame");
    assert!(closed);
    expect_no_command(&mut h.cmd_rx).await;
}

#[tokio::test]
async fn malformed_and_unversioned_messages_are_ignored() {
    let mut h = start().await;
    let (mut ws, _) = pair(&h).await;

    ws.send(Message::Text("{not json".into())).await.unwrap();
    assert!(next_text(&mut ws).await.contains("\"Error\""));
    ws.send(Message::Text(r#"{"type":"Play"}"#.into()))
        .await
        .unwrap();
    assert!(next_text(&mut ws).await.contains("\"Error\""));
    ws.send(Message::Text(r#"{"v":99,"type":"Play"}"#.into()))
        .await
        .unwrap();
    assert!(next_text(&mut ws)
        .await
        .contains("unsupported protocol version"));
    ws.send(Message::Text(
        r#"{"v":1,"type":"SetVolume","payload":"NaN"}"#.into(),
    ))
    .await
    .unwrap();
    assert!(next_text(&mut ws).await.contains("\"Error\""));
    expect_no_command(&mut h.cmd_rx).await;

    // The session survives garbage and still accepts valid commands.
    ws.send(Message::Text(r#"{"v":1,"type":"Play"}"#.into()))
        .await
        .unwrap();
    assert!(matches!(
        expect_command(&mut h.cmd_rx).await,
        PlayerCommand::TogglePause
    ));
}

#[tokio::test]
async fn remote_cannot_enqueue_paths_or_urls() {
    let mut h = start().await;
    let (mut ws, _) = pair(&h).await;
    for payload in [
        r#"{"v":1,"type":"EnqueueTrack","payload":{"source":{"LocalFile":{"path":"/etc/passwd"}}}}"#,
        r#"{"v":1,"type":"EnqueueTrack","payload":{"source":{"DirectHttp":{"url":"http://attacker/x.mp3"}}}}"#,
        r#"{"v":1,"type":"EnqueueById","payload":"/etc/passwd"}"#,
    ] {
        ws.send(Message::Text(payload.into())).await.unwrap();
        assert!(next_text(&mut ws).await.contains("\"Error\""));
    }
    expect_no_command(&mut h.cmd_rx).await;

    // Only ids are accepted; the host decides whether the id exists.
    let id = uuid::Uuid::new_v4();
    ws.send(Message::Text(format!(
        r#"{{"v":1,"type":"EnqueueById","payload":"{id}"}}"#
    )))
    .await
    .unwrap();
    assert!(matches!(
        expect_command(&mut h.cmd_rx).await,
        PlayerCommand::EnqueueExisting(t) if t.0 == id
    ));
}

#[tokio::test]
async fn http_responses_carry_security_headers() {
    let h = start().await;
    let mut tcp = TcpStream::connect(h.addr).await.unwrap();
    tcp.write_all(format!("GET / HTTP/1.1\r\nHost: {}\r\n\r\n", h.addr).as_bytes())
        .await
        .unwrap();
    let mut buf = Vec::new();
    tcp.read_to_end(&mut buf).await.unwrap();
    let resp = String::from_utf8_lossy(&buf).to_lowercase();
    assert!(resp.starts_with("http/1.1 200 ok"));
    assert!(resp.contains("x-content-type-options: nosniff"));
    assert!(resp.contains("referrer-policy: no-referrer"));
    assert!(resp.contains("content-security-policy:"));
    assert!(!resp.contains("access-control-allow-origin"));

    let mut tcp = TcpStream::connect(h.addr).await.unwrap();
    tcp.write_all(b"GET /secret HTTP/1.1\r\nHost: x\r\n\r\n")
        .await
        .unwrap();
    let mut buf = Vec::new();
    tcp.read_to_end(&mut buf).await.unwrap();
    assert!(String::from_utf8_lossy(&buf).starts_with("HTTP/1.1 404"));
}

#[tokio::test]
async fn shutdown_closes_listener() {
    let h = start().await;
    h.daemon.shutdown();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(TcpStream::connect(h.addr).await.is_err());
}
