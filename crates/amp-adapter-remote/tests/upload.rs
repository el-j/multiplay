//! Opt-in phone audio upload (TASK-218): off by default (403), only for paired devices, size
//! and format limited, decode-probed before the host sees it, stored under generated names in
//! the upload folder, and removed again when the daemon stops.

#![forbid(unsafe_code)]

mod common;

use amp_adapter_remote::{HostRequest, RemoteDaemonConfig, ValidReference};
use common::*;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

struct Dir(PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Real decode probe: the file must open and yield samples with the fuzzed decoder.
fn decoder_probe(path: &Path) -> Result<(), String> {
    let mut dec = amp_adapter_decoder::AudioStreamDecoder::open_file(&path.to_string_lossy())
        .map_err(|e| e.to_string())?;
    let mut buf = vec![0.0f32; 4096];
    match dec
        .read_interleaved_samples(&mut buf)
        .map_err(|e| e.to_string())?
    {
        0 => Err("no audio".into()),
        _ => Ok(()),
    }
}

/// A 0.1 s, 8 kHz mono 16-bit WAV.
fn wav() -> Vec<u8> {
    let samples: Vec<i16> = (0..800)
        .map(|i| ((i as f32 * 0.2).sin() * 8000.0) as i16)
        .collect();
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&8000u32.to_le_bytes());
    out.extend_from_slice(&16000u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

async fn start_uploads(max_bytes: u64) -> (Harness, Dir) {
    let dir = Dir(std::env::temp_dir().join(format!("multiplay-upload-{}", uuid::Uuid::new_v4())));
    let mut config = RemoteDaemonConfig::loopback(0);
    config.upload_dir = Some(dir.0.clone());
    config.max_upload_bytes = max_bytes;
    let h = start_with(config).await;
    h.daemon.set_upload_probe(Arc::new(decoder_probe));
    (h, dir)
}

/// Raw POST /api/upload; returns the status code and the response body.
async fn upload(
    addr: SocketAddr,
    session: Option<&str>,
    filename: &str,
    body: &[u8],
    content_length: Option<usize>,
) -> (u16, String) {
    let mut tcp = TcpStream::connect(addr).await.unwrap();
    let mut head = format!(
        "POST /api/upload HTTP/1.1\r\nHost: {addr}\r\nOrigin: http://{addr}\r\nX-Filename: {filename}\r\n"
    );
    if let Some(s) = session {
        head.push_str(&format!("Authorization: Bearer {s}\r\n"));
    }
    head.push_str(&format!(
        "Content-Length: {}\r\n\r\n",
        content_length.unwrap_or(body.len())
    ));
    tcp.write_all(head.as_bytes()).await.unwrap();
    let _ = tcp.write_all(body).await;
    let mut buf = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), tcp.read_to_end(&mut buf)).await;
    let text = String::from_utf8_lossy(&buf).into_owned();
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

fn files_in(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|it| it.flatten().map(|e| e.path()).collect())
        .unwrap_or_default()
}

#[tokio::test]
async fn upload_is_forbidden_until_the_host_enables_it() {
    let (mut h, dir) = start_uploads(1 << 20).await;
    let (_ws, session) = pair(&h).await;
    let (status, _) = upload(h.addr, Some(&session), "song.wav", &wav(), None).await;
    assert_eq!(status, 403);
    assert!(files_in(&dir.0).is_empty());
    expect_no_request(&mut h.requests).await;

    // A daemon without an upload folder never accepts uploads, even when asked to.
    let plain = start().await;
    plain.daemon.set_uploads_enabled(true);
    let (_ws2, s2) = pair(&plain).await;
    assert_eq!(
        upload(plain.addr, Some(&s2), "song.wav", &wav(), None)
            .await
            .0,
        403
    );
}

#[tokio::test]
async fn upload_needs_a_paired_device() {
    let (h, dir) = start_uploads(1 << 20).await;
    h.daemon.set_uploads_enabled(true);
    assert_eq!(upload(h.addr, None, "song.wav", &wav(), None).await.0, 401);
    let fake = "0".repeat(32);
    assert_eq!(
        upload(h.addr, Some(&fake), "song.wav", &wav(), None)
            .await
            .0,
        401
    );
    assert!(files_in(&dir.0).is_empty());
}

#[tokio::test]
async fn upload_rejects_oversize_non_audio_and_malformed_files() {
    let (mut h, dir) = start_uploads(4096).await;
    h.daemon.set_uploads_enabled(true);
    let (_ws, session) = pair(&h).await;

    let big = vec![0u8; 8192];
    assert_eq!(
        upload(h.addr, Some(&session), "big.wav", &big, None)
            .await
            .0,
        413
    );
    assert_eq!(
        upload(h.addr, Some(&session), "a.wav", b"", Some(usize::MAX))
            .await
            .0,
        413,
        "absurd lengths are refused before reading"
    );
    assert_eq!(
        upload(h.addr, Some(&session), "notes.txt", b"hello", None)
            .await
            .0,
        415,
        "unknown extensions are refused"
    );
    assert_eq!(
        upload(
            h.addr,
            Some(&session),
            "fake.mp3",
            b"this is not audio at all",
            None
        )
        .await
        .0,
        415,
        "files that do not decode are refused"
    );
    let mut broken = wav();
    broken.truncate(30);
    assert_eq!(
        upload(h.addr, Some(&session), "cut.wav", &broken, None)
            .await
            .0,
        415
    );
    assert!(files_in(&dir.0).is_empty(), "refused files are deleted");
    expect_no_request(&mut h.requests).await;
}

#[tokio::test]
async fn upload_becomes_a_request_with_a_generated_file_name() {
    let (mut h, dir) = start_uploads(1 << 20).await;
    h.daemon.set_uploads_enabled(true);
    let (mut ws, session) = pair(&h).await;
    assert_eq!(
        next_of(&mut ws, "Uploads").await["payload"]["enabled"],
        true
    );

    let (status, body) = upload(
        h.addr,
        Some(&session),
        "..%2F..%2Fevil%0A.wav",
        &wav(),
        None,
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let req = match expect_request(&mut h.requests).await {
        HostRequest::Track(r) => r,
        other => panic!("expected a track request, got {other:?}"),
    };
    let ValidReference::Upload { path, name } = &req.reference else {
        panic!("expected an upload, got {:?}", req.reference);
    };
    assert_eq!(
        path.parent(),
        Some(dir.0.as_path()),
        "stored inside the upload folder"
    );
    let stored = path.file_name().unwrap().to_string_lossy();
    assert!(
        !stored.contains("evil") && stored.ends_with(".wav"),
        "{stored}"
    );
    assert!(
        !name.contains('/') && !name.contains('\n') && name.ends_with(".wav"),
        "{name:?}"
    );
    assert!(body.contains(&format!("\"id\":{}", req.id.0)));
    assert_eq!(
        next_of(&mut ws, "RequestUpdate").await["payload"]["status"],
        "Pending"
    );

    // Turning uploads off is announced, and stopping the daemon removes what was stored.
    h.daemon.set_uploads_enabled(false);
    assert_eq!(
        next_of(&mut ws, "Uploads").await["payload"]["enabled"],
        false
    );
    h.daemon.shutdown();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        files_in(&dir.0).is_empty(),
        "uploads are cleaned up on exit"
    );
}
