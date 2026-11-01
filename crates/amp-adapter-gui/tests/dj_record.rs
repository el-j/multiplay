//! REC in the DJ console (TASK-224): records the master to a FLAC file beside the music (or in
//! the app's folder), and finishing adds the mix to the playlist.

use amp_adapter_gui::dj_record::{finish_recording, mixes_dir, toggle_recording};
use amp_adapter_gui::MultiplayGuiApp;
use std::time::{Duration, Instant};

#[test]
fn dj_record_saves_the_mix_and_adds_it_to_the_playlist() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).unwrap();
    // Never the real ~/Music in a test.
    let music = dir.path().join("Music");
    app.config.library.music_directories = vec![music.to_string_lossy().into_owned()];
    assert_eq!(mixes_dir(&app), music.join("multiplay mixes"));
    app.toggle_dj_mode(None);
    let mut dj = app.dj.take().expect("a DJ session");
    let started = toggle_recording(&mut app, &mut dj);
    assert_eq!(started, "Recording the mix");
    assert!(dj.control().recorder().is_recording());
    std::thread::sleep(Duration::from_millis(300));
    let saved = finish_recording(&mut app, &mut dj).expect("was recording");
    assert!(saved.starts_with("Saved a 0:00 mix to"), "{saved}");
    assert!(finish_recording(&mut app, &mut dj).is_none(), "only once");

    let mixes: Vec<_> = std::fs::read_dir(mixes_dir(&app))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    assert_eq!(mixes.len(), 1, "{mixes:?}");
    let mix = &mixes[0];
    assert!(mix.starts_with(&music), "beside the music");
    assert_eq!(mix.extension().unwrap(), "flac");
    assert!(mix
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("multiplay mix "));

    let deadline = Instant::now() + Duration::from_secs(10);
    while !app
        .playlist
        .iter()
        .any(|t| t.metadata.title.starts_with("multiplay mix "))
    {
        app.poll_import();
        assert!(Instant::now() < deadline, "the mix reaches the playlist");
        std::thread::sleep(Duration::from_millis(20));
    }
    app.dj = Some(dj);
}

#[test]
fn dj_record_without_a_music_folder_uses_the_app_folder() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).unwrap();
    app.config.library.music_directories.clear();
    assert!(mixes_dir(&app).starts_with(dir.path()));
}
