//! Album covers for the library browser (TASK-172): a small PNG thumbnail per audio file, made
//! from the file's embedded art or the folder's cover image (`cover.jpg`, `folder.png`, ...),
//! cached on disk and pruned oldest-first above a size limit. Only local files: covers of online
//! tracks would mean new connections (docs/privacy.md), so they get none.

#![forbid(unsafe_code)]

use crate::tags::embedded_cover_art;
use amp_domain::{AudioSource, Track};
use std::path::{Path, PathBuf};

/// Thumbnail edge in pixels; the album grid draws them at up to this size.
pub const THUMBNAIL_PX: u32 = 160;

/// File names (case-insensitive, without extension) a folder's cover image goes by.
const FOLDER_COVER_NAMES: [&str; 4] = ["cover", "folder", "front", "album"];
const IMAGE_EXTENSIONS: [&str; 3] = ["jpg", "jpeg", "png"];

/// The on-disk thumbnail cache.
#[derive(Debug, Clone)]
pub struct ArtworkCache {
    dir: PathBuf,
    max_bytes: u64,
}

/// FNV-1a: a stable file name for a source path (std's hasher may change between releases).
fn key(path: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in path.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

fn folder_cover(audio: &Path) -> Option<Vec<u8>> {
    let dir = audio.parent()?;
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let stem = p.file_stem().map(|s| s.to_string_lossy().to_lowercase());
            let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase());
            stem.is_some_and(|s| FOLDER_COVER_NAMES.contains(&s.as_str()))
                && ext.is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.as_str()))
        })
        .collect();
    // "cover" before "folder" before ...; the same name in any case and extension.
    candidates.sort_by_key(|p| {
        let stem = p
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        FOLDER_COVER_NAMES.iter().position(|n| *n == stem)
    });
    std::fs::read(candidates.first()?).ok()
}

impl ArtworkCache {
    /// A cache in `dir` holding at most `max_bytes`.
    #[must_use]
    pub fn new(dir: &Path, max_bytes: u64) -> Self {
        Self {
            dir: dir.to_path_buf(),
            max_bytes,
        }
    }

    /// The thumbnail for `track`, made now if needed; `None` when it has no art (remembered, so
    /// the files are not read again) or is not a local file.
    #[must_use]
    pub fn thumbnail(&self, track: &Track) -> Option<PathBuf> {
        let AudioSource::LocalFile { path } = &track.source else {
            return None;
        };
        let k = key(path);
        let thumb = self.dir.join(format!("{k}.png"));
        let none = self.dir.join(format!("{k}.none"));
        if thumb.is_file() {
            return Some(thumb);
        }
        if none.is_file() {
            return None;
        }
        let _ = std::fs::create_dir_all(&self.dir);
        let audio = Path::new(path);
        let made = embedded_cover_art(audio)
            .map(|c| c.data)
            .or_else(|| folder_cover(audio))
            .and_then(|bytes| image::load_from_memory(&bytes).ok())
            .map(|img| img.thumbnail(THUMBNAIL_PX, THUMBNAIL_PX))
            .and_then(|small| small.save(&thumb).ok());
        if made.is_none() {
            let _ = std::fs::write(&none, b"");
            return None;
        }
        self.prune(&thumb);
        Some(thumb)
    }

    /// Deletes the oldest thumbnails until the cache fits, keeping `keep`.
    fn prune(&self, keep: &Path) {
        let mut files: Vec<(std::time::SystemTime, u64, PathBuf)> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                Some((meta.modified().ok()?, meta.len(), e.path()))
            })
            .collect();
        let mut total: u64 = files.iter().map(|f| f.1).sum();
        files.sort();
        for (_, len, path) in files {
            if total <= self.max_bytes {
                break;
            }
            if path != keep && std::fs::remove_file(&path).is_ok() {
                total -= len;
            }
        }
    }
}
