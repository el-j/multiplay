//! Negative and positive security tests for the remote-control daemon (TASK-117/118/119).
//! Track requests (protocol v2) are covered in `requests.rs`.

#![forbid(unsafe_code)]

mod common;

use amp_adapter_remote::RemoteDaemonConfig;
use amp_core_engine::events::PlayerCommand;
use common::*;
use futures_util::SinkExt;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;

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
    let _ = ws.send(Message::Text(huge.into())).await;
    let closed = closes(&mut ws).await;
    assert!(closed, "socket should close after an oversized frame");
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

    // Only ids are accepted, and even those only become a request the host must approve.
    let id = uuid::Uuid::new_v4();
    ws.send(Message::Text(
        format!(r#"{{"v":1,"type":"EnqueueById","payload":"{id}"}}"#).into(),
    ))
    .await
    .unwrap();
    match expect_request(&mut h.requests).await {
        amp_adapter_remote::HostRequest::Track(req) => assert_eq!(
            req.reference,
            amp_adapter_remote::ValidReference::Library(amp_domain::TrackId(id))
        ),
        other => panic!("expected a track request, got {other:?}"),
    }
    expect_no_command(&mut h.cmd_rx).await;
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
