//! Host moderation of phone requests (TASK-216): a paired phone requests a reference, the host
//! previews it with its own resolvers (mocked here), and only an accepted request reaches the
//! playlist. Runs the real daemon on loopback with a blocking WebSocket "phone".

#![forbid(unsafe_code)]

use amp_adapter_gui::remote_lookup::LookupServices;
use amp_adapter_gui::remote_requests::{Moderation, Preview};
use amp_adapter_gui::MultiplayGuiApp;
use amp_domain::{AudioSource, DeckAssignment, RemoteControlConfig, Track, TrackMetadata};
use amp_ports::{async_trait, ProviderError, ProviderSearchPort, TrackLookupPort};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::{self, Message, WebSocket};

/// Stands in for YouTube: knows exactly one video, never touches the network.
struct FixtureYouTube;

#[async_trait]
impl TrackLookupPort for FixtureYouTube {
    fn can_lookup(&self, source: &AudioSource) -> bool {
        matches!(source, AudioSource::YouTube { .. })
    }
    async fn lookup(&self, source: &AudioSource) -> Result<TrackMetadata, ProviderError> {
        match source {
            AudioSource::YouTube { video_id } if video_id == "dQw4w9WgXcQ" => {
                let mut m = TrackMetadata::new("Never Gonna Give You Up", "Rick Astley");
                m.duration = Some(Duration::from_secs(213));
                Ok(m)
            }
            _ => Err(ProviderError::NotFound),
        }
    }
}

#[async_trait]
impl ProviderSearchPort for FixtureYouTube {
    fn provider_id(&self) -> &'static str {
        "youtube"
    }
    async fn query(&self, _query: &str) -> Result<Vec<Track>, ProviderError> {
        Ok(vec![Track::new(
            AudioSource::YouTube {
                video_id: "dQw4w9WgXcQ".into(),
            },
            TrackMetadata::new("Never Gonna Give You Up", "Rick Astley"),
        )])
    }
}

struct Host {
    app: MultiplayGuiApp,
    dir: PathBuf,
    /// Session secret of the phone paired last.
    secret: String,
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn host() -> Host {
    let dir = std::env::temp_dir().join(format!("multiplay-moderation-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut app = MultiplayGuiApp::headless(&dir).expect("headless app");
    app.lookup_services = Some(LookupServices {
        lookups: vec![Arc::new(FixtureYouTube)],
        providers: vec![Arc::new(FixtureYouTube)],
    });
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    app.config.remote = RemoteControlConfig {
        enabled: true,
        allow_lan: false,
        port,
    };
    app.remote.apply_config(&app.config.remote);
    assert!(app.remote.is_running, "{:?}", app.remote.last_error);
    Host {
        app,
        dir,
        secret: String::new(),
    }
}

type Phone = WebSocket<TcpStream>;

fn phone(host: &mut Host) -> Phone {
    let port = host.app.remote.port;
    let token = host.app.remote.current_token.clone();
    host.app.remote.refresh_token();
    let mut req = format!("ws://127.0.0.1:{port}/ws?token={token}")
        .into_client_request()
        .unwrap();
    let origin = HeaderValue::from_str(&format!("http://127.0.0.1:{port}")).unwrap();
    req.headers_mut().insert("Origin", origin);
    req.headers_mut().insert(
        "User-Agent",
        HeaderValue::from_static("Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)"),
    );
    let tcp = TcpStream::connect(("127.0.0.1", port)).unwrap();
    tcp.set_read_timeout(Some(Duration::from_millis(30)))
        .unwrap();
    let (mut ws, _) = tungstenite::client::client(req, tcp).expect("pairing");
    let granted = next_of(host, &mut ws, "SessionGranted");
    host.secret = granted["payload"]["session"].as_str().unwrap().to_string();
    ws
}

/// Next phone message of type `kind`, running the host's frame work while waiting.
fn next_of(host: &mut Host, ws: &mut Phone, kind: &str) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        host.app.handle_remote_requests();
        match ws.read() {
            Ok(Message::Text(t)) => {
                let v: serde_json::Value = serde_json::from_str(&t).unwrap();
                if v["type"] == kind {
                    return v;
                }
            }
            Ok(_) => {}
            // Read timeout: nothing arrived yet.
            Err(tungstenite::Error::Io(_)) => {}
            Err(e) => panic!("phone socket failed: {e}"),
        }
    }
    panic!("no {kind} message");
}

/// The first status update after "Pending".
fn answer(host: &mut Host, ws: &mut Phone) -> serde_json::Value {
    loop {
        let u = next_of(host, ws, "RequestUpdate");
        if u["payload"]["status"] != "Pending" {
            return u;
        }
    }
}

fn request(ws: &mut Phone, kind: &str, value: &str) {
    let msg = serde_json::json!({"v": 2, "type": "RequestTrack", "payload": {"kind": kind, "value": value}});
    ws.send(Message::Text(msg.to_string().into())).unwrap();
}

/// Runs the host's per-frame request handling until `done` holds (or fails after 3 s).
fn pump_until(host: &mut Host, mut done: impl FnMut(&MultiplayGuiApp) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        host.app.handle_remote_requests();
        if done(&host.app) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("host never reached the expected state");
}

fn ready(app: &MultiplayGuiApp) -> bool {
    app.remote_requests
        .rows
        .iter()
        .any(|r| matches!(r.preview, Preview::Ready(_)))
}

#[test]
fn moderation_accepted_youtube_request_is_resolved_by_the_host() {
    let mut host = host();
    let mut ws = phone(&mut host);
    request(&mut ws, "Link", "https://youtu.be/dQw4w9WgXcQ");
    pump_until(&mut host, ready);

    let row = host.app.remote_requests.rows[0].clone();
    assert_eq!(row.device_name, "iPhone");
    let Preview::Ready(preview) = &row.preview else {
        unreachable!()
    };
    assert_eq!(preview.metadata.title, "Never Gonna Give You Up");
    assert!(
        host.app.playlist.is_empty(),
        "nothing is added before approval"
    );

    assert!(host.app.moderate(row.id, Moderation::AddToEnd));
    let added = host.app.playlist.last().expect("track added");
    assert_eq!(
        added.source,
        AudioSource::YouTube {
            video_id: "dQw4w9WgXcQ".into()
        }
    );
    assert_eq!(added.metadata.artist, "Rick Astley");
    assert_eq!(
        host.app.requested_by.get(&added.id).map(String::as_str),
        Some("iPhone")
    );
    assert!(host.app.remote_requests.rows.is_empty());

    let update = answer(&mut host, &mut ws);
    assert_eq!(update["payload"]["status"], "Accepted");
    assert_eq!(
        update["payload"]["title"],
        "Never Gonna Give You Up - Rick Astley"
    );
}

#[test]
fn moderation_rejected_requests_never_touch_the_playlist() {
    let mut host = host();
    let mut ws = phone(&mut host);
    request(
        &mut ws,
        "Link",
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    );
    pump_until(&mut host, ready);
    let id = host.app.remote_requests.rows[0].id;
    assert!(host.app.moderate(id, Moderation::Reject));
    assert!(host.app.playlist.is_empty());
    assert!(
        !host.app.moderate(id, Moderation::AddToEnd),
        "answered once"
    );
    let update = answer(&mut host, &mut ws);
    assert_eq!(update["payload"]["status"], "Rejected");
}

#[test]
fn moderation_play_next_and_deck_assignment() {
    let mut host = host();
    for name in ["One", "Two", "Three"] {
        host.app.playlist.push(Track::new(
            AudioSource::LocalFile {
                path: format!("/music/{name}.flac"),
            },
            TrackMetadata::new(name, "Band"),
        ));
    }
    let mut ws = phone(&mut host);
    request(&mut ws, "Search", "band three");
    pump_until(&mut host, ready);
    let id = host.app.remote_requests.rows[0].id;
    assert!(host.app.moderate(id, Moderation::PlayNext));
    assert_eq!(host.app.playlist[1].metadata.title, "Three");
    assert_eq!(
        host.app.playlist.len(),
        4,
        "a copy is queued; the original stays"
    );

    request(&mut ws, "Link", "https://youtu.be/dQw4w9WgXcQ");
    pump_until(&mut host, ready);
    let id = host.app.remote_requests.rows[0].id;
    assert!(host.app.moderate(id, Moderation::Deck(1)));
    let added = host.app.playlist.last().unwrap();
    assert_eq!(
        host.app.deck_assign.get(&added.id),
        Some(&DeckAssignment::B)
    );
}

#[test]
fn moderation_trusted_devices_are_added_without_asking() {
    let mut host = host();
    let mut ws = phone(&mut host);
    let deadline = Instant::now() + Duration::from_secs(2);
    while host.app.remote.devices().is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    let device = host.app.remote.devices()[0].id.clone();
    assert!(host.app.remote.set_trusted(&device, true));

    request(&mut ws, "Link", "https://youtu.be/dQw4w9WgXcQ");
    pump_until(&mut host, |app| !app.playlist.is_empty());
    assert!(host.app.remote_requests.rows.is_empty());
    assert_eq!(
        host.app.playlist[0].metadata.title,
        "Never Gonna Give You Up"
    );
    assert!(
        !host
            .app
            .audio_driver
            .is_playing
            .load(std::sync::atomic::Ordering::Relaxed),
        "requests never auto-play"
    );
}

#[test]
fn moderation_unknown_library_ids_are_rejected_automatically() {
    let mut host = host();
    let mut ws = phone(&mut host);
    request(&mut ws, "Library", &uuid::Uuid::new_v4().to_string());
    let update = answer(&mut host, &mut ws);
    assert_eq!(update["payload"]["status"], "Rejected");
    assert!(host.app.remote_requests.rows.is_empty());
}

#[test]
fn moderation_phone_search_returns_library_and_provider_results() {
    let mut host = host();
    host.app.playlist.push(Track::new(
        AudioSource::LocalFile {
            path: "/music/never.flac".into(),
        },
        TrackMetadata::new("Never Gonna Give You Up (Live)", "Rick Astley"),
    ));
    host.app.config.streaming.enable_youtube = true;
    let mut ws = phone(&mut host);
    ws.send(Message::Text(
        r#"{"v":2,"type":"Search","payload":"never gonna"}"#.into(),
    ))
    .unwrap();
    let results = next_of(&mut host, &mut ws, "SearchResults");
    let items = results["payload"]["items"].as_array().unwrap();
    assert_eq!(items[0]["source"], "library");
    assert_eq!(items[0]["reference"]["kind"], "Library");
    assert!(items.iter().any(|i| i["source"] == "youtube"
        && i["reference"]["value"] == "https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
}

/// A 0.1 s, 8 kHz mono 16-bit WAV.
fn wav() -> Vec<u8> {
    let data: Vec<u8> = (0..800)
        .flat_map(|i| (((i as f32 * 0.2).sin() * 8000.0) as i16).to_le_bytes())
        .collect();
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    for v in [1u16, 1] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for v in [8000u32, 16000] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for v in [2u16, 16] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}

/// Posts `body` to the host's upload endpoint as the last paired phone; returns the status.
fn upload(host: &Host, name: &str, body: &[u8]) -> u16 {
    use std::io::{Read, Write};
    let port = host.app.remote.port;
    let mut tcp = TcpStream::connect(("127.0.0.1", port)).unwrap();
    let head = format!(
        "POST /api/upload HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\n\
         Authorization: Bearer {}\r\nX-Filename: {name}\r\nContent-Length: {}\r\n\r\n",
        host.secret,
        body.len()
    );
    tcp.write_all(head.as_bytes()).unwrap();
    tcp.write_all(body).unwrap();
    let mut resp = String::new();
    let _ = tcp.read_to_string(&mut resp);
    resp.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

#[test]
fn moderation_uploaded_file_joins_the_playlist_but_not_the_saved_session() {
    let mut host = host();
    let mut ws = phone(&mut host);
    assert_eq!(
        upload(&host, "song.wav", &wav()),
        403,
        "off until the host allows it"
    );

    host.app.remote.set_uploads_enabled(true);
    assert_eq!(
        next_of(&mut host, &mut ws, "Uploads")["payload"]["enabled"],
        true
    );
    assert_eq!(upload(&host, "My%20Song.wav", &wav()), 200);
    pump_until(&mut host, ready);
    let id = host.app.remote_requests.rows[0].id;
    assert!(host.app.moderate(id, Moderation::AddToEnd));
    let added = host.app.playlist.last().unwrap().clone();
    let AudioSource::LocalFile { path } = &added.source else {
        panic!("uploads play from a local file");
    };
    assert!(std::path::Path::new(path).starts_with(host.app.remote.upload_dir.as_ref().unwrap()));
    assert_eq!(added.metadata.title, "My Song");

    // The saved session leaves the temporary upload out; stopping remote deletes the file.
    host.app.save_session();
    let saved = std::fs::read_to_string(host.app.paths.session_file()).unwrap();
    assert!(!saved.contains("remote-uploads"), "{saved}");
    host.app.remote.stop_daemon();
    std::thread::sleep(Duration::from_millis(100));
    assert!(!std::path::Path::new(path).exists());
}
