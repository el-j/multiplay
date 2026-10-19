//! Session playlist persistence and standard M3U/M3U8 import/export.

#![forbid(unsafe_code)]

use amp_domain::{AudioSource, DeckAssignment, Track, TrackId, TrackMetadata};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Serializable snapshot of user playlist and playback state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionPlaylistSnapshot {
    pub tracks: Vec<Track>,
    pub selected_idx: usize,
    pub volume: f32,
    pub eq_preamp: f32,
    pub eq_gains: [f32; 10],
    pub eq_enabled: bool,
    /// DJ deck of each track (TASK-210). Missing in older files.
    #[serde(default)]
    pub deck_assignments: HashMap<TrackId, DeckAssignment>,
    /// The DJ first-run hint was dismissed. Missing in older files.
    #[serde(default)]
    pub dj_hint_done: bool,
}

impl Default for SessionPlaylistSnapshot {
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            selected_idx: 0,
            volume: 0.85,
            eq_preamp: 0.0,
            eq_gains: [0.0; 10],
            eq_enabled: true,
            deck_assignments: HashMap::new(),
            dj_hint_done: false,
        }
    }
}

impl SessionPlaylistSnapshot {
    /// Returns default session storage path (`~/.multiplay/session_playlist.json`).
    pub fn default_session_path() -> PathBuf {
        let base_dir = dirs_or_home_dir().join(".multiplay");
        let _ = fs::create_dir_all(&base_dir);
        base_dir.join("session_playlist.json")
    }

    /// Atomically saves snapshot to disk.
    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Serialization failure: {e}"))?;

        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, json.as_bytes())
            .map_err(|e| format!("Failed to write session file: {e}"))?;
        fs::rename(&temp_path, path).map_err(|e| format!("Failed to commit session file: {e}"))?;
        Ok(())
    }

    /// Loads session snapshot from disk.
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read session file: {e}"))?;
        let snapshot: Self =
            serde_json::from_str(&content).map_err(|e| format!("Invalid session format: {e}"))?;
        Ok(snapshot)
    }
}

/// Exports playlist to standard UTF-8 M3U8 string with EXTINF metadata tags.
pub fn export_m3u8(tracks: &[Track]) -> String {
    let mut out = String::from("#EXTM3U\n");
    for track in tracks {
        let duration_secs = track
            .metadata
            .duration
            .map(|d| d.as_secs() as i64)
            .unwrap_or(-1);
        let artist = &track.metadata.artist;
        let title = &track.metadata.title;

        out.push_str(&format!("#EXTINF:{duration_secs},{artist} - {title}\n"));
        match &track.source {
            AudioSource::LocalFile { path } => {
                out.push_str(path);
                out.push('\n');
            }
            AudioSource::DirectHttp { url } => {
                out.push_str(url);
                out.push('\n');
            }
            AudioSource::YouTube { video_id } => {
                out.push_str(&format!("https://www.youtube.com/watch?v={video_id}\n"));
            }
            AudioSource::Spotify { track_uri } => {
                out.push_str(&format!("spotify:track:{track_uri}\n"));
            }
            AudioSource::Soulseek { username, filename } => {
                out.push_str(&format!("slsk://{username}/{filename}\n"));
            }
            AudioSource::NetworkSmb { share_url } => {
                out.push_str(share_url);
                out.push('\n');
            }
            AudioSource::Bandcamp { stream_url } => {
                out.push_str(stream_url);
                out.push('\n');
            }
            AudioSource::SoundCloud { track_url } => {
                out.push_str(track_url);
                out.push('\n');
            }
            AudioSource::P2PShared { peer_id, file_hash } => {
                out.push_str(&format!("p2p://{peer_id}/{file_hash}\n"));
            }
        }
    }
    out
}

/// Imports playlist tracks from standard M3U/M3U8 string.
pub fn import_m3u8(content: &str) -> Vec<Track> {
    let mut tracks = Vec::new();
    let mut pending_meta: Option<TrackMetadata> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(inf) = trimmed.strip_prefix("#EXTINF:") {
            let mut parts = inf.splitn(2, ',');
            let dur_sec = parts.next().and_then(|s| s.parse::<i64>().ok());
            let title_part = parts.next().unwrap_or("Unknown Title");

            let (artist, title) = if let Some((a, t)) = title_part.split_once(" - ") {
                (a.trim(), t.trim())
            } else {
                ("Unknown Artist", title_part.trim())
            };

            let mut meta = TrackMetadata::new(title, artist);
            if let Some(secs) = dur_sec {
                if secs > 0 {
                    meta.duration = Some(Duration::from_secs(secs as u64));
                }
            }
            pending_meta = Some(meta);
        } else if !trimmed.starts_with('#') {
            // Path or URI line
            let meta = pending_meta.take().unwrap_or_else(|| {
                let filename = Path::new(trimmed)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(trimmed);
                TrackMetadata::new(filename, "Unknown Artist")
            });

            let source = if trimmed.starts_with("smb://") {
                AudioSource::NetworkSmb {
                    share_url: trimmed.to_string(),
                }
            } else if let Some(p2p_rest) = trimmed.strip_prefix("p2p://") {
                let (peer, hash) = p2p_rest.split_once('/').unwrap_or((p2p_rest, ""));
                AudioSource::P2PShared {
                    peer_id: peer.to_string(),
                    file_hash: hash.to_string(),
                }
            } else if let Some(slsk_rest) = trimmed.strip_prefix("slsk://") {
                let (user, file) = slsk_rest.split_once('/').unwrap_or((slsk_rest, ""));
                AudioSource::Soulseek {
                    username: user.to_string(),
                    filename: file.to_string(),
                }
            } else if let Some(uri) = trimmed.strip_prefix("spotify:track:") {
                AudioSource::Spotify {
                    track_uri: uri.to_string(),
                }
            } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                if trimmed.contains("youtube.com") || trimmed.contains("youtu.be") {
                    AudioSource::YouTube {
                        video_id: trimmed.to_string(),
                    }
                } else if trimmed.contains("bandcamp.com") {
                    AudioSource::Bandcamp {
                        stream_url: trimmed.to_string(),
                    }
                } else if trimmed.contains("soundcloud.com") {
                    AudioSource::SoundCloud {
                        track_url: trimmed.to_string(),
                    }
                } else {
                    AudioSource::DirectHttp {
                        url: trimmed.to_string(),
                    }
                }
            } else {
                AudioSource::LocalFile {
                    path: trimmed.to_string(),
                }
            };

            tracks.push(Track::new(source, meta));
        }
    }

    tracks
}

fn dirs_or_home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_m3u8_roundtrip_export_and_import() {
        let mut meta1 = TrackMetadata::new("Windowlicker", "Aphex Twin");
        meta1.duration = Some(Duration::from_secs(367));
        let track1 = Track::new(
            AudioSource::LocalFile {
                path: "/music/aphex.flac".to_string(),
            },
            meta1,
        );

        let mut meta2 = TrackMetadata::new("Groove Salad", "SomaFM");
        meta2.duration = None;
        let track2 = Track::new(
            AudioSource::DirectHttp {
                url: "https://somafm.com/groovesalad130.pls".to_string(),
            },
            meta2,
        );

        let tracks = vec![track1, track2];
        let exported = export_m3u8(&tracks);

        assert!(exported.contains("#EXTM3U"));
        assert!(exported.contains("Aphex Twin - Windowlicker"));
        assert!(exported.contains("/music/aphex.flac"));

        let imported = import_m3u8(&exported);
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].metadata.title, "Windowlicker");
        assert_eq!(imported[0].metadata.artist, "Aphex Twin");
        assert_eq!(
            imported[0].metadata.duration,
            Some(Duration::from_secs(367))
        );
        assert_eq!(imported[1].metadata.title, "Groove Salad");
    }

    #[test]
    fn test_session_playlist_snapshot_save_and_load() {
        let temp_dir =
            std::env::temp_dir().join(format!("multiplay_session_{}", uuid::Uuid::new_v4()));
        let session_file = temp_dir.join("test_session.json");

        let mut snapshot = SessionPlaylistSnapshot {
            volume: 0.92,
            ..Default::default()
        };
        snapshot.eq_gains[3] = 4.5;
        snapshot.tracks.push(Track::new(
            AudioSource::LocalFile {
                path: "/music/song.mp3".to_string(),
            },
            TrackMetadata::new("Song", "Artist"),
        ));

        snapshot
            .save_to_file(&session_file)
            .expect("Failed to save session");
        assert!(session_file.exists());

        let loaded =
            SessionPlaylistSnapshot::load_from_file(&session_file).expect("Failed to load session");
        assert_eq!(loaded.volume, 0.92);
        assert_eq!(loaded.eq_gains[3], 4.5);
        assert_eq!(loaded.tracks.len(), 1);
        assert_eq!(loaded.tracks[0].metadata.title, "Song");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
