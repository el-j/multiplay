//! Protocol v2 track requests (TASK-215): phones send a *reference*, the host moderates it.
//! Nothing a phone sends may make the host open, play or fetch anything without approval.

#![forbid(unsafe_code)]

mod common;

use amp_adapter_remote::{
    HostRequest, RequestStatus, SearchItem, TrackReference, ValidReference, MAX_PENDING_REQUESTS,
};
use amp_domain::TrackId;
use common::*;
use std::time::Duration;

const IPHONE: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) Safari/604.1";
const ANDROID: &str = "Mozilla/5.0 (Linux; Android 15; Pixel 9) Chrome/131.0 Mobile Safari/537.36";

fn request_json(kind: &str, value: &str) -> String {
    serde_json::json!({"v": 2, "type": "RequestTrack", "payload": {"kind": kind, "value": value}})
        .to_string()
}

async fn expect_track_request(h: &mut Harness) -> amp_adapter_remote::TrackRequest {
    match expect_request(&mut h.requests).await {
        HostRequest::Track(req) => req,
        other => panic!("expected a track request, got {other:?}"),
    }
}

#[tokio::test]
async fn youtube_link_becomes_a_pending_request_and_never_plays() {
    let mut h = start().await;
    let (mut ws, _) = pair_as(&h, Some(IPHONE)).await;
    send(
        &mut ws,
        &request_json("Link", "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42"),
    )
    .await;

    let req = expect_track_request(&mut h).await;
    assert_eq!(
        req.reference,
        ValidReference::YouTube {
            video_id: "dQw4w9WgXcQ".into()
        }
    );
    assert_eq!(req.device_name, "iPhone");
    assert!(!req.trusted);

    let update = next_of(&mut ws, "RequestUpdate").await;
    assert_eq!(update["payload"]["id"], req.id.0);
    assert_eq!(update["payload"]["status"], "Pending");
    expect_no_command(&mut h.cmd_rx).await;
}

#[tokio::test]
async fn every_allow_listed_link_form_is_normalized() {
    for link in [
        "https://youtu.be/dQw4w9WgXcQ",
        "https://m.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://music.youtube.com/watch?v=dQw4w9WgXcQ&list=x",
        "http://youtube.com/shorts/dQw4w9WgXcQ",
        "  https://www.youtube.com/watch?feature=share&v=dQw4w9WgXcQ  ",
    ] {
        assert_eq!(
            TrackReference::Link(link.into()).validate(),
            Ok(ValidReference::YouTube {
                video_id: "dQw4w9WgXcQ".into()
            }),
            "{link}"
        );
    }
}

#[tokio::test]
async fn unsafe_references_are_refused_before_reaching_the_host() {
    let mut h = start().await;
    let (mut ws, _) = pair(&h).await;
    let long_link = format!(
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ&x={}",
        "a".repeat(600)
    );
    let long_query = "q".repeat(300);
    let refused = [
        ("Link", "/etc/passwd"),
        ("Link", "C:\\Users\\me\\song.mp3"),
        ("Link", "file:///etc/passwd"),
        ("Link", "http://192.168.1.10/song.mp3"),
        ("Link", "http://10.0.0.1/watch?v=dQw4w9WgXcQ"),
        ("Link", "http://[::1]:9002/ws"),
        ("Link", "http://localhost/watch?v=dQw4w9WgXcQ"),
        ("Link", "https://evil.example/watch?v=dQw4w9WgXcQ"),
        (
            "Link",
            "https://youtube.com.evil.example/watch?v=dQw4w9WgXcQ",
        ),
        (
            "Link",
            "https://user:pw@www.youtube.com/watch?v=dQw4w9WgXcQ",
        ),
        ("Link", "https://www.youtube.com:8443/watch?v=dQw4w9WgXcQ"),
        ("Link", "javascript:alert(1)"),
        ("Link", "https://www.youtube.com/watch?v=../../etc"),
        ("Link", "https://www.youtube.com/"),
        ("Link", long_link.as_str()),
        ("Search", ""),
        ("Search", "   "),
        ("Search", "bad\u{0}query"),
        ("Search", long_query.as_str()),
    ];
    for (kind, value) in refused {
        send(&mut ws, &request_json(kind, value)).await;
        let msg = next_of(&mut ws, "RequestRefused").await;
        assert!(
            !msg["payload"]["reason"].as_str().unwrap().is_empty(),
            "{kind} {value:?}"
        );
    }
    expect_no_request(&mut h.requests).await;
    expect_no_command(&mut h.cmd_rx).await;
}

#[tokio::test]
async fn library_ids_and_search_queries_are_forwarded() {
    let mut h = start().await;
    let (mut ws, _) = pair(&h).await;
    let id = TrackId::new();
    send(&mut ws, &request_json("Library", &id.0.to_string())).await;
    assert_eq!(
        expect_track_request(&mut h).await.reference,
        ValidReference::Library(id)
    );
    send(
        &mut ws,
        &request_json("Search", "  Daft Punk  One More Time "),
    )
    .await;
    assert_eq!(
        expect_track_request(&mut h).await.reference,
        ValidReference::Search("Daft Punk One More Time".into())
    );
}

#[tokio::test]
async fn request_flood_is_capped_per_device() {
    let mut h = start().await;
    let (mut ws, _) = pair(&h).await;
    for _ in 0..20 {
        send(&mut ws, &request_json("Search", "flood")).await;
    }
    let mut ids = Vec::new();
    for _ in 0..MAX_PENDING_REQUESTS {
        ids.push(expect_track_request(&mut h).await.id);
    }
    expect_no_request(&mut h.requests).await;

    // Another device is not affected by the first one's flood.
    let (mut other, _) = pair(&h).await;
    send(&mut other, &request_json("Search", "fine")).await;
    expect_track_request(&mut h).await;

    // Answering a request frees a slot for the flooding device again.
    h.daemon
        .update_request(ids[0], RequestStatus::Rejected, None);
    tokio::time::sleep(Duration::from_millis(50)).await;
    send(&mut ws, &request_json("Search", "again")).await;
    expect_track_request(&mut h).await;
}

#[tokio::test]
async fn request_status_reaches_only_the_requesting_phone() {
    let mut h = start().await;
    let (mut phone, _) = pair(&h).await;
    let (mut bystander, _) = pair(&h).await;
    send(&mut phone, &request_json("Search", "song")).await;
    let req = expect_track_request(&mut h).await;
    assert_eq!(
        next_of(&mut phone, "RequestUpdate").await["payload"]["status"],
        "Pending"
    );

    for status in [RequestStatus::Accepted, RequestStatus::Playing] {
        h.daemon
            .update_request(req.id, status, Some("Song - Artist".into()));
        let msg = next_of(&mut phone, "RequestUpdate").await;
        assert_eq!(msg["payload"]["status"], format!("{status:?}"));
        assert_eq!(msg["payload"]["title"], "Song - Artist");
    }
    // The bystander only ever sees status broadcasts.
    h.daemon
        .broadcast(amp_adapter_remote::RemoteEventBroadcast::ActionAck {
            action: "marker".into(),
        });
    let first = next_text(&mut bystander).await;
    assert!(first.contains("marker"), "bystander saw {first}");
}

#[tokio::test]
async fn search_goes_through_the_host_and_answers_the_asker() {
    let mut h = start().await;
    let (mut ws, _) = pair(&h).await;
    send(
        &mut ws,
        r#"{"v":2,"type":"Search","payload":"one more time"}"#,
    )
    .await;
    let device = match expect_request(&mut h.requests).await {
        HostRequest::Search { device, query } => {
            assert_eq!(query, "one more time");
            device
        }
        other => panic!("expected a search, got {other:?}"),
    };
    h.daemon.send_search_results(
        &device,
        "one more time",
        vec![SearchItem {
            title: "One More Time".into(),
            artist: "Daft Punk".into(),
            duration_secs: Some(320),
            source: "youtube".into(),
            reference: TrackReference::Link("https://youtu.be/FGBhQbmPwH8".into()),
        }],
    );
    let msg = next_of(&mut ws, "SearchResults").await;
    assert_eq!(msg["payload"]["items"][0]["title"], "One More Time");
    assert_eq!(msg["payload"]["items"][0]["reference"]["kind"], "Link");

    send(&mut ws, r#"{"v":2,"type":"Search","payload":""}"#).await;
    next_of(&mut ws, "RequestRefused").await;
}

#[tokio::test]
async fn revoking_one_device_closes_only_its_session() {
    let h = start().await;
    let (mut iphone, iphone_secret) = pair_as(&h, Some(IPHONE)).await;
    let (mut android, android_secret) = pair_as(&h, Some(ANDROID)).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let devices = h.daemon.devices();
    assert_eq!(devices.len(), 2);
    let target = devices
        .iter()
        .find(|d| d.name == "iPhone")
        .expect("iPhone listed");
    assert!(target.connected);
    assert!(devices.iter().any(|d| d.name == "Android phone"));

    assert!(h.daemon.revoke_device(&target.id));
    assert!(
        closes(&mut iphone).await,
        "revoked device must be disconnected"
    );
    let err = connect(h.addr, &format!("session={iphone_secret}"), None, None)
        .await
        .unwrap_err();
    assert_eq!(http_status(err), 401);

    // The other phone stays connected and can still reconnect.
    h.daemon
        .broadcast(amp_adapter_remote::RemoteEventBroadcast::ActionAck {
            action: "still-here".into(),
        });
    assert!(next_text(&mut android).await.contains("still-here"));
    drop(android);
    assert!(
        connect(h.addr, &format!("session={android_secret}"), None, None)
            .await
            .is_ok()
    );
    assert_eq!(h.daemon.devices().len(), 1);
}

#[tokio::test]
async fn trusted_flag_travels_with_requests_and_survives_restart() {
    let dir = std::env::temp_dir().join(format!("multiplay-trust-{}", uuid::Uuid::new_v4()));
    let mut config = amp_adapter_remote::RemoteDaemonConfig::loopback(0);
    config.session_store = Some(dir.join("sessions.json"));
    let mut h = start_with(config.clone()).await;
    let (mut ws, _) = pair(&h).await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    let id = h.daemon.devices()[0].id.clone();
    assert!(h.daemon.set_trusted(&id, true));

    send(&mut ws, &request_json("Search", "trusted")).await;
    assert!(expect_track_request(&mut h).await.trusted);

    let reloaded = start_with(config).await;
    assert!(reloaded.daemon.devices()[0].trusted);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn revoke_all_disconnects_every_phone() {
    let h = start().await;
    let (mut a, _) = pair(&h).await;
    let (mut b, _) = pair(&h).await;
    h.daemon.revoke_all_sessions();
    assert!(closes(&mut a).await);
    assert!(closes(&mut b).await);
    assert!(h.daemon.devices().is_empty());
}
