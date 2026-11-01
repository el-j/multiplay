//! Keyboard-only use (TASK-175): Winamp's transport keys plus seek, volume, jump and open-location,
//! one action table shared by the buttons and the keys.

#![forbid(unsafe_code)]

use amp_adapter_gui::transport::{key_action, TransportAction as A};
use amp_adapter_gui::MultiplayGuiApp;
use amp_domain::{AudioSource, Track, TrackMetadata};
use eframe::egui::{Key, Modifiers};
use std::path::Path;

#[test]
fn winamp_keys_and_arrows_map_to_transport_actions() {
    let none = Modifiers::NONE;
    for (key, mods, want) in [
        (Key::Z, none, Some(A::Previous)),
        (Key::X, none, Some(A::Play)),
        (Key::C, none, Some(A::PlayPause)),
        (Key::V, none, Some(A::Stop)),
        (Key::B, none, Some(A::Next)),
        (Key::Space, none, Some(A::PlayPause)),
        (Key::ArrowLeft, none, Some(A::SeekBy(-5))),
        (Key::ArrowRight, none, Some(A::SeekBy(5))),
        (Key::ArrowUp, none, Some(A::VolumeBy(5))),
        (Key::ArrowDown, none, Some(A::VolumeBy(-5))),
        (Key::J, none, Some(A::JumpToFile)),
        (Key::L, Modifiers::COMMAND, Some(A::OpenLocation)),
        // Command shortcuts belong to the app (Cmd+C copies, Cmd+V pastes).
        (Key::C, Modifiers::COMMAND, None),
        (Key::V, Modifiers::COMMAND, None),
        (Key::Q, none, None),
    ] {
        assert_eq!(key_action(key, mods), want, "{key:?} {mods:?}");
    }
}

fn fixture(file: &str) -> Track {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../amp-adapter-decoder/tests/fixtures")
        .join(file);
    Track::new(
        AudioSource::LocalFile {
            path: path.to_string_lossy().into_owned(),
        },
        TrackMetadata::new(file, "multiplay"),
    )
}

#[test]
fn actions_drive_the_playlist_and_the_engine() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = MultiplayGuiApp::headless(dir.path()).expect("headless app");
    app.playlist = vec![fixture("wav_16.wav"), fixture("mp3_cbr_128.mp3")];

    app.apply_transport(A::Play);
    assert_eq!(app.selected_track_idx, 0);
    app.apply_transport(A::Next);
    assert_eq!(app.selected_track_idx, 1);
    app.apply_transport(A::Next); // already the last track
    assert_eq!(app.selected_track_idx, 1);
    app.apply_transport(A::Previous);
    assert_eq!(app.selected_track_idx, 0);

    app.volume = 0.5;
    app.apply_transport(A::VolumeBy(5));
    assert!((app.volume - 0.55).abs() < 1e-6, "{}", app.volume);
    app.volume = 0.98;
    app.apply_transport(A::VolumeBy(5));
    assert!((app.volume - 1.0).abs() < 1e-6, "clamped: {}", app.volume);

    app.apply_transport(A::OpenLocation);
    assert!(
        app.focus_location_field,
        "Ctrl+L puts the cursor in the link field"
    );
    assert!(app
        .dock_manager
        .is_visible(amp_adapter_gui::WindowId::Search));
}

#[test]
fn the_scrubber_seeks_once_on_release_not_while_dragging() {
    use amp_adapter_gui::transport::Scrub;
    let mut scrub = Scrub::default();
    // Dragging shows the target but does not seek yet.
    assert_eq!(scrub.drag_to(0.25), None);
    assert_eq!(scrub.drag_to(0.5), None);
    assert_eq!(scrub.shown(0.1), 0.5, "the knob follows the drag");
    // Release: one seek, to where the knob was let go.
    assert_eq!(scrub.release(), Some(0.5));
    assert_eq!(scrub.shown(0.1), 0.1, "back to the playing position");
    // A click (no drag) seeks straight away.
    assert_eq!(scrub.click(0.8), Some(0.8));
}
