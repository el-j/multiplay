//! Album covers for the library browser (TASK-172): small thumbnails from the file's embedded
//! art or the folder's cover image, cached on disk under a size limit.

#![forbid(unsafe_code)]

use amp_adapter_storage::artwork::{ArtworkCache, THUMBNAIL_PX};
use amp_domain::{AudioSource, Track, TrackMetadata};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../amp-adapter-decoder/tests/fixtures")
        .join(name)
}

fn local(path: &Path) -> Track {
    Track::new(
        AudioSource::LocalFile {
            path: path.to_string_lossy().into_owned(),
        },
        TrackMetadata::new("t", "a"),
    )
}

fn png_size(path: &Path) -> (u32, u32) {
    let img = image::open(path).expect("a decodable PNG");
    (img.width(), img.height())
}

#[test]
fn embedded_art_becomes_a_cached_thumbnail() {
    let cache_dir = tempfile::tempdir().unwrap();
    let cache = ArtworkCache::new(cache_dir.path(), 1 << 20);
    for file in ["cover.flac", "cover.mp3", "cover.m4a"] {
        let thumb = cache
            .thumbnail(&local(&fixture(file)))
            .unwrap_or_else(|| panic!("{file}"));
        assert!(thumb.starts_with(cache_dir.path()));
        let (w, h) = png_size(&thumb);
        assert!(w <= THUMBNAIL_PX && h <= THUMBNAIL_PX, "{file}: {w}x{h}");
        // Second ask: the same file, not made again.
        let again = cache.thumbnail(&local(&fixture(file))).unwrap();
        assert_eq!(again, thumb);
    }
}

#[test]
fn a_folder_cover_image_is_used_when_the_file_has_none() {
    let music = tempfile::tempdir().unwrap();
    let song = music.path().join("01.wav");
    std::fs::copy(fixture("wav_16.wav"), &song).unwrap();
    let cache_dir = tempfile::tempdir().unwrap();
    let cache = ArtworkCache::new(cache_dir.path(), 1 << 20);
    assert_eq!(cache.thumbnail(&local(&song)), None, "no art anywhere");

    image::RgbImage::from_pixel(600, 400, image::Rgb([200, 30, 30]))
        .save(music.path().join("Folder.JPG"))
        .unwrap();
    // A fresh cache (the "no art" answer above is remembered by the first one).
    let cache = ArtworkCache::new(tempfile::tempdir().unwrap().path(), 1 << 20);
    let thumb = cache.thumbnail(&local(&song)).expect("folder image");
    // 600x400 scaled to fit 160 wide: 106.7 rounds to 107.
    assert_eq!(png_size(&thumb), (THUMBNAIL_PX, 107));
}

#[test]
fn online_tracks_get_no_thumbnail_and_nothing_is_fetched() {
    let cache = ArtworkCache::new(tempfile::tempdir().unwrap().path(), 1 << 20);
    let yt = Track::new(
        AudioSource::YouTube {
            video_id: "dQw4w9WgXcQ".into(),
        },
        TrackMetadata::new("t", "a"),
    );
    assert_eq!(cache.thumbnail(&yt), None);
}

#[test]
fn the_cache_stays_under_its_size_limit() {
    let cache_dir = tempfile::tempdir().unwrap();
    let music = tempfile::tempdir().unwrap();
    // Room for about two of these (noisy, so ~22 KB each); eight are made.
    let cache = ArtworkCache::new(cache_dir.path(), 60_000);
    for i in 0..8 {
        let dir = music.path().join(format!("album{i}"));
        std::fs::create_dir(&dir).unwrap();
        let song = dir.join("01.wav");
        std::fs::copy(fixture("wav_16.wav"), &song).unwrap();
        image::RgbImage::from_fn(300, 300, |x, y| {
            image::Rgb([(x * 7 + i) as u8, (y * 3) as u8, (x ^ y) as u8])
        })
        .save(dir.join("cover.png"))
        .unwrap();
        assert!(cache.thumbnail(&local(&song)).is_some());
    }
    let total: u64 = std::fs::read_dir(cache_dir.path())
        .unwrap()
        .flatten()
        .map(|e| e.metadata().unwrap().len())
        .sum();
    assert!(total <= 60_000, "cache holds {total} bytes");
    let left = std::fs::read_dir(cache_dir.path()).unwrap().count();
    assert!(
        (1..8).contains(&left),
        "oldest pruned, newest kept: {left} files"
    );
}
