//! Playlists and CUE sheets come from anywhere (TASK-185): arbitrary text never panics, and an
//! oversized list file is not read at all.

#![forbid(unsafe_code)]

use amp_adapter_storage::cue::decode_cue_text;
use amp_adapter_storage::import::{import_list_file, MAX_LIST_FILE_BYTES};
use amp_adapter_storage::{import_m3u8_with_base, import_pls, import_xspf, parse_cue};
use proptest::prelude::*;
use std::path::Path;

#[test]
fn an_oversized_playlist_is_not_read() {
    let dir = tempfile::tempdir().unwrap();
    let big = dir.path().join("huge.m3u");
    let f = std::fs::File::create(&big).unwrap();
    f.set_len(MAX_LIST_FILE_BYTES + 1).unwrap();
    assert_eq!(import_list_file(&big).map(|t| t.len()), Some(0));
}

/// Cases per property: the default in CI, many more in the nightly fuzz job (`FUZZ_CASES`).
fn cases(default: u32) -> u32 {
    std::env::var("FUZZ_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(cases(512)))]

    #[test]
    fn arbitrary_playlist_text_never_panics(text in "\\PC{0,3000}") {
        let base = Some(Path::new("/music"));
        let _ = import_m3u8_with_base(&text, base);
        let _ = import_pls(&text, base);
        let _ = import_xspf(&text, base);
        let _ = parse_cue(&text);
    }

    #[test]
    fn line_shaped_playlist_text_never_panics(lines in proptest::collection::vec(
        prop_oneof![
            Just("#EXTM3U".to_string()),
            "#EXTINF:-?[0-9]{0,6},.{0,40}",
            "File[0-9]{0,3}=.{0,60}",
            "Title[0-9]{0,3}=.{0,40}",
            "Length[0-9]{0,3}=-?[0-9]{0,8}",
            "(FILE|TRACK|INDEX|TITLE|PERFORMER) .{0,40}",
            "INDEX 0[0-9] [0-9]{1,3}:[0-9]{1,3}:[0-9]{1,3}",
            "<(track|location|title|creator|duration)>.{0,40}</[a-z]{0,10}>",
            ".{0,80}",
        ], 0..60)) {
        let text = lines.join("\n");
        let _ = import_m3u8_with_base(&text, None);
        let _ = import_pls(&text, None);
        let _ = import_xspf(&text, None);
        let _ = parse_cue(&text);
    }

    #[test]
    fn arbitrary_cue_bytes_decode(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
        let _ = parse_cue(&decode_cue_text(&bytes));
    }
}
