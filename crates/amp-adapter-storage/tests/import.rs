//! One import for every front end (TASK-174 brought the TUI up to the GUI): files, folders,
//! playlists and CUE sheets become tracks; skins are handed back; unplayable audio is counted.

#![forbid(unsafe_code)]

use amp_adapter_storage::import::{import_paths, LIST_EXTENSIONS};
use std::path::{Path, PathBuf};

fn decoder_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../amp-adapter-decoder/tests/fixtures")
        .join(name)
}

#[test]
fn files_folders_lists_skins_and_unplayable_audio() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("album")).unwrap();
    std::fs::copy(
        decoder_fixture("flac_16_44k.flac"),
        root.join("album/01.flac"),
    )
    .unwrap();
    std::fs::copy(
        decoder_fixture("mp3_cbr_128.mp3"),
        root.join("album/02.mp3"),
    )
    .unwrap();
    std::fs::write(root.join("album/03.ape"), b"MAC ").unwrap();
    std::fs::copy(decoder_fixture("wav_16.wav"), root.join("single.wav")).unwrap();
    std::fs::write(root.join("list.m3u"), "album/02.mp3\nsingle.wav\n").unwrap();
    std::fs::write(root.join("look.wsz"), b"PK").unwrap();

    let paths = [
        root.join("album"),
        root.join("single.wav"),
        root.join("list.m3u"),
        root.join("look.wsz"),
    ];
    let report = import_paths(&paths, &|_| None);

    // Folder: 2 playable files; single file: 1; playlist: 2 entries.
    assert_eq!(report.tracks.len(), 5, "{:?}", report.tracks);
    assert_eq!(report.skins, vec![root.join("look.wsz")]);
    assert_eq!(
        report.skipped.get("Monkey's Audio"),
        Some(&1),
        "{:?}",
        report.skipped
    );
}

#[test]
fn a_file_no_tag_reader_understands_uses_the_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let dsf = dir.path().join("x.dsf");
    std::fs::copy(decoder_fixture("dsd_stereo.dsf"), &dsf).unwrap();
    let report = import_paths(std::slice::from_ref(&dsf), &|p| {
        Some(amp_adapter_storage::file_name_track(
            p,
            None,
            Some(2_822_400),
        ))
    });
    assert_eq!(report.tracks.len(), 1);
    assert!(LIST_EXTENSIONS.contains(&"cue"));
}

#[test]
fn internet_shortcuts_url_and_webloc_import_as_tracks() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // 1. Windows .url internet shortcut
    let yt_url_file = root.join("rick.url");
    std::fs::write(
        &yt_url_file,
        "[InternetShortcut]\r\nURL=https://www.youtube.com/watch?v=dQw4w9WgXcQ\r\n",
    )
    .unwrap();

    // 2. macOS .webloc internet shortcut
    let bc_webloc_file = root.join("track.webloc");
    std::fs::write(
        &bc_webloc_file,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>URL</key>
    <string>https://artist.bandcamp.com/track/my-track</string>
</dict>
</plist>"#,
    )
    .unwrap();

    let paths = [yt_url_file, bc_webloc_file];
    let report = import_paths(&paths, &|_| None);

    assert_eq!(report.tracks.len(), 2, "{:?}", report.tracks);
    assert!(matches!(
        report.tracks[0].source,
        amp_domain::AudioSource::YouTube { ref video_id } if video_id == "dQw4w9WgXcQ"
    ));
    assert!(matches!(
        report.tracks[1].source,
        amp_domain::AudioSource::Bandcamp { ref stream_url } if stream_url.contains("bandcamp.com")
    ));
}

