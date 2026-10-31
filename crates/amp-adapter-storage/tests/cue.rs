//! CUE sheets (TASK-232): a single-file album becomes one track per song, each covering its
//! range of the file, with album and per-track tags.

#![forbid(unsafe_code)]

use amp_adapter_storage::{import_cue, scan_audio_files};
use amp_domain::AudioSource;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cue")
}

fn secs(s: f64) -> Duration {
    Duration::from_secs_f64(s)
}

#[test]
fn a_cue_sheet_splits_the_album_into_tracks() {
    let tracks = import_cue(&dir().join("album.cue")).expect("cue sheet imports");
    let titles: Vec<_> = tracks.iter().map(|t| t.metadata.title.as_str()).collect();
    assert_eq!(titles, vec!["First", "Second", "Café Third"]);
    let artists: Vec<_> = tracks.iter().map(|t| t.metadata.artist.as_str()).collect();
    assert_eq!(artists, vec!["Cue Band", "Guest Singer", "Cue Band"]);

    let flac = dir().join("album.flac").to_string_lossy().into_owned();
    for (i, t) in tracks.iter().enumerate() {
        let m = &t.metadata;
        assert_eq!(t.source, AudioSource::LocalFile { path: flac.clone() });
        assert_eq!(m.album.as_deref(), Some("Cue Album"));
        assert_eq!(m.album_artist.as_deref(), Some("Cue Band"));
        assert_eq!(m.genre.as_deref(), Some("Test Tones"));
        assert_eq!(m.year, Some(2024));
        assert_eq!(m.track_number, Some(i as u32 + 1));
        let seg = m.segment.expect("each song is a range of the file");
        assert_eq!(seg.start, secs(i as f64), "track {}", i + 1);
        let d = m.duration.expect("duration");
        assert!(
            (d.as_secs_f64() - 1.0).abs() < 0.02,
            "track {}: {d:?}",
            i + 1
        );
    }
    // INDEX 01 is where a song starts (INDEX 00 is the pregap); the next song's start ends it.
    assert_eq!(tracks[0].metadata.segment.unwrap().end, Some(secs(1.0)));
    assert_eq!(tracks[1].metadata.segment.unwrap().end, Some(secs(2.0)));
    assert_eq!(
        tracks[2].metadata.segment.unwrap().end,
        None,
        "the last runs to the end"
    );

    let ids: HashSet<_> = tracks.iter().map(|t| t.id).collect();
    assert_eq!(ids.len(), 3, "songs of one file still have their own ids");
    let again = import_cue(&dir().join("album.cue")).unwrap();
    assert_eq!(tracks[1].id, again[1].id, "ids are stable across imports");
}

#[test]
fn latin1_sheets_with_windows_line_endings_are_read() {
    let tracks = import_cue(&dir().join("latin1.cue")).expect("cue sheet imports");
    assert_eq!(tracks.len(), 3);
    assert_eq!(tracks[2].metadata.title, "Café Third");
}

#[test]
fn a_missing_audio_file_is_an_error_not_an_empty_album() {
    let tmp = std::env::temp_dir().join(format!("multiplay-cue-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tmp).unwrap();
    let cue = tmp.join("gone.cue");
    std::fs::write(
        &cue,
        "FILE \"gone.flac\" WAVE\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n",
    )
    .unwrap();
    assert!(import_cue(&cue).is_err());
    let _ = std::fs::remove_dir_all(tmp);
}

#[test]
fn scanning_a_folder_prefers_the_cue_sheet_over_its_single_file() {
    let report = scan_audio_files(&dir());
    assert!(
        report.cue_sheets.iter().any(|c| c.ends_with("album.cue")),
        "{:?}",
        report.cue_sheets
    );
    assert!(
        !report.files.iter().any(|f| f.ends_with("album.flac")),
        "the album file is played through its cue sheet: {:?}",
        report.files
    );
}
