//! A library entry is a source plus, for CUE tracks, the part of it (TASK-172): the songs of a
//! single-file album are separate entries, and storing a track again replaces it.

#![forbid(unsafe_code)]

use amp_adapter_storage::RedbStorageAdapter;
use amp_domain::{AudioSource, Track, TrackId, TrackMetadata, TrackSegment};
use std::time::Duration;

fn song(title: &str, start_secs: Option<u64>) -> Track {
    let mut m = TrackMetadata::new(title, "a");
    m.segment = start_secs.map(|s| TrackSegment {
        start: Duration::from_secs(s),
        end: None,
    });
    let source = AudioSource::LocalFile {
        path: "/music/album.flac".into(),
    };
    let mut t = Track::new(source, m);
    t.id = TrackId::from_key(&t.library_key());
    t
}

#[test]
fn cue_songs_of_one_file_are_separate_entries() {
    let dir = tempfile::tempdir().unwrap();
    let db = RedbStorageAdapter::new(dir.path().join("library.redb")).unwrap();
    db.store_tracks_batch_sync(&[song("One", Some(0)), song("Two", Some(240))])
        .unwrap();
    // Storing one again replaces it rather than adding a third.
    db.store_track_sync(&song("Two (fixed tag)", Some(240)))
        .unwrap();
    let mut titles: Vec<String> = db
        .fetch_all_sync()
        .unwrap()
        .into_iter()
        .map(|t| t.metadata.title)
        .collect();
    titles.sort();
    assert_eq!(titles, ["One", "Two (fixed tag)"]);
}

#[test]
fn whole_file_keys_are_unchanged() {
    let whole = song("Whole", None);
    assert_eq!(whole.library_key(), whole.source.canonical_uri());
    assert_eq!(whole.id, TrackId::from_source(&whole.source));
    assert_ne!(song("One", Some(0)).library_key(), whole.library_key());
}
