//! File dialogs never freeze the window (TASK-175): the picker runs on its own thread and its
//! result is added on a later frame; cancelling adds nothing and starts no scan.

#![forbid(unsafe_code)]

use amp_adapter_gui::MultiplayGuiApp;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn fixture(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../amp-adapter-decoder/tests/fixtures")
        .join(file)
}

fn wait_for_dialog(app: &mut MultiplayGuiApp) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.dialog_open() {
        app.poll_dialog();
        assert!(Instant::now() < deadline, "dialog result never arrived");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn the_picker_runs_off_the_ui_thread_and_its_files_are_added_later() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).expect("headless app");
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    app.start_dialog(move || {
        // Blocks like a real dialog until "the user" picks: the UI thread must not wait.
        rx.recv().unwrap();
        vec![fixture("wav_16.wav"), fixture("mp3_cbr_128.mp3")]
    });
    assert!(app.dialog_open());
    app.poll_dialog();
    assert!(
        app.playlist.is_empty(),
        "nothing yet, and poll did not block"
    );

    // A second dialog while one is open is ignored.
    app.start_dialog(|| vec![fixture("aac.m4a")]);

    tx.send(()).unwrap();
    wait_for_dialog(&mut app);
    let names: Vec<_> = app.playlist.iter().map(|t| t.source.clone()).collect();
    assert_eq!(names.len(), 2, "{names:?}");
}

#[test]
fn cancelling_adds_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).expect("headless app");
    app.start_dialog(Vec::new);
    wait_for_dialog(&mut app);
    assert!(app.playlist.is_empty());
    assert!(
        app.import_notice.is_none(),
        "no 'added 0 files' message either"
    );
}
