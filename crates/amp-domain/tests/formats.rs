//! The audio format registry (TASK-225) is the only list of file formats: scanner, file dialog,
//! drop, phone upload and decoder hints all ask it, so they can never disagree again.

#![forbid(unsafe_code)]

use amp_domain::formats::{self, FormatStatus};
use std::collections::HashSet;
use std::path::Path;

#[test]
fn common_formats_are_playable() {
    for ext in ["mp3", "flac", "wav", "ogg", "oga", "m4a", "aac"] {
        assert!(formats::is_playable_extension(ext), ".{ext}");
    }
}

#[test]
fn lookup_ignores_case_and_a_leading_dot() {
    assert!(formats::is_playable_extension("FLAC"));
    assert!(formats::is_playable_extension(".Mp3"));
    assert_eq!(formats::by_extension(".FLAC").map(|f| f.name), Some("FLAC"));
    assert!(formats::is_playable_path(Path::new(
        "/m/Album/01 Song.FLAC"
    )));
    assert!(!formats::is_playable_path(Path::new("/m/cover.jpg")));
    assert!(!formats::is_playable_path(Path::new("/m/README")));
}

#[test]
fn every_extension_belongs_to_one_format() {
    let mut seen = HashSet::new();
    for f in formats::AUDIO_FORMATS {
        assert!(!f.extensions.is_empty(), "{} has no extension", f.name);
        for ext in f.extensions {
            assert_eq!(*ext, ext.to_ascii_lowercase(), "{ext}");
            assert!(seen.insert(*ext), ".{ext} is listed twice");
        }
    }
}

#[test]
fn audio_that_cannot_play_yet_says_why() {
    let reason = formats::skip_reason(Path::new("song.wv")).expect("WavPack is known");
    assert!(reason.contains("WavPack"), "{reason}");
    let opus = formats::skip_reason(Path::new("song.opus")).expect("Opus is known");
    assert!(opus.contains("Opus"), "{opus}");
    assert_eq!(
        formats::skip_reason(Path::new("song.flac")),
        None,
        "playable"
    );
    assert_eq!(
        formats::skip_reason(Path::new("cover.jpg")),
        None,
        "not audio"
    );
}

#[test]
fn planned_formats_name_their_task() {
    for f in formats::AUDIO_FORMATS {
        if let FormatStatus::Planned { task } = f.status {
            assert!(task.starts_with("TASK-"), "{}: {task}", f.name);
        }
    }
}

#[test]
fn playable_extensions_are_listed_once_in_registry_order() {
    let list: Vec<_> = formats::playable_extensions().collect();
    assert!(list.contains(&"flac") && list.contains(&"mp3"));
    assert!(!list.contains(&"wv") && !list.contains(&"opus"));
    let unique: HashSet<_> = list.iter().collect();
    assert_eq!(unique.len(), list.len());
}

/// No other crate may keep its own extension list: a literal like `"m4a"` in production code
/// outside the registry means a second list has crept back in.
#[test]
fn no_crate_keeps_its_own_extension_list() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    // Container sniffing by magic bytes names the result; it is not a list of accepted formats.
    let allowed = [
        "amp-domain/src/formats.rs",
        "amp-core-engine/src/downloader.rs",
    ];
    let mut offenders = Vec::new();
    for entry in walk(&crates) {
        let rel = entry
            .strip_prefix(&crates)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if !rel.contains("/src/") || allowed.iter().any(|a| rel.ends_with(a)) {
            continue;
        }
        let text = std::fs::read_to_string(&entry).unwrap_or_default();
        // Unit tests at the bottom of a file may name formats.
        let code = text.split("#[cfg(test)]").next().unwrap_or("");
        if code.contains("\"m4a\"") || code.contains("\"flac\"") {
            offenders.push(rel);
        }
    }
    assert!(
        offenders.is_empty(),
        "own extension lists in: {offenders:?}"
    );
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name().unwrap_or_default().to_string_lossy();
        if p.is_dir() && name != "target" && !name.starts_with('.') {
            out.extend(walk(&p));
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
    out
}

#[test]
fn mime_types_find_their_format() {
    assert_eq!(formats::by_mime("audio/flac").map(|f| f.name), Some("FLAC"));
    assert_eq!(
        formats::by_mime("Audio/MPEG; charset=x").map(|f| f.name),
        Some("MP3")
    );
    assert_eq!(formats::by_mime("audio/opus").map(|f| f.name), Some("Opus"));
    assert_eq!(formats::by_mime("text/html"), None);
}
