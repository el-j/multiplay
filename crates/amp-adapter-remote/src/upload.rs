//! Opt-in phone audio upload (TASK-218): `POST /api/upload`.
//!
//! Off unless the host enables it for the session (403 otherwise). A paired device sends one
//! audio file with `Authorization: Bearer <session>`; it is size- and format-limited, stored
//! under a generated name in the upload folder, decode-probed, and only then offered to the
//! host as an ordinary request. The phone's file name is used for display, never as a path.
//! Every stored file is deleted when the daemon stops.

#![forbid(unsafe_code)]

use crate::auth::random_hex;
use crate::handshake;
use crate::protocol::RemoteEventBroadcast;
use crate::reference::ValidReference;
use crate::requests::{HostRequest, RequestStatus, TrackRequest};
use crate::server::{lock, Outbound, Shared};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Default upload size limit: 50 MB.
pub const DEFAULT_MAX_UPLOAD_BYTES: u64 = 50 * 1024 * 1024;
/// Whether a phone may upload files with extension `ext`: every playable format
/// (`amp_domain::formats`), and the decode probe still checks the content.
pub fn is_allowed_extension(ext: &str) -> bool {
    amp_domain::formats::is_playable_extension(ext)
}
const MAX_HEAD_BYTES: usize = 8 * 1024;
const MAX_NAME_CHARS: usize = 80;
/// Time allowed to receive a whole upload.
const BODY_TIMEOUT: Duration = Duration::from_secs(120);

/// Checks a stored upload; `Err` (with a reason) refuses it. Production uses the decoder.
pub type UploadProbe = Arc<dyn Fn(&Path) -> Result<(), String> + Send + Sync>;

/// Host switch and probe for uploads.
#[derive(Default)]
pub(crate) struct Uploads {
    pub(crate) enabled: AtomicBool,
    pub(crate) probe: Mutex<Option<UploadProbe>>,
}

type Refusal = (&'static str, &'static str);

/// Deletes every file this daemon stores in `dir` (generated names only; nothing else).
pub fn clear(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_generated_name(&name) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn is_generated_name(name: &str) -> bool {
    let Some((stem, ext)) = name.split_once('.') else {
        return false;
    };
    stem.len() == 32
        && stem.bytes().all(|b| b.is_ascii_hexdigit())
        && is_allowed_extension(ext.strip_suffix(".part").unwrap_or(ext))
}

/// Serves one upload request and closes the connection.
pub(crate) async fn handle(shared: &Shared, mut stream: TcpStream, peer: SocketAddr) {
    let response = match receive(shared, &mut stream, peer).await {
        Ok(body) => crate::http::build("200 OK", "application/json", &body),
        Err((status, why)) => crate::http::build(status, "text/plain; charset=utf-8", why),
    };
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
    // Discard what the phone is still sending so closing does not reset the connection
    // before it has read the answer.
    let mut sink = [0u8; 16 * 1024];
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(500), stream.read(&mut sink)).await {
            Ok(Ok(n)) if n > 0 => {}
            _ => break,
        }
    }
}

struct Head {
    headers: Vec<(String, String)>,
    /// Body bytes that arrived together with the headers.
    rest: Vec<u8>,
}

impl Head {
    fn get(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

async fn read_head(stream: &mut TcpStream, timeout: Duration) -> Result<Head, Refusal> {
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    let end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        if buf.len() > MAX_HEAD_BYTES {
            return Err(("431 Request Header Fields Too Large", "headers too large"));
        }
        match tokio::time::timeout(timeout, stream.read(&mut chunk)).await {
            Ok(Ok(n)) if n > 0 => buf.extend_from_slice(&chunk[..n]),
            _ => return Err(("400 Bad Request", "incomplete request")),
        }
    };
    let text = String::from_utf8_lossy(&buf[..end]).into_owned();
    let headers = text
        .lines()
        .skip(1)
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    Ok(Head {
        headers,
        rest: buf[end + 4..].to_vec(),
    })
}

async fn receive(
    shared: &Shared,
    stream: &mut TcpStream,
    peer: SocketAddr,
) -> Result<String, Refusal> {
    let head = read_head(stream, shared.config.handshake_timeout).await?;
    let forbidden = ("403 Forbidden", "uploads are turned off on the host");
    if !shared.uploads_available() {
        return Err(forbidden);
    }
    let (Some(dir), Some(probe)) = (
        shared.config.upload_dir.clone(),
        lock(&shared.uploads.probe).clone(),
    ) else {
        return Err(forbidden);
    };
    let host = head.get("host").unwrap_or("");
    if !handshake::host_is_acceptable(host)
        || !handshake::origin_matches_host(head.get("origin"), host)
    {
        return Err(("403 Forbidden", "wrong host or origin"));
    }
    let device = authenticate(shared, peer, head.get("authorization"))?;

    let length: u64 = head
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .ok_or(("411 Length Required", "content length required"))?;
    if length > shared.config.max_upload_bytes {
        return Err(("413 Payload Too Large", "the file is too large"));
    }
    let name = display_name(head.get("x-filename").unwrap_or(""));
    let ext = extension(&name).ok_or(("415 Unsupported Media Type", "not an audio file type"))?;

    let Some((device_name, trusted)) = lock(&shared.auth).sessions.describe(&device) else {
        return Err(("401 Unauthorized", "not paired"));
    };
    let id = lock(&shared.requests)
        .open(&device, Instant::now())
        .map_err(|_| ("429 Too Many Requests", "too many requests waiting"))?;

    let stem = random_hex(16);
    let part = dir.join(format!("{stem}.{ext}.part"));
    let stored = dir.join(format!("{stem}.{ext}"));
    let outcome = store(stream, &head.rest, length, &part)
        .await
        .and_then(|()| {
            probe(&part).map_err(|_| {
                (
                    "415 Unsupported Media Type",
                    "the file is not playable audio",
                )
            })?;
            std::fs::rename(&part, &stored)
                .map_err(|_| ("500 Internal Server Error", "could not store"))
        });
    if let Err(refusal) = outcome {
        let _ = std::fs::remove_file(&part);
        lock(&shared.requests).update(id, RequestStatus::Rejected);
        return Err(refusal);
    }

    let request = TrackRequest {
        id,
        device: device.clone(),
        device_name,
        trusted,
        reference: ValidReference::Upload {
            path: stored.clone(),
            name,
        },
    };
    if shared.host_tx.send(HostRequest::Track(request)).is_err() {
        let _ = std::fs::remove_file(&stored);
        return Err(("503 Service Unavailable", "the host is not taking requests"));
    }
    let pending = RemoteEventBroadcast::RequestUpdate {
        id,
        status: RequestStatus::Pending,
        title: None,
    };
    let _ = shared.outbound.send(Outbound::To(device, pending));
    Ok(format!("{{\"id\":{}}}", id.0))
}

fn authenticate(
    shared: &Shared,
    peer: SocketAddr,
    authorization: Option<&str>,
) -> Result<crate::devices::DeviceId, Refusal> {
    let now = Instant::now();
    let mut auth = lock(&shared.auth);
    if auth.throttle.check(peer.ip(), now).is_err() {
        return Err(("429 Too Many Requests", "too many failed attempts"));
    }
    let secret = authorization
        .and_then(|a| a.strip_prefix("Bearer "))
        .unwrap_or("");
    match auth.sessions.verify(secret.trim()) {
        Some(device) => {
            auth.throttle.record_success(peer.ip());
            Ok(device)
        }
        None => {
            auth.throttle.record_failure(peer.ip(), now);
            Err(("401 Unauthorized", "not paired"))
        }
    }
}

/// Writes exactly `length` body bytes (starting with `already`) to `path`.
async fn store(
    stream: &mut TcpStream,
    already: &[u8],
    length: u64,
    path: &Path,
) -> Result<(), Refusal> {
    let io_error = ("500 Internal Server Error", "could not store");
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|_| io_error)?;
    }
    let mut file = tokio::fs::File::create(path).await.map_err(|_| io_error)?;
    let first = &already[..already.len().min(length as usize)];
    file.write_all(first).await.map_err(|_| io_error)?;
    let mut remaining = length - first.len() as u64;
    let mut buf = vec![0u8; 64 * 1024];
    let deadline = Instant::now() + BODY_TIMEOUT;
    while remaining > 0 {
        let left = deadline.saturating_duration_since(Instant::now());
        let want = buf.len().min(remaining as usize);
        let n = match tokio::time::timeout(left, stream.read(&mut buf[..want])).await {
            Ok(Ok(n)) if n > 0 => n,
            _ => return Err(("400 Bad Request", "the upload was cut short")),
        };
        file.write_all(&buf[..n]).await.map_err(|_| io_error)?;
        remaining -= n as u64;
    }
    file.flush().await.map_err(|_| io_error)
}

/// The phone's file name, percent-decoded and reduced to a short, single-line base name.
fn display_name(raw: &str) -> String {
    let decoded = percent_decode(raw);
    let base = decoded.rsplit(['/', '\\']).next().unwrap_or("");
    let clean: String = base
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME_CHARS)
        .collect();
    clean.trim().trim_start_matches('.').to_string()
}

fn extension(name: &str) -> Option<&'static str> {
    let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
    amp_domain::formats::playable_extensions().find(|e| *e == ext)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        match (
            bytes[i],
            bytes.get(i + 1).copied().and_then(hex),
            bytes.get(i + 2).copied().and_then(hex),
        ) {
            (b'%', Some(h), Some(l)) => {
                out.push((h * 16 + l) as u8);
                i += 3;
            }
            (b, _, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_reduced_to_safe_display_names() {
        assert_eq!(display_name("..%2F..%2Fevil%0A.wav"), "evil.wav");
        assert_eq!(display_name("C:\\\\x\\\\song.mp3"), "song.mp3");
        assert_eq!(display_name("..."), "");
        assert_eq!(extension("Song.FLAC"), Some("flac"));
        assert_eq!(extension("notes.txt"), None);
        assert_eq!(extension("noext"), None);
    }

    #[test]
    fn only_generated_names_are_cleared() {
        let stem = "0123456789abcdef0123456789abcdef";
        assert!(is_generated_name(&format!("{stem}.wav")));
        assert!(is_generated_name(&format!("{stem}.mp3.part")));
        assert!(!is_generated_name("holiday.wav"));
        assert!(!is_generated_name(&format!("{stem}.txt")));
    }
}
