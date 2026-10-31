//! Bandcamp search parsing against a real `bcsearch_public_api` response: albums (releases) and
//! tracks become playable results, artist pages are skipped, and an API error is reported
//! instead of silently returning nothing.

#![forbid(unsafe_code)]

use amp_adapter_bandcamp::{parse_search_response, stream_url, TralbumData};
use amp_domain::AudioSource;
use amp_ports::ProviderError;

const FIXTURE: &str = include_str!("fixtures/autocomplete_elastic.json");

#[test]
fn releases_and_tracks_are_found() {
    let tracks = parse_search_response(FIXTURE).expect("valid response");
    let found: Vec<_> = tracks
        .iter()
        .map(|t| (t.metadata.title.as_str(), t.metadata.artist.as_str()))
        .collect();
    assert_eq!(
        found,
        vec![
            ("In Rainbows", "Radiohead"),
            ("Weird Fishes/ Arpeggi", "Radiohead")
        ],
        "the artist page is not a playable result"
    );
    assert_eq!(
        tracks[0].source,
        AudioSource::Bandcamp {
            stream_url: "https://radiohead.bandcamp.com/album/in-rainbows".into()
        }
    );
    assert_eq!(tracks[0].metadata.album.as_deref(), Some("In Rainbows"));
    assert!(tracks[0]
        .metadata
        .cover_art_uri
        .as_deref()
        .is_some_and(|u| u.starts_with("https://f4.bcbits.com/img/a")));
}

#[test]
fn an_api_error_is_reported_not_swallowed() {
    // What the retired fuzzysearch endpoint answers today.
    let err = parse_search_response(r#"{"error":true,"error_message":"bad function"}"#)
        .expect_err("error responses must not look like 'no results'");
    assert!(matches!(err, ProviderError::Unavailable(m) if m.contains("bad function")));
    assert!(parse_search_response("<html>").is_err());
}

#[test]
fn an_empty_result_list_is_fine() {
    assert!(parse_search_response(r#"{"auto":{"results":[]}}"#)
        .unwrap()
        .is_empty());
}

/// Live check against bandcamp.com: `cargo test -p amp-adapter-bandcamp --test search -- --ignored`
#[tokio::test]
#[ignore = "Live network canary"]
async fn live_search_finds_releases() {
    use amp_ports::ProviderSearchPort;
    let bc = amp_adapter_bandcamp::PureBandcampAdapter::new().unwrap();
    let tracks = bc
        .query("radiohead in rainbows")
        .await
        .expect("search works");
    assert!(
        tracks
            .iter()
            .any(|t| t.metadata.album.as_deref() == Some("In Rainbows")),
        "{tracks:#?}"
    );
}

/// Live check that a found result actually plays: resolve the page and decode a second of audio.
#[tokio::test]
#[ignore = "Live network canary"]
async fn live_track_resolves_and_decodes() {
    use amp_ports::StreamResolverPort;
    let bc = amp_adapter_bandcamp::PureBandcampAdapter::new().unwrap();
    let source = AudioSource::Bandcamp {
        stream_url: "https://jeffrosenstock.bandcamp.com/track/ambient-7".into(),
    };
    let stream = bc.resolve_stream(&source).await.expect("stream resolves");
    let mut decoder =
        amp_adapter_decoder::AudioStreamDecoder::open_stream_with_hint(stream, Some("mp3"), None)
            .expect("decodes as mp3");
    let mut buf = vec![0.0f32; decoder.sample_rate() as usize * 2];
    let n = decoder.read_interleaved_samples(&mut buf).expect("samples");
    assert!(n > 0);
}

fn tralbum(json: &str) -> TralbumData {
    serde_json::from_str(json).unwrap()
}

#[test]
fn the_first_streamable_track_is_played() {
    let data = tralbum(
        r#"{"trackinfo":[{"title":"a","file":null},{"title":"b","file":{"mp3-128":"https://t4.bcbits.com/stream/b"}}]}"#,
    );
    assert_eq!(stream_url(data).unwrap(), "https://t4.bcbits.com/stream/b");
}

#[test]
fn a_release_without_streaming_says_so() {
    // Some artists turn streaming off; their pages list tracks without audio files.
    let data = tralbum(r#"{"trackinfo":[{"title":"a","file":null}]}"#);
    let err = stream_url(data).unwrap_err();
    assert!(
        matches!(&err, ProviderError::Unavailable(m) if m.contains("streaming")),
        "{err:?}"
    );
}
