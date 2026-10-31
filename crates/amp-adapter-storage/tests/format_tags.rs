//! Library metadata for every playable format (TASK-230): each golden fixture from the decoder's
//! corpus must become a track with its tags, duration and stream properties.

#![forbid(unsafe_code)]

use amp_adapter_storage::extract_track_from_file;
use amp_domain::formats;
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../amp-adapter-decoder/tests/fixtures")
}

/// What a fixture's tags should yield.
#[derive(Clone, Copy, PartialEq)]
enum Tags {
    /// Title, artist and album.
    Full,
    /// Title and artist (the writer set no album).
    NoAlbum,
    /// Matroska/WebM via the Symphonia fallback: artist and album; ffmpeg puts the title
    /// outside the tags, so it comes from the file name.
    ArtistAlbum,
    /// No tag support for this container (CAF, ADPCM WAV): file name and duration only.
    NameOnly,
}

/// (file, seconds, expected tags)
const FIXTURES: &[(&str, f64, Tags)] = &[
    ("flac_16_44k.flac", 1.0, Tags::Full),
    ("flac_24_96k.flac", 0.5, Tags::Full),
    ("flac_5_1_48k.flac", 0.5, Tags::Full),
    ("flac_no_seektable.flac", 1.0, Tags::NoAlbum),
    ("wav_16.wav", 1.0, Tags::Full),
    ("wav_f32.wav", 0.5, Tags::Full),
    ("mp3_cbr_128.mp3", 1.0, Tags::NoAlbum),
    ("mp3_vbr.mp3", 1.0, Tags::NoAlbum),
    ("aac.m4a", 1.0, Tags::Full),
    ("alac.m4a", 1.0, Tags::Full),
    ("vorbis.ogg", 1.0, Tags::Full),
    ("aiff_16.aiff", 1.0, Tags::Full),
    ("caf_16.caf", 1.0, Tags::NameOnly),
    ("flac.mka", 1.0, Tags::ArtistAlbum),
    ("vorbis.webm", 1.0, Tags::ArtistAlbum),
    ("mp2_192.mp2", 1.0, Tags::Full),
    ("adpcm_ms.wav", 1.0, Tags::NameOnly),
    ("opus_stereo.opus", 1.0, Tags::Full),
    ("opus.webm", 1.0, Tags::ArtistAlbum),
];

#[test]
fn every_playable_fixture_becomes_a_track_with_its_tags() {
    let mut failures = Vec::new();
    for (file, seconds, tags) in FIXTURES {
        let path = fixtures().join(file);
        assert!(formats::is_playable_path(&path), "{file} not playable");
        let Some(track) = extract_track_from_file(&path) else {
            failures.push(format!("{file}: no track"));
            continue;
        };
        let m = &track.metadata;
        let stem = Path::new(file).file_stem().unwrap().to_string_lossy();
        let (title, artist, album) = match tags {
            Tags::Full => ("Fixture", "multiplay", Some("Golden Fixtures")),
            Tags::NoAlbum => ("Fixture", "multiplay", None),
            Tags::ArtistAlbum => (stem.as_ref(), "multiplay", Some("Golden Fixtures")),
            Tags::NameOnly => (stem.as_ref(), "Unknown Artist", None),
        };
        if (m.title.as_str(), m.artist.as_str(), m.album.as_deref()) != (title, artist, album) {
            failures.push(format!(
                "{file}: got {:?} / {:?} / {:?}",
                m.title, m.artist, m.album
            ));
        }
        match m.duration {
            Some(d) if (d.as_secs_f64() - seconds).abs() < 0.06 => {}
            other => failures.push(format!("{file}: duration {other:?}, want {seconds}s")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_fixture_in_the_corpus_is_listed_here() {
    for entry in std::fs::read_dir(fixtures()).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let listed = FIXTURES.iter().any(|(f, ..)| *f == name)
            || [
                "flac_24_192k.flac",
                "wav_24.wav",
                "opus_mono.opus",
                "opus_5_1.opus",
            ]
            .contains(&name.as_str())
            || name.starts_with("cover");
        assert!(listed, "{name} has no tag expectations");
    }
}

#[test]
fn rich_tags_are_read_from_flac_mp3_and_m4a() {
    for ext in ["flac", "mp3", "m4a"] {
        let path = fixtures().join(format!("cover.{ext}"));
        let m = extract_track_from_file(&path).expect("track").metadata;
        assert_eq!(m.album_artist.as_deref(), Some("Various Fixtures"), "{ext}");
        assert_eq!(m.genre.as_deref(), Some("Test Tones"), "{ext}");
        assert_eq!(m.year, Some(2024), "{ext}");
        assert_eq!(m.disc_number, Some(2), "{ext}");
        assert_eq!(m.track_number, Some(3), "{ext}");
        assert!(
            m.cover_art_uri
                .as_deref()
                .is_some_and(|u| u.starts_with("embedded:")),
            "{ext}: {:?}",
            m.cover_art_uri
        );
    }
}

#[test]
fn embedded_cover_art_is_returned_as_an_image() {
    let icon = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icon/multiplay-64.png"),
    )
    .unwrap();
    for ext in ["flac", "mp3", "m4a"] {
        let art = amp_adapter_storage::embedded_cover_art(&fixtures().join(format!("cover.{ext}")))
            .unwrap_or_else(|| panic!("{ext}: no cover"));
        assert_eq!(art.mime_type, "image/png", "{ext}");
        assert_eq!(art.data, icon, "{ext}: the embedded image is the original");
    }
    assert!(amp_adapter_storage::embedded_cover_art(&fixtures().join("wav_16.wav")).is_none());
}

#[test]
fn replay_gain_tags_feed_loudness_normalization() {
    for ext in ["flac", "mp3"] {
        let m = extract_track_from_file(&fixtures().join(format!("cover.{ext}")))
            .unwrap()
            .metadata;
        // ReplayGain's reference is about -18 LUFS: a -6.5 dB gain means the track is -11.5 LUFS.
        let lufs = m.integrated_lufs().expect("loudness known from ReplayGain");
        assert!((lufs - -11.5).abs() < 0.01, "{ext}: {lufs}");
    }
}

#[test]
fn untagged_files_take_artist_and_title_from_the_file_name() {
    let dir = std::env::temp_dir().join(format!("multiplay-names-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let plain = untagged_wav();
    let named = dir.join("Daft Punk - One More Time.wav");
    std::fs::write(&named, &plain).unwrap();
    let m = extract_track_from_file(&named).expect("track").metadata;
    assert_eq!(
        (m.artist.as_str(), m.title.as_str()),
        ("Daft Punk", "One More Time")
    );
    let bare = dir.join("05 Interlude.wav");
    std::fs::write(&bare, &plain).unwrap();
    let m = extract_track_from_file(&bare).expect("track").metadata;
    assert_eq!(m.title, "05 Interlude");
    let _ = std::fs::remove_dir_all(dir);
}

/// A 0.1 s, 8 kHz mono 16-bit WAV without any tags.
fn untagged_wav() -> Vec<u8> {
    let data: Vec<u8> = (0..800i16).flat_map(|i| (i * 20).to_le_bytes()).collect();
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&[1, 0, 1, 0]);
    out.extend_from_slice(&8000u32.to_le_bytes());
    out.extend_from_slice(&16000u32.to_le_bytes());
    out.extend_from_slice(&[2, 0, 16, 0]);
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}
