//! Turning dropped, picked or typed paths into playlist entries, for every front end: audio
//! files, folders (scanned), M3U/M3U8, PLS and XSPF playlists, and CUE sheets (one entry per
//! song). Skins are handed back for the caller to load; audio that cannot be played is counted
//! by format so the caller can say so (TASK-225, TASK-232, TASK-174).

#![forbid(unsafe_code)]

use crate::{
    extract_track_from_file, import_cue, import_m3u8_with_base, import_pls, import_xspf,
    scan_audio_files,
};
use amp_domain::formats::{self, FormatStatus};
use amp_domain::Track;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Extensions of files that list tracks rather than hold audio.
pub const LIST_EXTENSIONS: [&str; 5] = ["m3u", "m3u8", "pls", "xspf", "cue"];

/// Extensions of skin files, which are not imported but handed back.
const SKIN_EXTENSIONS: [&str; 1] = ["wsz"];

/// What one import produced.
#[derive(Debug, Default)]
pub struct ImportReport {
    /// New playlist entries, in the order the paths were given.
    pub tracks: Vec<Track>,
    /// Skin files (`.wsz`) among the paths.
    pub skins: Vec<PathBuf>,
    /// Audio files that cannot be played, counted by format name.
    pub skipped: BTreeMap<&'static str, usize>,
}

fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
}

/// The tracks a playlist or CUE sheet lists, with relative paths resolved against its folder;
/// `None` when `path` is not one.
#[must_use]
pub fn import_list_file(path: &Path) -> Option<Vec<Track>> {
    let ext = extension(path).filter(|e| LIST_EXTENSIONS.contains(&e.as_str()))?;
    let base = path.parent();
    let text = || {
        std::fs::read(path)
            .ok()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    };
    let tracks = match ext.as_str() {
        "pls" => text().map(|t| import_pls(&t, base)),
        "xspf" => text().map(|t| import_xspf(&t, base)),
        "cue" => import_cue(path).ok(),
        _ => text().map(|t| import_m3u8_with_base(&t, base)),
    };
    Some(tracks.unwrap_or_default())
}

/// Imports `paths`. `fallback` makes a track for a playable file no tag reader understands
/// (DSD), typically from the decoder's duration and rate.
pub fn import_paths(paths: &[PathBuf], fallback: &dyn Fn(&Path) -> Option<Track>) -> ImportReport {
    import_paths_with_progress(paths, fallback, &|_| {})
}

/// Like [`import_paths`], calling `progress` with the number of tracks found so far as it goes,
/// for a progress line while a large folder is scanned.
pub fn import_paths_with_progress(
    paths: &[PathBuf],
    fallback: &dyn Fn(&Path) -> Option<Track>,
    progress: &dyn Fn(usize),
) -> ImportReport {
    let mut report = ImportReport::default();
    for path in paths {
        import_one(path, fallback, &mut report, progress);
    }
    report
}

fn read_track(path: &Path, fallback: &dyn Fn(&Path) -> Option<Track>) -> Option<Track> {
    extract_track_from_file(path).or_else(|| {
        formats::is_playable_path(path)
            .then(|| fallback(path))
            .flatten()
    })
}

fn import_one(
    path: &Path,
    fallback: &dyn Fn(&Path) -> Option<Track>,
    report: &mut ImportReport,
    progress: &dyn Fn(usize),
) {
    if extension(path).is_some_and(|e| SKIN_EXTENSIONS.contains(&e.as_str())) {
        report.skins.push(path.to_path_buf());
        return;
    }
    if let Some(listed) = import_list_file(path) {
        report.tracks.extend(listed);
        return;
    }
    if path.is_dir() {
        let scan = scan_audio_files(path);
        for file in &scan.files {
            if let Some(track) = read_track(file, fallback) {
                report.tracks.push(track);
                progress(report.tracks.len());
            }
        }
        for cue in &scan.cue_sheets {
            report
                .tracks
                .extend(import_list_file(cue).unwrap_or_default());
        }
        for (format, n) in scan.skipped {
            *report.skipped.entry(format).or_default() += n;
        }
    } else if let Some(format) =
        formats::by_path(path).filter(|f| f.status != FormatStatus::Playable)
    {
        *report.skipped.entry(format.name).or_default() += 1;
    } else if let Some(track) = read_track(path, fallback) {
        report.tracks.push(track);
    }
}
