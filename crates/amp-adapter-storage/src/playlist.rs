//! Session playlist persistence and lossless M3U/M3U8 (relative paths), PLS, and XSPF I/O (`TASK-152`).

#![forbid(unsafe_code)]

use amp_domain::{AudioSource, DeckAssignment, Track, TrackId, TrackMetadata};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::paths::AppPaths;

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
    /// Name of the phone that requested each track (TASK-216). Missing in older files.
    #[serde(default)]
    pub requested_by: HashMap<TrackId, String>,
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
            requested_by: HashMap::new(),
        }
    }
}

impl SessionPlaylistSnapshot {
    /// Returns the default session storage path resolved through [`AppPaths`].
    pub fn default_session_path() -> PathBuf {
        let paths = AppPaths::from_env();
        let _ = paths.ensure_dirs();
        paths.session_playlist_file()
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

/// Encodes an [`AudioSource`] into a playlist entry string, writing a relative path when
/// `base_dir` is provided and `source` is a `LocalFile` inside `base_dir` (`TASK-152`).
pub fn format_playlist_locator(source: &AudioSource, base_dir: Option<&Path>) -> String {
    match source {
        AudioSource::LocalFile { path } => {
            if let Some(base) = base_dir {
                if let Ok(rel) = Path::new(path).strip_prefix(base) {
                    let rel_str = rel.to_string_lossy();
                    if !rel_str.is_empty() && !rel_str.starts_with('#') {
                        return rel_str.into_owned();
                    }
                }
            }
            path.clone()
        }
        AudioSource::DirectHttp { url } => url.clone(),
        AudioSource::YouTube { video_id } => {
            if let Some(yt) = amp_domain::parse_youtube_url(video_id) {
                format!("https://www.youtube.com/watch?v={}", yt.video_id)
            } else if amp_domain::is_valid_youtube_id(video_id) {
                format!("https://www.youtube.com/watch?v={video_id}")
            } else {
                format!("multiplay:youtube:{video_id}")
            }
        }
        AudioSource::Spotify { track_uri } => format!("spotify:track:{track_uri}"),
        AudioSource::Bandcamp { stream_url }
            if stream_url.contains("bandcamp.com") && stream_url.starts_with("http") =>
        {
            stream_url.clone()
        }
        AudioSource::SoundCloud { track_url }
            if track_url.contains("soundcloud.com") && track_url.starts_with("http") =>
        {
            track_url.clone()
        }
        AudioSource::NetworkSmb { share_url } if share_url.starts_with("smb://") => {
            share_url.clone()
        }
        _ => source.canonical_uri(),
    }
}

/// Parses a playlist locator line back into an [`AudioSource`], resolving relative local file paths
/// against `base_dir` when provided (`TASK-152`).
pub fn parse_playlist_locator(locator: &str, base_dir: Option<&Path>) -> AudioSource {
    let trimmed = locator.trim();
    if let Some(src) = AudioSource::from_canonical_uri(trimmed) {
        return src;
    }
    if trimmed.starts_with("smb://") {
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
        if let Some(yt) = amp_domain::parse_youtube_url(trimmed) {
            AudioSource::YouTube {
                video_id: yt.video_id,
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
        let path_obj = Path::new(trimmed);
        let resolved = match (base_dir, path_obj.is_relative()) {
            (Some(base), true) => join_locator(base, trimmed),
            _ => trimmed.to_string(),
        };
        AudioSource::LocalFile { path: resolved }
    }
}

/// `rel` resolved against `base`. A base written with `/` (as on Unix, or a Windows path with
/// forward slashes) is joined with `/`, so the result matches the path the playlist was
/// exported from; `Path::join` would insert `\\` on Windows and change the track's identity.
fn join_locator(base: &Path, rel: &str) -> String {
    let b = base.to_string_lossy();
    if b.contains('/') && !b.contains('\\') {
        format!("{}/{rel}", b.trim_end_matches('/'))
    } else {
        base.join(rel).to_string_lossy().into_owned()
    }
}

/// Exports playlist to standard UTF-8 M3U8 string with `#EXTINF` metadata tags.
pub fn export_m3u8(tracks: &[Track]) -> String {
    export_m3u8_with_base(tracks, None)
}

/// Exports playlist to M3U8, writing relative paths for local files inside `base_dir`.
pub fn export_m3u8_with_base(tracks: &[Track], base_dir: Option<&Path>) -> String {
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
        out.push_str(&format_playlist_locator(&track.source, base_dir));
        out.push('\n');
    }
    out
}

/// Imports playlist tracks from standard M3U/M3U8 string, deduplicating by canonical source URI.
pub fn import_m3u8(content: &str) -> Vec<Track> {
    import_m3u8_with_base(content, None)
}

/// Imports playlist tracks from M3U/M3U8 resolving relative entries against `base_dir` and deduplicating.
pub fn import_m3u8_with_base(content: &str, base_dir: Option<&Path>) -> Vec<Track> {
    let mut tracks = Vec::new();
    let mut seen_sources = HashSet::new();
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
            if let Some(secs) = dur_sec.filter(|&s| s > 0) {
                meta.duration = Some(Duration::from_secs(secs as u64));
            }
            pending_meta = Some(meta);
        } else if !trimmed.starts_with('#') {
            let meta = pending_meta.take().unwrap_or_else(|| {
                let filename = Path::new(trimmed)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(trimmed);
                TrackMetadata::new(filename, "Unknown Artist")
            });
            let source = parse_playlist_locator(trimmed, base_dir);
            if seen_sources.insert(source.canonical_uri()) {
                tracks.push(Track::with_stable_id(source, meta));
            }
        }
    }
    tracks
}

/// Exports playlist to PLS format (`[playlist]`, `File1=...`, `Title1=...`, `Length1=...`).
pub fn export_pls(tracks: &[Track], base_dir: Option<&Path>) -> String {
    let mut out = format!("[playlist]\nNumberOfEntries={}\n", tracks.len());
    for (idx, track) in tracks.iter().enumerate() {
        let n = idx + 1;
        let loc = format_playlist_locator(&track.source, base_dir);
        let len = track
            .metadata
            .duration
            .map(|d| d.as_secs() as i64)
            .unwrap_or(-1);
        out.push_str(&format!(
            "File{n}={loc}\nTitle{n}={} - {}\nLength{n}={len}\n",
            track.metadata.artist, track.metadata.title
        ));
    }
    out.push_str("Version=2\n");
    out
}

///Imports playlist tracks from a PLS document, deduplicating by canonical source.
pub fn import_pls(content: &str, base_dir: Option<&Path>) -> Vec<Track> {
    let mut files: HashMap<usize, String> = HashMap::new();
    let mut titles: HashMap<usize, String> = HashMap::new();
    let mut lengths: HashMap<usize, i64> = HashMap::new();

    for raw in content.lines() {
        let Some((k, v)) = raw.trim().split_once('=') else {
            continue;
        };
        let (key, val) = (k.trim(), v.trim());
        let parse_idx = |p1, p2| {
            key.strip_prefix(p1)
                .or_else(|| key.strip_prefix(p2))?
                .parse::<usize>()
                .ok()
        };
        if let Some(idx) = parse_idx("File", "file") {
            files.insert(idx, val.to_string());
        } else if let Some(idx) = parse_idx("Title", "title") {
            titles.insert(idx, val.to_string());
        } else if let (Some(idx), Ok(secs)) = (parse_idx("Length", "length"), val.parse::<i64>()) {
            lengths.insert(idx, secs);
        }
    }

    let mut indices: Vec<usize> = files.keys().copied().collect();
    indices.sort_unstable();
    let mut seen = HashSet::new();
    let mut tracks = Vec::new();
    for idx in indices {
        let Some(loc) = files.get(&idx) else {
            continue;
        };
        let source = parse_playlist_locator(loc, base_dir);
        if !seen.insert(source.canonical_uri()) {
            continue;
        }
        let title_raw = titles
            .get(&idx)
            .map_or("Unknown Artist - Unknown Title", String::as_str);
        let (artist, title) = title_raw
            .split_once(" - ")
            .unwrap_or(("Unknown Artist", title_raw));
        let mut meta = TrackMetadata::new(title.trim(), artist.trim());
        if let Some(&secs) = lengths.get(&idx).filter(|&&s| s > 0) {
            meta.duration = Some(Duration::from_secs(secs as u64));
        }
        tracks.push(Track::with_stable_id(source, meta));
    }
    tracks
}

/// Exports playlist to XML Shareable Playlist Format (`.xspf`).
pub fn export_xspf(tracks: &[Track], base_dir: Option<&Path>) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<playlist version=\"1\" xmlns=\"http://xspf.org/ns/0/\">\n  <trackList>\n",
    );
    for track in tracks {
        let loc = xml_escape(&format_playlist_locator(&track.source, base_dir));
        let title = xml_escape(&track.metadata.title);
        let creator = xml_escape(&track.metadata.artist);
        out.push_str(&format!(
            "    <track><location>{loc}</location><creator>{creator}</creator><title>{title}</title></track>\n"
        ));
    }
    out.push_str("  </trackList>\n</playlist>\n");
    out
}

/// Imports playlist tracks from an `.xspf` document, deduplicating by canonical source.
pub fn import_xspf(content: &str, base_dir: Option<&Path>) -> Vec<Track> {
    let mut tracks = Vec::new();
    let mut seen = HashSet::new();
    let mut rest = content;
    while let Some(start) = rest.find("<track>") {
        let after = &rest[start + 7..];
        let Some(end) = after.find("</track>") else {
            break;
        };
        let block = &after[..end];
        if let Some(loc) = extract_xml_tag(block, "location") {
            let source = parse_playlist_locator(&loc, base_dir);
            if seen.insert(source.canonical_uri()) {
                let title =
                    extract_xml_tag(block, "title").unwrap_or_else(|| "Unknown Title".to_string());
                let artist = extract_xml_tag(block, "creator")
                    .unwrap_or_else(|| "Unknown Artist".to_string());
                tracks.push(Track::with_stable_id(
                    source,
                    TrackMetadata::new(title, artist),
                ));
            }
        }
        rest = &after[end + 8..];
    }
    tracks
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn xml_unescape(s: &str) -> String {
    s.replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
}

fn extract_xml_tag(block: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let s = block.find(&open)? + open.len();
    let e = block[s..].find(&close)?;
    Some(xml_unescape(block[s..s + e].trim()))
}

#[cfg(test)]
#[path = "playlist_tests.rs"]
mod tests;
