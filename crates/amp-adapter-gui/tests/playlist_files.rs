//! Playlists and CUE sheets added by drop or dialog (TASK-232): M3U, PLS and XSPF with paths
//! relative to the playlist, CUE sheets as one row per song, and folders with a single-file
//! album played through their sheet. A CUE song plays only its part of the file.

#![forbid(unsafe_code)]

use amp_adapter_gui::{create_capture_engine, MultiplayGuiApp};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

struct Root(PathBuf);

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn cue_fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../amp-adapter-storage/tests/fixtures/cue")
}

/// A temp folder with the CUE album (album.flac + album.cue) and the app.
fn setup() -> (Root, MultiplayGuiApp, PathBuf) {
    let root = std::env::temp_dir().join(format!("multiplay-lists-{}", uuid::Uuid::new_v4()));
    let music = root.join("music");
    std::fs::create_dir_all(&music).unwrap();
    for f in ["album.flac", "album.cue"] {
        std::fs::copy(cue_fixtures().join(f), music.join(f)).unwrap();
    }
    let mut app = MultiplayGuiApp::headless(&root.join("state")).expect("headless app");
    app.playlist.clear();
    (Root(root), app, music)
}

fn titles(app: &MultiplayGuiApp) -> Vec<String> {
    app.playlist
        .iter()
        .map(|t| t.metadata.title.clone())
        .collect()
}

#[test]
fn a_folder_with_a_cue_sheet_adds_one_row_per_song() {
    let (_root, mut app, music) = setup();
    app.ingest_paths(&[music]);
    assert_eq!(titles(&app), vec!["First", "Second", "Café Third"]);
}

#[test]
fn a_dropped_cue_sheet_adds_its_songs() {
    let (_root, mut app, music) = setup();
    app.ingest_paths(&[music.join("album.cue")]);
    assert_eq!(app.playlist.len(), 3);
    assert!(app.playlist.iter().all(|t| t.metadata.segment.is_some()));
}

#[test]
fn m3u_pls_and_xspf_resolve_paths_next_to_the_playlist() {
    let (_root, mut app, music) = setup();
    std::fs::write(music.join("list.m3u8"), "#EXTM3U\nalbum.flac\n").unwrap();
    std::fs::write(
        music.join("list.pls"),
        "[playlist]\nFile1=album.flac\nTitle1=From PLS\nNumberOfEntries=1\nVersion=2\n",
    )
    .unwrap();
    std::fs::write(
        music.join("list.xspf"),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<playlist version="1" xmlns="http://xspf.org/ns/0/"><trackList>
<track><location>album.flac</location><title>From XSPF</title></track>
</trackList></playlist>"#,
    )
    .unwrap();
    let flac = music.join("album.flac").to_string_lossy().into_owned();
    for list in ["list.m3u8", "list.pls", "list.xspf"] {
        app.playlist.clear();
        app.ingest_paths(&[music.join(list)]);
        assert_eq!(app.playlist.len(), 1, "{list}");
        assert_eq!(
            app.playlist[0].source,
            amp_domain::AudioSource::LocalFile { path: flac.clone() },
            "{list}: relative to the playlist's folder"
        );
    }
}

#[test]
fn a_cue_song_plays_only_its_part_of_the_file() {
    let (_root, mut app, music) = setup();
    app.ingest_paths(&[music.join("album.cue")]);
    let second = app.playlist[1].clone();
    let (engine, output) = create_capture_engine().expect("capture engine");
    engine.play(second);
    let deadline = Instant::now() + Duration::from_secs(8);
    while !engine.track_finished.load(Ordering::SeqCst) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let mut len = output.len();
    loop {
        std::thread::sleep(Duration::from_millis(150));
        if output.len() == len {
            break;
        }
        len = output.len();
    }
    let samples = output.samples();
    let left: Vec<f32> = samples.iter().step_by(2).copied().collect();
    let first = left.iter().position(|s| s.abs() > 0.05).unwrap_or(0);
    let last = left.iter().rposition(|s| s.abs() > 0.05).unwrap_or(0);
    let span = (last - first) as f64 / 48_000.0;
    assert!((span - 1.0).abs() < 0.05, "played {span:.3} s, want 1 s");
    // The second song is the 550 Hz tone (first 440, third 660).
    let mid = &left[first + 12_000..first + 36_000];
    let ups = mid.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count();
    let hz = ups as f64 * 48_000.0 / mid.len() as f64;
    assert!((hz - 550.0).abs() < 15.0, "pitch {hz:.0} Hz");
}
