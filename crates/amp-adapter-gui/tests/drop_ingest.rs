//! Files and folders dropped onto any window are added to the playlist. Every window is its own
//! native viewport with its own input, so each one has to read what was dropped on it.

#![forbid(unsafe_code)]

use amp_adapter_gui::viewports::show_body;
use amp_adapter_gui::{MultiplayGuiApp, WindowId};
use eframe::egui::{self, DroppedFile, Pos2, RawInput, Rect, Vec2};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A file the OS dropped onto a window.
#[derive(Debug)]
struct Dropped(PathBuf);

impl DroppedFile for Dropped {
    fn path(&self) -> &Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

struct TestRoot(PathBuf);

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A 0.1 s, 8 kHz mono 16-bit WAV.
fn write_wav(path: &Path) {
    let samples: Vec<i16> = (0..800)
        .map(|i| ((i as f32 * 0.2).sin() * 8000.0) as i16)
        .collect();
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    for v in [1u16, 1] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for v in [8000u32, 16000] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for v in [2u16, 16] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, out).unwrap();
}

fn setup() -> (TestRoot, MultiplayGuiApp, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("multiplay-drop-{}", uuid::Uuid::new_v4()));
    let music = root.join("music");
    let album = music.join("album");
    std::fs::create_dir_all(&album).unwrap();
    let single = music.join("single.wav");
    write_wav(&single);
    write_wav(&album.join("01.wav"));
    write_wav(&album.join("02.wav"));
    let mut app = MultiplayGuiApp::headless(&root.join("state")).expect("headless app");
    app.playlist.clear();
    (TestRoot(root), app, single, album)
}

/// One frame of `window` with `paths` dropped onto it.
fn drop_onto(app: &mut MultiplayGuiApp, window: WindowId, paths: &[&Path]) {
    let ctx = egui::Context::default();
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 300.0))),
        dropped_files: paths
            .iter()
            .map(|p| Arc::new(Dropped(p.to_path_buf())) as egui::DroppedFileHandle)
            .collect(),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| show_body(app, ui, window));
    output.textures_delta.clear();
}

#[test]
fn files_and_folders_dropped_on_the_playlist_are_added() {
    let (_root, mut app, single, album) = setup();
    drop_onto(&mut app, WindowId::Playlist, &[&single, &album]);
    assert_eq!(app.playlist.len(), 3, "one file plus a folder of two");
}

#[test]
fn drops_on_other_windows_are_added_too() {
    let (_root, mut app, single, _) = setup();
    drop_onto(&mut app, WindowId::Equalizer, &[&single]);
    assert_eq!(app.playlist.len(), 1);
}

#[test]
fn a_drop_on_the_main_window_is_added_once() {
    let (_root, mut app, single, _) = setup();
    drop_onto(&mut app, WindowId::Main, &[&single]);
    assert_eq!(app.playlist.len(), 1);
}

#[test]
fn unplayable_audio_is_reported_not_silently_skipped() {
    let (root, mut app, _, album) = setup();
    std::fs::write(album.join("03.tta"), b"TTA1").unwrap();
    std::fs::write(album.join("04.ape"), b"OggS").unwrap();
    std::fs::write(album.join("05.ape"), b"OggS").unwrap();
    std::fs::write(album.join("cover.jpg"), b"jpg").unwrap();
    drop_onto(&mut app, WindowId::Playlist, &[&album]);
    assert_eq!(app.playlist.len(), 2, "the two WAVs");
    let notice = app
        .import_notice
        .clone()
        .expect("skipped files are reported");
    assert!(notice.contains("3 files skipped"), "{notice}");
    assert!(
        notice.contains("Monkey's Audio (2)") && notice.contains("TTA (1)"),
        "{notice}"
    );
    assert!(
        !notice.contains("jpg"),
        "non-audio files are not mentioned: {notice}"
    );

    // A clean import clears the report.
    let single = root.0.join("music/single.wav");
    drop_onto(&mut app, WindowId::Playlist, &[&single]);
    assert!(app.import_notice.is_none());
}

/// Files no tag reader understands (DSD) still become rows, with the decoder's duration.
#[test]
fn dsd_files_are_added_with_their_duration() {
    let (root, mut app, _, _) = setup();
    let fixtures =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../amp-adapter-decoder/tests/fixtures");
    let dsf = root.0.join("Some Artist - DSD Song.dsf");
    std::fs::copy(fixtures.join("dsd_stereo.dsf"), &dsf).unwrap();
    drop_onto(&mut app, WindowId::Playlist, &[&dsf]);
    assert_eq!(app.playlist.len(), 1);
    let m = &app.playlist[0].metadata;
    assert_eq!(
        (m.artist.as_str(), m.title.as_str()),
        ("Some Artist", "DSD Song")
    );
    let d = m.duration.expect("duration from the decoder");
    assert!((d.as_secs_f64() - 0.25).abs() < 0.01, "{d:?}");
    assert!(app.import_notice.is_none());
}
