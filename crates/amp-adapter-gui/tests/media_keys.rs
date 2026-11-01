//! Media keys and Now Playing (TASK-173): the app tells the system what plays and does what the
//! system's keys and panels ask. Uses a stand-in for the OS session.

#![forbid(unsafe_code)]

use amp_adapter_gui::MultiplayGuiApp;
use amp_domain::{AudioSource, Track, TrackMetadata};
use amp_ports::{MediaControlsPort, MediaKeyEvent, MediaPlayback, NowPlaying};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Seen {
    now: Option<NowPlaying>,
    playback: Option<MediaPlayback>,
    queued: Vec<MediaKeyEvent>,
}

struct FakeOs(Arc<Mutex<Seen>>);

impl MediaControlsPort for FakeOs {
    fn set_now_playing(&mut self, now: &NowPlaying) {
        self.0.lock().unwrap().now = Some(now.clone());
    }
    fn set_playback(&mut self, playback: MediaPlayback) {
        self.0.lock().unwrap().playback = Some(playback);
    }
    fn take_events(&mut self) -> Vec<MediaKeyEvent> {
        std::mem::take(&mut self.0.lock().unwrap().queued)
    }
}

fn fixture(file: &str, title: &str) -> Track {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../amp-adapter-decoder/tests/fixtures")
        .join(file);
    let mut meta = TrackMetadata::new(title, "multiplay");
    meta.album = Some("Golden Fixtures".into());
    Track::new(
        AudioSource::LocalFile {
            path: path.to_string_lossy().into_owned(),
        },
        meta,
    )
}

fn sync_until(app: &mut MultiplayGuiApp, what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.sync_media_controls();
        if done() {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn now_playing_follows_the_engine_and_keys_drive_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).expect("headless app");
    let seen = Arc::new(Mutex::new(Seen::default()));
    app.attach_media_controls(Box::new(FakeOs(Arc::clone(&seen))));
    app.playlist = vec![
        fixture("wav_16.wav", "First"),
        fixture("mp3_cbr_128.mp3", "Second"),
    ];
    app.play_index(0);

    sync_until(&mut app, "Now Playing", || {
        seen.lock()
            .unwrap()
            .now
            .as_ref()
            .is_some_and(|n| n.title == "First")
    });
    let now = seen.lock().unwrap().now.clone().unwrap();
    assert_eq!(
        (now.artist.as_str(), now.album.as_deref()),
        ("multiplay", Some("Golden Fixtures"))
    );
    sync_until(&mut app, "playing state", || {
        matches!(
            seen.lock().unwrap().playback,
            Some(MediaPlayback::Playing { .. })
        )
    });

    // The Next key / panel button.
    seen.lock().unwrap().queued.push(MediaKeyEvent::Next);
    sync_until(&mut app, "next track", || app_title(&seen) == "Second");
    assert_eq!(app_selected(&app), 1);

    seen.lock().unwrap().queued.push(MediaKeyEvent::Pause);
    sync_until(&mut app, "paused", || {
        matches!(
            seen.lock().unwrap().playback,
            Some(MediaPlayback::Paused { .. })
        )
    });
}

fn app_title(seen: &Arc<Mutex<Seen>>) -> String {
    seen.lock()
        .unwrap()
        .now
        .as_ref()
        .map(|n| n.title.clone())
        .unwrap_or_default()
}

fn app_selected(app: &MultiplayGuiApp) -> usize {
    app.selected_track_idx
}
