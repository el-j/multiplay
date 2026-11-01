//! The library browser (TASK-172): everything imported is remembered across restarts, browsable
//! by artist, album and track, and its covers stay within a fixed number of textures.

#![forbid(unsafe_code)]

use amp_adapter_gui::library_window::CoverLru;
use amp_adapter_gui::MultiplayGuiApp;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../amp-adapter-decoder/tests/fixtures")
        .join(name)
}

fn wait_for_library(app: &mut MultiplayGuiApp) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.library_loading() {
        app.poll_library();
        assert!(Instant::now() < deadline, "library never loaded");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn library_titles(app: &MultiplayGuiApp) -> Vec<String> {
    let mut t: Vec<String> = app
        .library_index
        .as_ref()
        .map(|i| {
            i.tracks()
                .iter()
                .map(|t| t.metadata.title.clone())
                .collect()
        })
        .unwrap_or_default();
    t.sort();
    t
}

#[test]
fn imports_are_remembered_once_and_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut app = MultiplayGuiApp::headless(dir.path()).expect("headless app");
        app.load_library();
        wait_for_library(&mut app);
        assert!(library_titles(&app).is_empty());
        app.ingest_paths(&[fixture("cover.flac"), fixture("mp3_cbr_128.mp3")]);
        let added: Vec<u64> = app
            .library_index
            .as_ref()
            .unwrap()
            .tracks()
            .iter()
            .map(|t| t.date_added_epoch)
            .collect();
        assert_eq!(added.len(), 2);
        assert!(added.iter().all(|&d| d > 0), "dated on import: {added:?}");
        // The same files again: no duplicates.
        app.ingest_paths(&[fixture("cover.flac")]);
        assert_eq!(library_titles(&app).len(), 2);
    }
    let mut again = MultiplayGuiApp::headless(dir.path()).expect("restart");
    again.load_library();
    wait_for_library(&mut again);
    assert_eq!(
        library_titles(&again).len(),
        2,
        "remembered across restarts"
    );
}

#[test]
fn library_rows_go_to_the_playlist() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).expect("headless app");
    app.load_library();
    wait_for_library(&mut app);
    app.ingest_paths(&[fixture("wav_16.wav"), fixture("aac.m4a")]);
    app.playlist.clear();
    app.library_add_rows(&[1, 0]);
    assert_eq!(app.playlist.len(), 2);
    let first = app.library_index.as_ref().unwrap().tracks()[1]
        .source
        .clone();
    assert_eq!(app.playlist[0].source, first, "in the order given");
}

#[test]
fn cover_textures_are_capped_least_recently_used_first() {
    let mut lru: CoverLru<u32> = CoverLru::new(3);
    for k in ["a", "b", "c"] {
        lru.insert(k.to_string(), 0);
    }
    lru.touch("a"); // a is now the most recent
    lru.insert("d".to_string(), 0); // evicts b, the least recently used
    assert!(lru.get("a").is_some() && lru.get("c").is_some() && lru.get("d").is_some());
    assert!(lru.get("b").is_none());
    assert_eq!(lru.len(), 3);
}
