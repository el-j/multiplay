//! Unit tests for `playlist.rs` (kept apart to respect the file length limit).
use super::*;

fn sample_all_source_variants() -> Vec<Track> {
    [
        "multiplay:file:/music/albums/aphex/xtal.flac",
        "multiplay:smb:smb://nas.local/music/track.wav",
        "multiplay:http:https://somafm.com/groovesalad130.pls",
        "multiplay:youtube:dQw4w9WgXcQ",
        "multiplay:spotify:4uLU6hMCjMI75M1A2tKUQC",
        "multiplay:bandcamp:https://artist.bandcamp.com/track/tune",
        "multiplay:soundcloud:https://soundcloud.com/artist/tune",
        "slsk://audiophile42/@music/Aphex Twin/01 - Xtal.flac",
        "p2p://12D3KooWPeerNode/blake3_deadbeef1234",
    ]
    .into_iter()
    .enumerate()
    .map(|(i, loc)| {
        let src = parse_playlist_locator(loc, None);
        let mut meta = TrackMetadata::new(format!("Title {i}"), format!("Artist {i}"));
        meta.duration = Some(Duration::from_secs(180 + i as u64));
        Track::with_stable_id(src, meta)
    })
    .collect()
}

/// A relative entry keeps the base folder's separator style, so a round-trip reproduces
/// the original path string (and track id) on every platform.
#[test]
fn relative_entries_join_in_the_base_folders_style() {
    assert_eq!(
        join_locator(Path::new("/music/albums"), "aphex/xtal.flac"),
        "/music/albums/aphex/xtal.flac"
    );
    assert_eq!(
        join_locator(Path::new("/music/albums/"), "x.flac"),
        "/music/albums/x.flac"
    );
    assert_eq!(
        join_locator(Path::new("C:/Music"), "a/b.flac"),
        "C:/Music/a/b.flac"
    );
}

#[test]
fn playlist_roundtrip_all_audio_source_variants_across_m3u8_pls_and_xspf() {
    let base_dir = Path::new("/music/albums");
    let tracks = sample_all_source_variants();
    let expected: Vec<AudioSource> = tracks.iter().map(|t| t.source.clone()).collect();

    let m3u = export_m3u8_with_base(&tracks, Some(base_dir));
    assert!(m3u.contains("aphex/xtal.flac") && !m3u.contains("/music/albums/aphex/xtal.flac"));
    let from_m3u: Vec<_> = import_m3u8_with_base(&m3u, Some(base_dir))
        .into_iter()
        .map(|t| t.source)
        .collect();
    assert_eq!(from_m3u, expected);

    let mut with_dupes = tracks.clone();
    with_dupes.extend(tracks.clone());
    let pls = export_pls(&with_dupes, Some(base_dir));
    let from_pls: Vec<_> = import_pls(&pls, Some(base_dir))
        .into_iter()
        .map(|t| t.source)
        .collect();
    assert_eq!(from_pls, expected);

    let xspf = export_xspf(&tracks, Some(base_dir));
    let from_xspf: Vec<_> = import_xspf(&xspf, Some(base_dir))
        .into_iter()
        .map(|t| t.source)
        .collect();
    assert_eq!(from_xspf, expected);
}

#[test]
fn test_session_playlist_snapshot_save_and_load() {
    let temp_dir = std::env::temp_dir().join(format!("multiplay_session_{}", uuid::Uuid::new_v4()));
    let session_file = temp_dir.join("test_session.json");
    let mut snapshot = SessionPlaylistSnapshot {
        volume: 0.92,
        ..Default::default()
    };
    snapshot.eq_gains[3] = 4.5;
    snapshot.tracks.push(Track::new(
        AudioSource::LocalFile {
            path: "/music/song.mp3".into(),
        },
        TrackMetadata::new("Song", "Artist"),
    ));
    snapshot.save_to_file(&session_file).unwrap();
    let loaded = SessionPlaylistSnapshot::load_from_file(&session_file).unwrap();
    assert_eq!(
        (loaded.volume, loaded.eq_gains[3], loaded.tracks.len()),
        (0.92, 4.5, 1)
    );
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_youtube_url_normalization_on_playlist_import() {
    let m3u = "#EXTM3U\n#EXTINF:180,Artist - Title\nhttps://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42s\n#EXTINF:200,B - T2\nhttps://youtu.be/9bZkp7q19f0\n#EXTINF:180,Duplicate - Song\nhttps://youtu.be/dQw4w9WgXcQ\n";
    let tracks = import_m3u8(m3u);
    assert_eq!(tracks.len(), 2, "Duplicate video ID normalized and deduped");
    assert_eq!(
        tracks[0].source,
        AudioSource::YouTube {
            video_id: "dQw4w9WgXcQ".to_string()
        }
    );
    assert_eq!(
        tracks[1].source,
        AudioSource::YouTube {
            video_id: "9bZkp7q19f0".to_string()
        }
    );
}
