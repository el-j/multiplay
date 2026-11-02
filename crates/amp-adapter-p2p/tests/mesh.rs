//! The private mesh (TASK-167, libp2p): two real nodes on this computer. Same mesh code: search
//! and play each other's shared music, and pass jam operations. Different code: no connection.

#![forbid(unsafe_code)]

use amp_adapter_p2p::{MeshCode, MeshConfig, MeshNode, SharedFile};
use amp_domain::jam_crdt::{JamItem, JamOperation, LamportTimestamp};
use amp_domain::{AudioSource, Track, TrackMetadata};
use std::io::Read;
use std::time::{Duration, Instant};

fn shared_song(dir: &std::path::Path) -> (SharedFile, Vec<u8>) {
    let bytes: Vec<u8> = (0..700_000u32)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
        .collect();
    let path = dir.join("Windowlicker.flac");
    std::fs::write(&path, &bytes).unwrap();
    let mut meta = TrackMetadata::new("Windowlicker", "Aphex Twin");
    meta.album = Some("Windowlicker".into());
    meta.duration = Some(Duration::from_secs(367));
    (
        SharedFile {
            path,
            metadata: meta,
        },
        bytes,
    )
}

fn config(code: &MeshCode, shared: Vec<SharedFile>, name: &str) -> MeshConfig {
    MeshConfig {
        code: code.clone(),
        name: name.into(),
        listen_port: 0,
        discover_on_lan: false,
        shared,
        identity: None,
    }
}

async fn connected(a: &MeshNode, b: &MeshNode) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if b.peers().await.contains(&a.peer_id()) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

#[tokio::test(flavor = "multi_thread")]
async fn the_same_code_searches_and_plays_shared_music() {
    let dir = tempfile::tempdir().unwrap();
    let (song, bytes) = shared_song(dir.path());
    let code = MeshCode::generate();
    let a = MeshNode::start(config(&code, vec![song], "A"))
        .await
        .unwrap();
    let b = MeshNode::start(config(&code, vec![], "B")).await.unwrap();
    b.dial(a.listen_addrs().await[0].clone()).await;
    assert!(connected(&a, &b).await, "B connects to A");

    let found = b.search("aphex", Duration::from_secs(3)).await;
    assert_eq!(found.len(), 1, "{found:?}");
    let track: &Track = &found[0];
    assert_eq!(track.metadata.title, "Windowlicker");
    let AudioSource::P2PShared { peer_id, file_hash } = &track.source else {
        panic!("{:?}", track.source)
    };
    assert_eq!(peer_id, &a.peer_id());

    let mut stream = b.fetch(peer_id, file_hash).await.expect("fetches");
    let mut got = Vec::new();
    stream.read_to_end(&mut got).unwrap();
    assert_eq!(got.len(), bytes.len());
    assert!(got == bytes, "the same bytes arrive");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_different_code_cannot_connect() {
    let dir = tempfile::tempdir().unwrap();
    let (song, _) = shared_song(dir.path());
    let a = MeshNode::start(config(&MeshCode::generate(), vec![song], "A"))
        .await
        .unwrap();
    let b = MeshNode::start(config(&MeshCode::generate(), vec![], "B"))
        .await
        .unwrap();
    b.dial(a.listen_addrs().await[0].clone()).await;
    assert!(!connected(&a, &b).await, "no connection without the code");
    assert!(b
        .search("aphex", Duration::from_millis(500))
        .await
        .is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn only_shared_files_can_be_fetched() {
    let dir = tempfile::tempdir().unwrap();
    let (song, _) = shared_song(dir.path());
    let code = MeshCode::generate();
    let a = MeshNode::start(config(&code, vec![song], "A"))
        .await
        .unwrap();
    let b = MeshNode::start(config(&code, vec![], "B")).await.unwrap();
    b.dial(a.listen_addrs().await[0].clone()).await;
    assert!(connected(&a, &b).await);
    assert!(b.fetch(&a.peer_id(), "not-a-shared-hash").await.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn jam_operations_reach_the_other_peer() {
    let code = MeshCode::generate();
    let a = MeshNode::start(config(&code, vec![], "A")).await.unwrap();
    let b = MeshNode::start(config(&code, vec![], "B")).await.unwrap();
    let mut events = b.jam_events();
    b.dial(a.listen_addrs().await[0].clone()).await;
    assert!(connected(&a, &b).await);
    let op = JamOperation::Insert {
        item: Box::new(JamItem::new(
            uuid::Uuid::new_v4(),
            Track::new(
                AudioSource::LocalFile {
                    path: "/x.flac".into(),
                },
                TrackMetadata::new("x", "y"),
            ),
            "A".into(),
            LamportTimestamp::new(1, uuid::Uuid::new_v4()),
        )),
        after_id: None,
    };
    // Gossip needs a moment to learn who subscribed; keep offering until it arrives.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        a.publish_jam(op.clone()).await;
        if let Ok(Ok(got)) = tokio::time::timeout(Duration::from_millis(300), events.recv()).await {
            assert_eq!(got, op);
            break;
        }
        assert!(Instant::now() < deadline, "the jam operation never arrived");
    }
}

#[test]
fn mesh_codes_roundtrip_and_the_discovery_id_does_not_reveal_the_key() {
    let code = MeshCode::generate();
    let text = code.to_string();
    assert_eq!(text.parse::<MeshCode>().unwrap(), code);
    assert_eq!(
        text.to_lowercase()
            .replace('-', " ")
            .parse::<MeshCode>()
            .unwrap(),
        code,
        "forgiving input"
    );
    assert!("not a code".parse::<MeshCode>().is_err());
    assert_ne!(MeshCode::generate(), code);
    let id = code.discovery_id();
    assert!(!text.contains(&id) && id.len() == 16);
}
