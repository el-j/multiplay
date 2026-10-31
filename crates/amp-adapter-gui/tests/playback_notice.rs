//! A track that cannot be played says why, instead of the player silently skipping it (search
//! results from a broken provider looked "unplayable" with no explanation).

#![forbid(unsafe_code)]

use amp_adapter_gui::MultiplayGuiApp;
use amp_domain::{AudioSource, Track, TrackMetadata};
use std::time::{Duration, Instant};

#[test]
fn a_failed_track_shows_why_it_did_not_play() {
    let dir = std::env::temp_dir().join(format!("multiplay-notice-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut app = MultiplayGuiApp::headless(&dir).expect("headless app");
    let track = Track::new(
        AudioSource::LocalFile {
            path: dir.join("missing.flac").to_string_lossy().into_owned(),
        },
        TrackMetadata::new("Ghost Song", "Nobody"),
    );
    app.play_search_result(track);

    let deadline = Instant::now() + Duration::from_secs(10);
    while app.playback_notice.is_none() && Instant::now() < deadline {
        app.poll_playback_events();
        std::thread::sleep(Duration::from_millis(20));
    }
    let notice = app
        .playback_notice
        .clone()
        .expect("a notice explains the failure");
    assert!(notice.contains("Ghost Song"), "{notice}");
    let _ = std::fs::remove_dir_all(dir);
}
