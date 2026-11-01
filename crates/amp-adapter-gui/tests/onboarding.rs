//! First-run setup (TASK-171): music folders (scanned in the background), output device, which
//! online sources to use, remote control off. Finishing or skipping never shows it again; it can
//! be run again from Preferences.

#![forbid(unsafe_code)]

use amp_adapter_gui::onboarding::{Onboarding, Step};
use amp_adapter_gui::MultiplayGuiApp;
use amp_domain::AppConfig;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../amp-adapter-decoder/tests/fixtures")
}

#[test]
fn the_steps_go_forward_and_back() {
    // The default settings name "~/Music": shown as the real folder.
    let defaults = Onboarding::new(&AppConfig::default(), None);
    assert!(defaults.folders[0].ends_with("Music") && !defaults.folders[0].starts_with("~"));

    let mut config = AppConfig::default();
    config.library.music_directories.clear();
    let mut ob = Onboarding::new(&config, Some(PathBuf::from("/Users/me/Music")));
    assert_eq!(ob.step, Step::Music);
    assert_eq!(
        ob.folders,
        [PathBuf::from("/Users/me/Music")],
        "~/Music is suggested"
    );
    ob.next();
    assert_eq!(ob.step, Step::Output);
    ob.next();
    assert_eq!(ob.step, Step::Sources);
    ob.back();
    assert_eq!(ob.step, Step::Output);
    ob.next();
    ob.next();
    assert_eq!(ob.step, Step::Remote);
    ob.next();
    assert_eq!(ob.step, Step::Finish);
    ob.next();
    assert_eq!(ob.step, Step::Finish, "nothing after the last step");
}

#[test]
fn choices_land_in_the_settings() {
    let mut config = AppConfig::default();
    let mut ob = Onboarding::new(&config, None);
    ob.folders = vec![PathBuf::from("/srv/music")];
    ob.device = Some("USB DAC".into());
    ob.youtube = false;
    ob.bandcamp = true;
    ob.apply(&mut config);
    assert_eq!(config.library.music_directories, ["/srv/music"]);
    assert_eq!(config.audio.output_device.as_deref(), Some("USB DAC"));
    assert!(!config.streaming.enable_youtube && config.streaming.enable_bandcamp);
    assert!(
        !config.remote.enabled,
        "remote control stays off unless chosen"
    );
    assert!(config.ui.first_run_done);
}

#[test]
fn finishing_scans_the_folders_in_the_background_and_shows_progress() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).expect("headless app");
    app.load_library();
    let mut ob = Onboarding::new(&app.config, None);
    ob.folders = vec![fixture_dir()];
    app.onboarding = Some(ob);
    app.finish_onboarding();
    assert!(app.onboarding.is_none());
    assert!(app.config.ui.first_run_done);
    assert!(app.importing(), "the scan runs in the background");

    let deadline = Instant::now() + Duration::from_secs(30);
    while app.importing() {
        app.poll_import();
        app.poll_library();
        assert!(Instant::now() < deadline, "scan never finished");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        app.playlist.len() >= 20,
        "the fixtures were added: {}",
        app.playlist.len()
    );
    let notice = app.import_notice.clone().unwrap_or_default();
    assert!(notice.contains("Added"), "{notice}");
}

#[test]
fn skipping_never_asks_again() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).expect("headless app");
    app.onboarding = Some(Onboarding::new(&app.config, None));
    app.skip_onboarding();
    assert!(app.onboarding.is_none() && app.config.ui.first_run_done);
    assert!(app.playlist.is_empty() && !app.importing());
}
