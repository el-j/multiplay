//! Stand-alone demo host for trying the phone web client without the desktop app.
//!
//! ```text
//! cargo run -p amp-adapter-remote --example phone_demo           # this computer only
//! cargo run -p amp-adapter-remote --example phone_demo -- --lan  # phones on your Wi-Fi
//! cargo run -p amp-adapter-remote --example phone_demo -- --uploads  # also accept uploads
//! ```
//!
//! Prints a pairing URL. Searches get two canned results; every request is accepted after a
//! second and reported as playing a second later. Nothing is played or fetched.

#![forbid(unsafe_code)]

use amp_adapter_remote::{
    HostRequest, RemoteDaemon, RemoteDaemonConfig, RequestStatus, SearchItem, TrackReference,
    DEFAULT_PORT,
};
use amp_core_engine::EventBus;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    let lan = std::env::args().any(|a| a == "--lan");
    let port = std::env::args()
        .skip_while(|a| a != "--port")
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let uploads = std::env::args().any(|a| a == "--uploads");
    let mut config = if lan {
        RemoteDaemonConfig::lan(port)
    } else {
        RemoteDaemonConfig::loopback(port)
    };
    if uploads {
        config.upload_dir = Some(std::env::temp_dir().join("multiplay-phone-demo-uploads"));
    }
    let (bus, _commands) = EventBus::new(64);
    let daemon = Arc::new(RemoteDaemon::new(config, Arc::new(bus)));
    let mut requests = daemon.take_request_receiver().expect("first take");
    if uploads {
        daemon.set_upload_probe(Arc::new(|path: &std::path::Path| {
            let mut dec =
                amp_adapter_decoder::AudioStreamDecoder::open_file(&path.to_string_lossy())?;
            let mut buf = vec![0.0f32; 4096];
            match dec.read_interleaved_samples(&mut buf)? {
                0 => Err("no audio".to_string()),
                _ => Ok(()),
            }
        }));
        daemon.set_uploads_enabled(true);
    }
    let token = daemon.issue_pairing_token(Duration::from_secs(600));
    println!("Pair: http://127.0.0.1:{port}/?token={token}");

    let host = Arc::clone(&daemon);
    tokio::spawn(async move {
        while let Some(request) = requests.recv().await {
            match request {
                HostRequest::Search { device, query } => {
                    println!("search from {}: {query}", device.0);
                    host.send_search_results(&device, &query, canned(&query));
                }
                HostRequest::Track(r) => {
                    println!(
                        "request {} from {}: {:?}",
                        r.id.0, r.device_name, r.reference
                    );
                    let host = Arc::clone(&host);
                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        let title = Some("Demo Track - Demo Artist".to_string());
                        host.update_request(r.id, RequestStatus::Accepted, title);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        host.update_request(r.id, RequestStatus::Playing, None);
                    });
                }
            }
        }
    });
    daemon.run().await
}

fn canned(query: &str) -> Vec<SearchItem> {
    vec![
        SearchItem {
            title: format!("{query} (library hit)"),
            artist: "Library Artist".into(),
            duration_secs: Some(201),
            source: "library".into(),
            reference: TrackReference::Library(amp_domain::TrackId::new()),
        },
        SearchItem {
            title: "<b>Never</b> Gonna Give You Up".into(),
            artist: "Rick Astley".into(),
            duration_secs: Some(213),
            source: "youtube".into(),
            reference: TrackReference::Link("https://www.youtube.com/watch?v=dQw4w9WgXcQ".into()),
        },
    ]
}
