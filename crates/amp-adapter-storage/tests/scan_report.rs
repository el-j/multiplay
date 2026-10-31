//! Scanning reports audio files it cannot play, grouped by format (TASK-225), and ignores
//! everything that is not audio.

#![forbid(unsafe_code)]

use amp_adapter_storage::scan_audio_files;

#[test]
fn skipped_audio_is_counted_by_format() {
    let dir = std::env::temp_dir().join(format!("multiplay-scan-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    for name in [
        "a.flac",
        "sub/b.MP3",
        "c.tta",
        "sub/d.tta",
        "e.ape",
        "cover.jpg",
        "notes.txt",
    ] {
        std::fs::write(dir.join(name), b"x").unwrap();
    }
    let report = scan_audio_files(&dir);
    let mut files: Vec<_> = report
        .files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    files.sort();
    assert_eq!(files, vec!["a.flac", "b.MP3"]);
    assert_eq!(report.skipped_count(), 3);
    assert_eq!(report.skipped.get("TTA"), Some(&2));
    assert_eq!(report.skipped.get("Monkey's Audio"), Some(&1));
    let _ = std::fs::remove_dir_all(dir);
}
