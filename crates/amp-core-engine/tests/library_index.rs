//! The library browser's model (TASK-172): artists, albums in track order, sortable and
//! filterable tracks, recently added and sources, fast enough for 100k tracks.

#![forbid(unsafe_code)]

use amp_core_engine::library_index::{LibraryFilter, LibraryIndex, TrackSort};
use amp_domain::{AudioSource, SourceKind, Track, TrackMetadata};
use std::time::{Duration, Instant};

fn track(
    title: &str,
    artist: &str,
    album: Option<&str>,
    disc_track: (u32, u32),
    added: u64,
) -> Track {
    let mut m = TrackMetadata::new(title, artist);
    m.album = album.map(str::to_string);
    m.disc_number = Some(disc_track.0);
    m.track_number = Some(disc_track.1);
    m.duration = Some(Duration::from_secs(60 + u64::from(disc_track.1)));
    let mut t = Track::new(
        AudioSource::LocalFile {
            path: format!("/m/{artist}/{title}.flac"),
        },
        m,
    );
    t.date_added_epoch = added;
    t
}

fn sample() -> LibraryIndex {
    let mut comp = track("Intro", "Various", Some("Mix Tape"), (1, 1), 50);
    comp.metadata.album_artist = Some("Various Artists".into());
    let mut comp2 = track("Outro", "Someone", Some("Mix Tape"), (1, 2), 51);
    comp2.metadata.album_artist = Some("Various Artists".into());
    let mut yt = Track::new(
        AudioSource::YouTube {
            video_id: "dQw4w9WgXcQ".into(),
        },
        TrackMetadata::new("Video", "Channel"),
    );
    yt.date_added_epoch = 99;
    LibraryIndex::build(vec![
        track(
            "Windowlicker",
            "Aphex Twin",
            Some("Windowlicker"),
            (1, 1),
            10,
        ),
        track("Flim", "Aphex Twin", Some("Come to Daddy"), (1, 3), 11),
        track(
            "Come to Daddy",
            "aphex twin",
            Some("Come to Daddy"),
            (1, 1),
            12,
        ),
        track(
            "Disc Two Opener",
            "Aphex Twin",
            Some("Come to Daddy"),
            (2, 1),
            13,
        ),
        track("Roygbiv", "Boards of Canada", None, (1, 1), 20),
        comp,
        comp2,
        yt,
    ])
}

#[test]
fn artists_are_grouped_case_insensitively_and_sorted() {
    let index = sample();
    let artists: Vec<_> = index
        .artists()
        .iter()
        .map(|a| (a.name.as_str(), a.tracks))
        .collect();
    assert_eq!(
        artists,
        [
            ("Aphex Twin", 4),
            ("Boards of Canada", 1),
            ("Channel", 1),
            ("Various Artists", 2)
        ]
    );
}

#[test]
fn albums_keep_disc_and_track_order_and_use_the_album_artist() {
    let index = sample();
    let daddy = index
        .albums()
        .iter()
        .find(|a| a.title == "Come to Daddy")
        .unwrap();
    assert_eq!(daddy.artist, "Aphex Twin");
    let titles: Vec<_> = daddy
        .tracks
        .iter()
        .map(|&i| index.tracks()[i].metadata.title.as_str())
        .collect();
    assert_eq!(titles, ["Come to Daddy", "Flim", "Disc Two Opener"]);
    let mix = index
        .albums()
        .iter()
        .find(|a| a.title == "Mix Tape")
        .unwrap();
    assert_eq!(
        (mix.artist.as_str(), mix.tracks.len()),
        ("Various Artists", 2)
    );
    assert!(
        index.albums().iter().all(|a| !a.title.is_empty()),
        "tracks without an album are not an album"
    );
}

#[test]
fn tracks_sort_and_filter() {
    let index = sample();
    let titles = |rows: Vec<usize>| -> Vec<String> {
        rows.iter()
            .map(|&i| index.tracks()[i].metadata.title.clone())
            .collect()
    };
    let by_title = index.tracks_sorted(TrackSort::Title, true, &LibraryFilter::default());
    assert_eq!(titles(by_title)[0], "Come to Daddy");
    let longest_first = index.tracks_sorted(TrackSort::Duration, false, &LibraryFilter::default());
    assert_eq!(index.tracks()[longest_first[0]].metadata.title, "Flim");

    let text = LibraryFilter {
        text: "DADDY".into(),
        ..Default::default()
    };
    assert_eq!(
        index.tracks_sorted(TrackSort::Title, true, &text).len(),
        3,
        "title or album match"
    );
    let artist = LibraryFilter {
        artist: Some("Boards of Canada".into()),
        ..Default::default()
    };
    assert_eq!(
        titles(index.tracks_sorted(TrackSort::Title, true, &artist)),
        ["Roygbiv"]
    );
}

#[test]
fn recently_added_and_sources() {
    let index = sample();
    let recent: Vec<_> = index
        .recently_added(3)
        .iter()
        .map(|&i| index.tracks()[i].metadata.title.as_str())
        .collect();
    assert_eq!(recent, ["Video", "Outro", "Intro"]);
    assert_eq!(
        index.sources(),
        [(SourceKind::LocalFile, 7), (SourceKind::YouTube, 1)]
    );
}

#[test]
fn a_hundred_thousand_tracks_index_and_sort_quickly() {
    let tracks: Vec<Track> = (0..100_000u32)
        .map(|i| {
            track(
                &format!("Song {i}"),
                &format!("Artist {}", i % 2_000),
                Some(&format!("Album {}", i % 8_000)),
                (1, i % 20),
                u64::from(i),
            )
        })
        .collect();
    let start = Instant::now();
    let index = LibraryIndex::build(tracks);
    let built = start.elapsed();
    let start = Instant::now();
    let rows = index.tracks_sorted(TrackSort::Artist, true, &LibraryFilter::default());
    let sorted = start.elapsed();
    assert_eq!(rows.len(), 100_000);
    assert_eq!(
        (index.artists().len(), index.albums().len()),
        (2_000, 8_000)
    );
    // Debug builds are ~10x slower; these bounds hold there too.
    assert!(built < Duration::from_secs(3), "build took {built:?}");
    assert!(sorted < Duration::from_secs(1), "sort took {sorted:?}");
}
