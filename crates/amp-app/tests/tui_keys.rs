//! The TUI as a complete player (TASK-174): every key in the help overlay does what it says,
//! checked against the real engine (null output) and a ratatui test terminal.

#![forbid(unsafe_code)]

use amp_app::tui_keys::{complete_path, KEY_HELP};
use amp_app::MultiplayApp;
use amp_core_engine::playback::{RepeatMode, ShuffleMode};
use amp_domain::PlaybackState;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../amp-adapter-decoder/tests/fixtures")
        .join(name)
}

fn press(app: &mut MultiplayApp, key: KeyCode) -> bool {
    app.handle_key(key, KeyModifiers::empty())
}

fn type_text(app: &mut MultiplayApp, text: &str) {
    for c in text.chars() {
        press(app, KeyCode::Char(c));
    }
}

fn screen(app: &MultiplayApp) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
    terminal.draw(|f| app.render_frame(f)).unwrap();
    let buf = terminal.backend().buffer().clone();
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn wait_for(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !cond() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn app_with_two_tracks() -> MultiplayApp {
    let (mut app, _rx) = MultiplayApp::new_headless().expect("headless TUI");
    press(&mut app, KeyCode::Char('o'));
    type_text(&mut app, &fixture("wav_16.wav").to_string_lossy());
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Char('o'));
    type_text(&mut app, &fixture("mp3_cbr_128.mp3").to_string_lossy());
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.playlist.len(), 2, "o adds files");
    app
}

#[test]
fn adding_files_folders_and_links_through_the_prompt() {
    let (mut app, _rx) = MultiplayApp::new_headless().expect("headless TUI");
    press(&mut app, KeyCode::Char('o'));
    assert!(
        screen(&app).contains("Add file, folder or link"),
        "{}",
        screen(&app)
    );
    type_text(&mut app, "https://youtu.be/dQw4w9WgXcQ");
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.playlist.len(), 1);

    // Esc cancels without adding.
    press(&mut app, KeyCode::Char('o'));
    type_text(&mut app, "/nowhere");
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.playlist.len(), 1);
    assert!(!screen(&app).contains("Add file, folder or link"));
}

#[test]
fn tab_completes_paths() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Albums")).unwrap();
    std::fs::write(dir.path().join("Albums.txt"), b"").unwrap();
    std::fs::write(dir.path().join("song.flac"), b"").unwrap();
    let base = dir.path().to_string_lossy().into_owned();
    let sep = std::path::MAIN_SEPARATOR;
    assert_eq!(
        complete_path(&format!("{base}{sep}so")),
        format!("{base}{sep}song.flac")
    );
    // Two matches: completes the shared part only.
    assert_eq!(
        complete_path(&format!("{base}{sep}Al")),
        format!("{base}{sep}Albums")
    );
    // A unique folder gets its separator so the next Tab goes inside.
    assert_eq!(
        complete_path(&format!("{base}{sep}Albums")),
        format!("{base}{sep}Albums{sep}")
    );
    assert_eq!(complete_path("/no/such/dir/x"), "/no/such/dir/x");
}

#[test]
fn transport_keys_drive_the_engine() {
    let mut app = app_with_two_tracks();
    let driver = app.audio_driver.clone().expect("engine");

    press(&mut app, KeyCode::Char('x'));
    wait_for("playing", || driver.stream_info().is_some());
    assert_eq!(app.selected_playlist_idx, 0);
    press(&mut app, KeyCode::Char('b'));
    assert_eq!(app.selected_playlist_idx, 1, "b: next");
    press(&mut app, KeyCode::Char('z'));
    assert_eq!(app.selected_playlist_idx, 0, "z: previous");
    press(&mut app, KeyCode::Char('c'));
    assert_eq!(app.playback_state, PlaybackState::Paused, "c: pause");
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(app.playback_state, PlaybackState::Playing, "space: resume");

    *driver.elapsed.lock().unwrap() = Duration::ZERO;
    press(&mut app, KeyCode::Right);
    assert_eq!(
        *driver.seek_requested.lock().unwrap(),
        Some(Duration::from_secs(1)),
        "→ seeks (clamped to the 1 s file)"
    );

    press(&mut app, KeyCode::Char('r'));
    assert_eq!(driver.repeat_mode(), RepeatMode::RepeatAll);
    press(&mut app, KeyCode::Char('r'));
    assert_eq!(driver.repeat_mode(), RepeatMode::RepeatOne);
    press(&mut app, KeyCode::Char('r'));
    assert_eq!(driver.repeat_mode(), RepeatMode::Off);
    press(&mut app, KeyCode::Char('s'));
    assert_ne!(driver.shuffle_mode(), ShuffleMode::Off);
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(driver.shuffle_mode(), ShuffleMode::Off);

    press(&mut app, KeyCode::Char('v'));
    assert_eq!(app.playback_state, PlaybackState::Stopped, "v: stop");

    press(&mut app, KeyCode::Delete);
    assert_eq!(app.playlist.len(), 1, "Delete removes the selected track");
}

#[test]
fn eq_keys_pick_a_band_and_change_it() {
    let (mut app, _rx) = MultiplayApp::new_headless().expect("headless TUI");
    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Char('}'));
    press(&mut app, KeyCode::Char('}'));
    assert_eq!(app.eq_gains[2], 2.0, "band 3 up by 2 dB");
    press(&mut app, KeyCode::Char('['));
    press(&mut app, KeyCode::Char('{'));
    assert_eq!(app.eq_gains[1], -1.0);
    let driver = app.audio_driver.clone().unwrap();
    assert_eq!(
        driver.eq_gains.lock().unwrap()[2],
        2.0,
        "the engine hears it"
    );
    for _ in 0..30 {
        press(&mut app, KeyCode::Char('}'));
    }
    assert_eq!(app.eq_gains[1], 12.0, "clamped to +12 dB");
}

#[test]
fn help_lists_every_key_and_quitting_while_playing_asks_first() {
    let mut app = app_with_two_tracks();
    press(&mut app, KeyCode::Char('?'));
    let shown = screen(&app);
    for (key, what) in KEY_HELP {
        assert!(
            shown.contains(key) && shown.contains(what),
            "help shows {key} {what}:\n{shown}"
        );
    }
    press(&mut app, KeyCode::Esc);
    assert!(!screen(&app).contains(KEY_HELP[0].1));

    press(&mut app, KeyCode::Char('x'));
    assert!(
        !press(&mut app, KeyCode::Char('q')),
        "q while playing asks first"
    );
    assert!(screen(&app).contains("Quit while playing?"));
    assert!(!press(&mut app, KeyCode::Char('n')), "n keeps playing");
    assert!(!press(&mut app, KeyCode::Char('q')));
    assert!(press(&mut app, KeyCode::Char('y')), "y quits");

    let (mut idle, _rx) = MultiplayApp::new_headless().unwrap();
    assert!(
        press(&mut idle, KeyCode::Char('q')),
        "q quits at once when nothing plays"
    );
}
