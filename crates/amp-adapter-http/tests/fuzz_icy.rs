//! A radio stream is whatever the server sends (TASK-185): arbitrary bytes through the ICY
//! demuxer and playlist parsers never panic, and the title history stays bounded.

#![forbid(unsafe_code)]

use amp_adapter_http::icy::{parse_icy_stream_title, IcyDemuxer, IcyMetadataTap};
use amp_adapter_http::playlist::{parse_playlist_urls, RadioPlaylistKind};
use proptest::prelude::*;

#[test]
fn a_chatty_station_does_not_grow_the_title_history_forever() {
    let tap = IcyMetadataTap::new();
    for i in 0..10_000 {
        tap.record_title(format!("Song {i}"));
    }
    assert!(
        tap.title_history().len() <= 100,
        "{}",
        tap.title_history().len()
    );
    assert_eq!(tap.latest_title().as_deref(), Some("Song 9999"));
}

/// Cases per property: the default in CI, many more in the nightly fuzz job (`FUZZ_CASES`).
fn cases(default: u32) -> u32 {
    std::env::var("FUZZ_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(cases(512)))]

    #[test]
    fn arbitrary_streams_never_panic(
        metaint in 0usize..64,
        chunks in proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..300), 0..20),
    ) {
        let mut demux = IcyDemuxer::new(metaint, IcyMetadataTap::new());
        let mut audio = Vec::new();
        let mut fed = 0;
        for c in &chunks {
            demux.push_bytes(c, &mut audio);
            fed += c.len();
        }
        prop_assert!(audio.len() <= fed);
    }

    #[test]
    fn arbitrary_metadata_blocks_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..4080)) {
        let _ = parse_icy_stream_title(&bytes);
    }

    #[test]
    fn arbitrary_playlist_bodies_never_panic(body in "\\PC{0,3000}") {
        for kind in [RadioPlaylistKind::Pls, RadioPlaylistKind::M3u, RadioPlaylistKind::Xspf] {
            let _ = parse_playlist_urls(kind, &body);
        }
    }
}
