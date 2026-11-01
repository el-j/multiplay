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
