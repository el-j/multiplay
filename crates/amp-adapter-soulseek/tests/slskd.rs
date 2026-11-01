//! "Use Soulseek" through slskd (TASK-162): talking to a running slskd over its REST API, with a
//! fake slskd that answers the documented routes.

#![forbid(unsafe_code)]

use amp_adapter_soulseek::slskd::{SlskdClient, SlskdError};
use amp_adapter_soulseek::SoulseekProvider;
use amp_domain::AudioSource;
use amp_ports::{ProviderSearchPort, StreamResolverPort};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const KEY: &str = "0123456789abcdef-test-key";

/// A fake slskd: routes requests by method and path, checks the API key.
struct Fake {
    url: String,
    downloads: PathBuf,
    batches: Arc<Mutex<Vec<String>>>,
}

fn respond(conn: &mut std::net::TcpStream, status: &str, body: &str) {
    let _ = write!(
        conn,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

fn fake_slskd(transfer_state: &'static str) -> Fake {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let downloads = tempfile::tempdir().unwrap().keep();
    let batches = Arc::new(Mutex::new(Vec::new()));
    let (dl, seen) = (downloads.clone(), Arc::clone(&batches));
    let polls = Arc::new(AtomicUsize::new(0));
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(mut conn) = conn else { continue };
            let mut reader = BufReader::new(conn.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let (method, path) = {
                let mut it = line.split_whitespace();
                (
                    it.next().unwrap_or("").to_string(),
                    it.next().unwrap_or("").to_string(),
                )
            };
            let (mut key_ok, mut len) = (false, 0usize);
            loop {
                let mut h = String::new();
                reader.read_line(&mut h).unwrap();
                let h = h.trim_end();
                if h.is_empty() {
                    break;
                }
                let lower = h.to_ascii_lowercase();
                key_ok |= lower == format!("x-api-key: {}", KEY.to_ascii_lowercase());
                if let Some(v) = lower.strip_prefix("content-length: ") {
                    len = v.parse().unwrap_or(0);
                }
            }
            let mut body = vec![0u8; len];
            reader.read_exact(&mut body).unwrap();
            let body = String::from_utf8_lossy(&body).into_owned();
            if !key_ok {
                respond(&mut conn, "401 Unauthorized", "\"Unauthorized\"");
                continue;
            }
            let n = polls.fetch_add(1, Ordering::SeqCst);
            match (method.as_str(), path.as_str()) {
                ("GET", "/api/v0/application/version") => respond(&mut conn, "200 OK", "\"0.26.2\""),
                ("GET", "/api/v0/options") => respond(
                    &mut conn,
                    "200 OK",
                    &serde_json::json!({ "directories": { "downloads": dl, "incomplete": dl.join("incomplete") } }).to_string(),
                ),
                ("POST", "/api/v0/searches") => {
                    let id = serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"].as_str().unwrap().to_string();
                    respond(&mut conn, "200 OK", &serde_json::json!({ "id": id, "state": "InProgress" }).to_string());
                }
                ("GET", p) if p.starts_with("/api/v0/searches/") => {
                    let state = if n.is_multiple_of(2) { "InProgress" } else { "Completed, TimedOut" };
                    respond(&mut conn, "200 OK", &serde_json::json!({
                        "state": state,
                        "responses": [
                            { "username": "slowpeer", "hasFreeUploadSlot": false, "uploadSpeed": 1000, "queueLength": 50,
                              "files": [ { "filename": "@@x\\Music\\Boards of Canada\\Geogaddi\\01 - Ready Lets Go.mp3", "size": 1000, "bitRate": 320, "length": 60, "extension": "mp3" } ] },
                            { "username": "fastpeer", "hasFreeUploadSlot": true, "uploadSpeed": 900000, "queueLength": 0,
                              "files": [
                                  { "filename": "@@y\\share\\Boards of Canada - Music Has the Right\\02 - An Eagle in Your Mind.flac", "size": 2000, "length": 383, "extension": "flac" },
                                  { "filename": "@@y\\share\\Boards of Canada - Music Has the Right\\cover.jpg", "size": 30, "extension": "jpg" }
                              ],
                              "lockedFiles": [ { "filename": "@@y\\locked\\secret.flac", "size": 5 } ] }
                        ]
                    }).to_string());
                }
                ("POST", "/api/v0/transfers/downloads/batches") => {
                    let req: serde_json::Value = serde_json::from_str(&body).unwrap();
                    let dest = req["options"]["destination"].as_str().unwrap().to_string();
                    let file = req["files"][0]["filename"].as_str().unwrap().to_string();
                    seen.lock().unwrap().push(dest.clone());
                    let name = file.rsplit('\\').next().unwrap();
                    let dir = dl.join(&dest);
                    std::fs::create_dir_all(&dir).unwrap();
                    std::fs::write(dir.join(name), b"AUDIO-BYTES").unwrap();
                    respond(&mut conn, "201 Created", &serde_json::json!({ "batch": { "id": "b1" }, "failures": [] }).to_string());
                }
                ("GET", p) if p.starts_with("/api/v0/transfers/downloads/") => {
                    let state = if n.is_multiple_of(3) { "InProgress" } else { transfer_state };
                    respond(&mut conn, "200 OK", &serde_json::json!({
                        "username": "fastpeer",
                        "directories": [ { "directory": "@@y\\share\\Boards of Canada - Music Has the Right", "files": [
                            { "filename": "@@y\\share\\Boards of Canada - Music Has the Right\\02 - An Eagle in Your Mind.flac",
                              "state": state, "percentComplete": 100.0, "bytesTransferred": 2000, "size": 2000,
                              "exception": if state.contains("Errored") { "peer went away" } else { "" } }
                        ] } ]
                    }).to_string());
                }
                _ => respond(&mut conn, "404 Not Found", "\"no\""),
            }
        }
    });
    Fake {
        url,
        downloads,
        batches,
    }
}

#[tokio::test]
async fn a_running_slskd_is_found_and_a_wrong_key_is_refused() {
    let fake = fake_slskd("Completed, Succeeded");
    let ok = SlskdClient::new(&fake.url, KEY).unwrap();
    assert_eq!(ok.version().await.unwrap(), "0.26.2");
    let bad = SlskdClient::new(&fake.url, "wrong-key-wrong-key").unwrap();
    assert!(matches!(bad.version().await, Err(SlskdError::Unauthorized)));
    let nobody = SlskdClient::new("http://127.0.0.1:1", KEY).unwrap();
    assert!(matches!(
        nobody.version().await,
        Err(SlskdError::NotRunning(_))
    ));
}

#[tokio::test]
async fn search_gives_playable_unlocked_audio_best_peers_first() {
    let fake = fake_slskd("Completed, Succeeded");
    let provider = SoulseekProvider::new();
    provider.connect(SlskdClient::new(&fake.url, KEY).unwrap());
    let tracks = provider.query("boards of canada").await.unwrap();
    let titles: Vec<_> = tracks
        .iter()
        .map(|t| (t.metadata.artist.as_str(), t.metadata.title.as_str()))
        .collect();
    assert_eq!(
        titles,
        [
            ("Boards of Canada", "An Eagle in Your Mind"),
            ("Boards of Canada", "Ready Lets Go")
        ],
        "free upload slot first; no cover.jpg, no locked files"
    );
    assert_eq!(
        tracks[0].metadata.duration,
        Some(std::time::Duration::from_secs(383))
    );
    assert!(
        matches!(&tracks[0].source, AudioSource::Soulseek { username, .. } if username == "fastpeer")
    );
}

#[tokio::test]
async fn playing_downloads_into_its_own_folder_then_streams_the_file() {
    let fake = fake_slskd("Completed, Succeeded");
    let provider = SoulseekProvider::new();
    provider.connect(SlskdClient::new(&fake.url, KEY).unwrap());
    let source = AudioSource::Soulseek {
        username: "fastpeer".into(),
        filename:
            "@@y\\share\\Boards of Canada - Music Has the Right\\02 - An Eagle in Your Mind.flac"
                .into(),
    };
    assert!(provider.handles_source(&source));
    let mut stream = provider.resolve_stream(&source).await.expect("downloaded");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"AUDIO-BYTES");
    let dest = fake.batches.lock().unwrap()[0].clone();
    assert!(dest.starts_with("multiplay/"), "{dest}");
    assert!(fake.downloads.join(&dest).is_dir());
}

#[tokio::test]
async fn a_failed_download_says_why() {
    let fake = fake_slskd("Completed, Errored");
    let provider = SoulseekProvider::new();
    provider.connect(SlskdClient::new(&fake.url, KEY).unwrap());
    let source = AudioSource::Soulseek {
        username: "fastpeer".into(),
        filename:
            "@@y\\share\\Boards of Canada - Music Has the Right\\02 - An Eagle in Your Mind.flac"
                .into(),
    };
    let err = provider.resolve_stream(&source).await.err().expect("fails");
    assert!(err.to_string().contains("peer went away"), "{err}");
}

#[tokio::test]
async fn without_a_connection_soulseek_asks_to_be_set_up() {
    let provider = SoulseekProvider::new();
    let err = provider.query("x").await.expect_err("not connected");
    assert!(err.to_string().contains("Soulseek"), "{err}");
}
