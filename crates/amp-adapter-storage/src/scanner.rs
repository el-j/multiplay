//! Asynchronous, incremental, symlink-safe local audio library scanner (`TASK-149`)
//! with pure-Rust Lofty metadata tag extraction.

#![forbid(unsafe_code)]

use amp_domain::{AudioSource, Track, TrackId, TrackMetadata};
use amp_ports::driven::LibraryStoragePort;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

/// Whether a file is a playable audio file; the format list lives in `amp_domain::formats` (TASK-225).
pub use amp_domain::formats::is_playable_path;

/// Options controlling directory traversal and incremental scanning (`TASK-149`).
#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Whether to follow symbolic links during traversal (`false` by default for safety).
    /// Even when `true`, a visited canonical-directory set prevents symlink loops.
    pub follow_symlinks: bool,
    /// Optional cancellation flag checked between files and directories.
    pub cancel_flag: Option<Arc<AtomicBool>>,
}

/// Progress snapshot emitted or returned by an incremental library scan (`TASK-149`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanProgress {
    pub discovered_files: usize,
    pub newly_indexed: usize,
    pub skipped_unchanged: usize,
    pub cancelled: bool,
}

/// Per-file `(mtime_secs, size_bytes)` fingerprint for incremental scanning (`TASK-149`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFingerprint {
    pub mtime_secs: u64,
    pub size_bytes: u64,
}

impl FileFingerprint {
    /// Computes the fingerprint of `path` from filesystem metadata.
    pub fn of(path: &Path) -> Option<Self> {
        let meta = fs::metadata(path).ok()?;
        let mtime_secs = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Some(Self {
            mtime_secs,
            size_bytes: meta.len(),
        })
    }
}

/// Incremental scan cache mapping canonical file path strings to their last indexed [`FileFingerprint`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IncrementalScanCache {
    entries: HashMap<String, FileFingerprint>,
}

impl IncrementalScanCache {
    /// Creates an empty [`IncrementalScanCache`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if `path` has the exact same `(mtime_secs, size_bytes)` as previously indexed.
    pub fn is_unchanged(&self, path: &Path) -> bool {
        let key = path.to_string_lossy();
        let Some(prev) = self.entries.get(key.as_ref()) else {
            return false;
        };
        FileFingerprint::of(path).is_some_and(|curr| curr == *prev)
    }

    /// Records the current fingerprint of `path`.
    pub fn record(&mut self, path: &Path) {
        if let Some(fp) = FileFingerprint::of(path) {
            self.entries.insert(path.to_string_lossy().into_owned(), fp);
        }
    }

    /// Number of tracked file fingerprints.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if no files are tracked yet.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub struct LibraryScanner;

impl LibraryScanner {
    /// Scans a directory recursively and indexes all discovered audio tracks into storage.
    pub async fn scan_directory<P: AsRef<Path>, S: LibraryStoragePort + 'static>(
        dir_path: P,
        storage: Arc<S>,
    ) -> Result<usize, String> {
        let mut cache = IncrementalScanCache::new();
        let progress =
            Self::scan_directory_incremental(dir_path, storage, &mut cache, ScanOptions::default())
                .await?;
        Ok(progress.newly_indexed)
    }

    /// Performs a symlink-safe, cancellable, incremental directory scan (`TASK-149`).
    /// Unchanged files (matching `mtime_secs` and `size_bytes` in `cache`) are skipped without re-probing tags.
    pub async fn scan_directory_incremental<P: AsRef<Path>, S: LibraryStoragePort + 'static>(
        dir_path: P,
        storage: Arc<S>,
        cache: &mut IncrementalScanCache,
        options: ScanOptions,
    ) -> Result<ScanProgress, String> {
        let root = dir_path.as_ref().to_path_buf();
        let opts_clone = options.clone();
        let files = tokio::task::spawn_blocking(move || {
            collect_audio_files_with_options(&root, &opts_clone)
        })
        .await
        .map_err(|e| format!("Directory crawler panic: {e}"))?;

        let mut progress = ScanProgress {
            discovered_files: files.len(),
            ..Default::default()
        };

        for path in files {
            if options
                .cancel_flag
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
            {
                progress.cancelled = true;
                break;
            }

            if cache.is_unchanged(&path) {
                progress.skipped_unchanged += 1;
                continue;
            }

            if let Some(track) = extract_track_from_file(&path) {
                if storage.store_track(&track).await.is_ok() {
                    cache.record(&path);
                    progress.newly_indexed += 1;
                }
            }
        }

        Ok(progress)
    }
}

/// Playable files found under a folder, and how many audio files of each unplayable format
/// were left out (TASK-225).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ScanReport {
    pub files: Vec<PathBuf>,
    /// Format name (e.g. "WavPack") to number of files skipped.
    pub skipped: std::collections::BTreeMap<&'static str, usize>,
}

impl ScanReport {
    pub fn skipped_count(&self) -> usize {
        self.skipped.values().sum()
    }
}

/// Collects supported audio files recursively under `dir` without following symlinks (`TASK-149`).
pub fn collect_audio_files(dir: &Path) -> Vec<PathBuf> {
    collect_audio_files_with_options(dir, &ScanOptions::default())
}

/// Like [`collect_audio_files`], and also counts audio files that cannot be played.
pub fn scan_audio_files(dir: &Path) -> ScanReport {
    let mut report = ScanReport::default();
    collect_recursive(
        dir,
        &ScanOptions::default(),
        &mut HashSet::new(),
        &mut report,
    );
    report
}

/// Collects supported audio files under `dir` honoring [`ScanOptions`] (symlink policy and cancellation).
pub fn collect_audio_files_with_options(dir: &Path, options: &ScanOptions) -> Vec<PathBuf> {
    let mut report = ScanReport::default();
    collect_recursive(dir, options, &mut HashSet::new(), &mut report);
    report.files
}

fn collect_recursive(
    dir: &Path,
    options: &ScanOptions,
    visited_dirs: &mut HashSet<PathBuf>,
    report: &mut ScanReport,
) {
    if options
        .cancel_flag
        .as_ref()
        .is_some_and(|f| f.load(Ordering::Relaxed))
    {
        return;
    }

    if let Ok(canon) = fs::canonicalize(dir) {
        if !visited_dirs.insert(canon) {
            return; // Cycle detected
        }
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        if options
            .cancel_flag
            .as_ref()
            .is_some_and(|f| f.load(Ordering::Relaxed))
        {
            return;
        }

        let path = entry.path();
        let Ok(sym_meta) = fs::symlink_metadata(&path) else {
            continue;
        };

        if sym_meta.file_type().is_symlink() && !options.follow_symlinks {
            continue;
        }

        if path.is_dir() {
            collect_recursive(&path, options, visited_dirs, report);
        } else if is_playable_path(&path) {
            report.files.push(path);
        } else if let Some(format) = amp_domain::formats::by_path(&path) {
            *report.skipped.entry(format.name).or_default() += 1;
        }
    }
}

pub fn extract_track_from_file(path: &Path) -> Option<Track> {
    let tagged_file = Probe::open(path).ok()?.read().ok()?;

    let properties = tagged_file.properties();
    let duration = Some(properties.duration());
    let sample_rate = properties.sample_rate();
    let channels = properties.channels().map(|c| c as u16);
    let bitrate_kbps = properties.audio_bitrate();

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());

    let title = tag
        .and_then(|t| t.title().as_deref().map(str::to_string))
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown Title")
                .to_string()
        });

    let artist = tag
        .and_then(|t| t.artist().as_deref().map(str::to_string))
        .unwrap_or_else(|| "Unknown Artist".to_string());

    let album = tag.and_then(|t| t.album().as_deref().map(str::to_string));
    let track_number = tag.and_then(|t| t.track());

    let (replay_gain_mb, integrated_lufs_mb) = if let Some(t) = tag {
        let rg = t
            .get_string(lofty::tag::ItemKey::ReplayGainTrackGain)
            .and_then(parse_gain_db)
            .map(|db| (db * 100.0).round() as i32);
        let lufs = t
            .get_string(lofty::tag::ItemKey::R128TrackGain)
            .and_then(r128_gain_to_lufs)
            .map(|v| (v * 100.0).round() as i32);

        (rg, lufs)
    } else {
        (None, None)
    };

    let metadata = TrackMetadata {
        title,
        artist,
        album,
        track_number,
        duration,
        sample_rate,
        channels,
        bit_depth: properties.bit_depth().map(|b| b as u16),
        bitrate_kbps,
        cover_art_uri: None,
        replay_gain_mb,
        integrated_lufs_mb,
    };

    let source = AudioSource::LocalFile {
        path: path.to_string_lossy().to_string(),
    };
    let id = TrackId::from_source(&source);

    Some(Track {
        id,
        source,
        metadata,
        date_added_epoch: 0,
    })
}

/// Parses a ReplayGain value such as `-6.48 dB`.
fn parse_gain_db(s: &str) -> Option<f32> {
    s.replace("dB", "").trim().parse::<f32>().ok()
}

/// Integrated loudness from a Vorbis `R128_TRACK_GAIN` tag. The tag holds the gain (in
/// Q7.8 fixed point, i.e. 1/256 dB) that brings the track to -23 LUFS, so the track's own
/// loudness is -23 LUFS minus that gain.
fn r128_gain_to_lufs(s: &str) -> Option<f32> {
    let q78 = s.trim().parse::<i32>().ok()?;
    Some(-23.0 - q78 as f32 / 256.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RedbStorageAdapter;
    use tempfile::{tempdir, NamedTempFile};

    fn write_minimal_wav(path: &Path) {
        let mut wav_bytes = Vec::new();
        wav_bytes.extend_from_slice(b"RIFF");
        wav_bytes.extend_from_slice(&76u32.to_le_bytes());
        wav_bytes.extend_from_slice(b"WAVEfmt ");
        wav_bytes.extend_from_slice(&16u32.to_le_bytes());
        wav_bytes.extend_from_slice(&1u16.to_le_bytes());
        wav_bytes.extend_from_slice(&2u16.to_le_bytes());
        wav_bytes.extend_from_slice(&44100u32.to_le_bytes());
        wav_bytes.extend_from_slice(&176400u32.to_le_bytes());
        wav_bytes.extend_from_slice(&4u16.to_le_bytes());
        wav_bytes.extend_from_slice(&16u16.to_le_bytes());
        wav_bytes.extend_from_slice(b"data");
        wav_bytes.extend_from_slice(&40u32.to_le_bytes());
        wav_bytes.extend_from_slice(&[0u8; 40]);
        fs::write(path, &wav_bytes).unwrap();
    }

    #[test]
    fn replaygain_and_r128_tags_parse() {
        assert_eq!(parse_gain_db("-6.48 dB"), Some(-6.48));
        assert_eq!(parse_gain_db("+2 dB"), Some(2.0));
        assert_eq!(r128_gain_to_lufs("-1280"), Some(-18.0));
        assert_eq!(r128_gain_to_lufs("512"), Some(-25.0));
        assert_eq!(r128_gain_to_lufs("loud"), None);
    }

    #[test]
    fn test_collect_audio_files() {
        let dir = tempdir().unwrap();
        let audio1 = dir.path().join("song1.mp3");
        let audio2 = dir.path().join("song2.flac");
        let txt = dir.path().join("notes.txt");

        fs::write(&audio1, b"mock").unwrap();
        fs::write(&audio2, b"mock").unwrap();
        fs::write(&txt, b"mock").unwrap();

        let collected = collect_audio_files(dir.path());
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn test_probe_metadata() {
        let dir = tempdir().unwrap();
        let wav_path = dir.path().join("Artist - Title.wav");
        write_minimal_wav(&wav_path);

        let track = extract_track_from_file(&wav_path);
        assert!(track.is_some());
        let t = track.unwrap();
        assert_eq!(t.metadata.title, "Artist - Title");
        assert_eq!(t.metadata.sample_rate, Some(44100));
        assert_eq!(t.metadata.channels, Some(2));
        // Deterministic ID matches TrackId::from_source
        assert_eq!(t.id, TrackId::from_source(&t.source));
    }

    #[cfg(unix)]
    #[test]
    fn scanner_terminates_on_symlink_loop_fixture() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("subdir");
        fs::create_dir_all(&sub).unwrap();
        write_minimal_wav(&sub.join("track.wav"));

        // Create a symlink inside `subdir` pointing back to root `dir`
        let loop_link = sub.join("loop_back");
        std::os::unix::fs::symlink(dir.path(), &loop_link).unwrap();

        // 1. Default (follow_symlinks = false): skips symlink and terminates
        let collected_default = collect_audio_files(dir.path());
        assert_eq!(collected_default.len(), 1);

        // 2. Even with follow_symlinks = true, visited canonical-directory set terminates loop!
        let opts = ScanOptions {
            follow_symlinks: true,
            cancel_flag: None,
        };
        let collected_follow = collect_audio_files_with_options(dir.path(), &opts);
        assert_eq!(collected_follow.len(), 1);
    }

    #[tokio::test]
    async fn scanner_incremental_skips_unchanged_and_honors_cancellation() {
        let dir = tempdir().unwrap();
        write_minimal_wav(&dir.path().join("one.wav"));
        write_minimal_wav(&dir.path().join("two.wav"));

        let db_file = NamedTempFile::new().unwrap();
        let storage = Arc::new(RedbStorageAdapter::new(db_file.path()).unwrap());
        let mut cache = IncrementalScanCache::new();

        let first = LibraryScanner::scan_directory_incremental(
            dir.path(),
            storage.clone(),
            &mut cache,
            ScanOptions::default(),
        )
        .await
        .unwrap();
        assert_eq!(first.discovered_files, 2);
        assert_eq!(first.newly_indexed, 2);
        assert_eq!(first.skipped_unchanged, 0);

        // Second scan on unchanged files skips both in O(1)
        let second = LibraryScanner::scan_directory_incremental(
            dir.path(),
            storage.clone(),
            &mut cache,
            ScanOptions::default(),
        )
        .await
        .unwrap();
        assert_eq!(second.newly_indexed, 0);
        assert_eq!(second.skipped_unchanged, 2);

        // Pre-cancelled scan stops immediately
        let cancel = Arc::new(AtomicBool::new(true));
        let third = LibraryScanner::scan_directory_incremental(
            dir.path(),
            storage,
            &mut cache,
            ScanOptions {
                follow_symlinks: false,
                cancel_flag: Some(cancel),
            },
        )
        .await
        .unwrap();
        assert_eq!(third.newly_indexed, 0);
    }
}
