//! Source URL and ID Normalization parser for Multiplay (`TASK-156`, `SRC-08`).
//!
//! Robustly extracts canonical IDs and parameters from various URL formats
//! (YouTube watch, youtu.be, shorts, music, embeds, playlists, and timestamps;
//! Bandcamp tracks and albums; Spotify URIs).

#![forbid(unsafe_code)]

use std::time::Duration;

/// Validates whether a string slice looks like a standard 11-char YouTube video ID.
pub fn is_valid_youtube_id(id: &str) -> bool {
    id.len() == 11
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Normalized YouTube reference extracted from a URL or raw ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YouTubeReference {
    pub video_id: String,
    pub start_offset: Option<Duration>,
    pub playlist_id: Option<String>,
}

/// Categorized source URL representations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedSourceUrl {
    YouTube(YouTubeReference),
    YouTubePlaylist { playlist_id: String },
    BandcampTrack { url: String },
    BandcampAlbum { url: String },
    SpotifyTrack { uri: String },
    DirectHttp { url: String },
}

/// Parses a time string (e.g. "42", "42s", "1m30s", "1h2m3s") into a [`Duration`].
pub fn parse_time_param(param: &str) -> Option<Duration> {
    let s = param.trim();
    if s.is_empty() {
        return None;
    }

    // Pure numeric seconds (e.g. "42", "120")
    if let Ok(secs) = s.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }

    // Single number with trailing 's' (e.g. "42s")
    if let Some(num) = s.strip_suffix('s') {
        if let Ok(secs) = num.parse::<u64>() {
            return Some(Duration::from_secs(secs));
        }
    }

    // Component-based: e.g. "1h2m3s", "2m30s", "90s"
    let mut total_secs: u64 = 0;
    let mut current_num: u64 = 0;
    let mut has_digit = false;
    let mut parsed_any = false;

    for ch in s.chars() {
        if let Some(digit) = ch.to_digit(10) {
            current_num = current_num.saturating_mul(10).saturating_add(digit as u64);
            has_digit = true;
        } else {
            match ch {
                'h' | 'H' if has_digit => {
                    total_secs = total_secs.saturating_add(current_num.saturating_mul(3600));
                    current_num = 0;
                    has_digit = false;
                    parsed_any = true;
                }
                'm' | 'M' if has_digit => {
                    total_secs = total_secs.saturating_add(current_num.saturating_mul(60));
                    current_num = 0;
                    has_digit = false;
                    parsed_any = true;
                }
                's' | 'S' if has_digit => {
                    total_secs = total_secs.saturating_add(current_num);
                    current_num = 0;
                    has_digit = false;
                    parsed_any = true;
                }
                _ => return None,
            }
        }
    }

    if has_digit {
        total_secs = total_secs.saturating_add(current_num);
        parsed_any = true;
    }

    if parsed_any {
        Some(Duration::from_secs(total_secs))
    } else {
        None
    }
}

/// Normalizes and extracts YouTube video reference from any supported URL form or raw ID.
pub fn parse_youtube_url(input: &str) -> Option<YouTubeReference> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }

    // 1. Raw 11-char ID
    if is_valid_youtube_id(s) {
        return Some(YouTubeReference {
            video_id: s.to_string(),
            start_offset: None,
            playlist_id: None,
        });
    }

    let url_str = if !s.contains("://") {
        format!("https://{s}")
    } else {
        s.to_string()
    };

    let (path_and_host, query) = match url_str.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (url_str.as_str(), None),
    };

    let mut video_id: Option<String> = None;
    let mut playlist_id: Option<String> = None;
    let mut start_offset: Option<Duration> = None;

    // Parse query parameters
    if let Some(q) = query {
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                match k {
                    "v" if is_valid_youtube_id(v) => video_id = Some(v.to_string()),
                    "list" if !v.is_empty() => playlist_id = Some(v.to_string()),
                    "t" | "start" => {
                        if let Some(dur) = parse_time_param(v) {
                            start_offset = Some(dur);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Path-based extraction if video_id not yet found in query
    if video_id.is_none() {
        let segments: Vec<&str> = path_and_host
            .split('/')
            .filter(|seg| !seg.is_empty())
            .collect();

        // Check youtu.be/<id>
        if path_and_host.contains("youtu.be") {
            if let Some(&id) = segments.last() {
                if is_valid_youtube_id(id) {
                    video_id = Some(id.to_string());
                }
            }
        } else {
            // Check /shorts/<id>, /embed/<id>, /v/<id>, /live/<id>
            for (i, &seg) in segments.iter().enumerate() {
                if matches!(seg, "shorts" | "embed" | "v" | "live") {
                    if let Some(&id) = segments.get(i + 1) {
                        if is_valid_youtube_id(id) {
                            video_id = Some(id.to_string());
                            break;
                        }
                    }
                }
            }
        }
    }

    video_id.map(|vid| YouTubeReference {
        video_id: vid,
        start_offset,
        playlist_id,
    })
}

/// Parses an arbitrary input URL or identifier into a canonical [`ParsedSourceUrl`].
pub fn parse_source_url(input: &str) -> ParsedSourceUrl {
    let s = input.trim();

    // 1. YouTube Reference
    if let Some(yt) = parse_youtube_url(s) {
        return ParsedSourceUrl::YouTube(yt);
    }

    // 2. YouTube Playlist Only (no video id)
    if s.contains("youtube.com") && s.contains("list=") {
        if let Some(q) = s.split_once('?').map(|(_, q)| q) {
            for pair in q.split('&') {
                if let Some((k, v)) = pair.split_once('=') {
                    if k == "list" && !v.is_empty() {
                        return ParsedSourceUrl::YouTubePlaylist {
                            playlist_id: v.to_string(),
                        };
                    }
                }
            }
        }
    }

    // 3. Bandcamp URL
    if s.contains("bandcamp.com") {
        if s.contains("/album/") {
            return ParsedSourceUrl::BandcampAlbum { url: s.to_string() };
        }
        return ParsedSourceUrl::BandcampTrack { url: s.to_string() };
    }

    // 4. Spotify Track URI or Link
    if s.contains("spotify.com") || s.starts_with("spotify:track:") {
        return ParsedSourceUrl::SpotifyTrack { uri: s.to_string() };
    }

    // 5. Default: Direct HTTP Stream
    ParsedSourceUrl::DirectHttp { url: s.to_string() }
}

/// True for text that is a link rather than search words: an http(s) URL, or a YouTube,
/// Bandcamp or Spotify address pasted without the scheme.
#[must_use]
pub fn looks_like_link(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() || t.contains(char::is_whitespace) {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("spotify:track:")
        || [
            "youtube.com/",
            "youtu.be/",
            ".bandcamp.com/",
            "open.spotify.com/",
        ]
        .iter()
        .any(|host| lower.contains(host))
}

/// Why a link cannot be added, for links multiplay recognises but does not play: Spotify was
/// dropped (ADR-007). `None` for every other link.
#[must_use]
pub fn unsupported_link(input: &str) -> Option<&'static str> {
    matches!(
        parse_source_url(input.trim()),
        ParsedSourceUrl::SpotifyTrack { .. }
    )
    .then_some("Spotify links can't be played: multiplay does not support Spotify.")
}

/// The playlist entry for a pasted link, or `None` when `input` is not a link. The title is a
/// placeholder until the provider lookup fills in the real one.
#[must_use]
pub fn track_from_link(input: &str) -> Option<crate::Track> {
    use crate::AudioSource;
    let trimmed = input.trim();
    if !looks_like_link(trimmed) || unsupported_link(trimmed).is_some() {
        return None;
    }
    let (source, title) = match parse_source_url(trimmed) {
        ParsedSourceUrl::YouTube(yt) => (
            AudioSource::YouTube {
                video_id: yt.video_id,
            },
            "YouTube Stream",
        ),
        ParsedSourceUrl::BandcampTrack { url } | ParsedSourceUrl::BandcampAlbum { url } => {
            (AudioSource::Bandcamp { stream_url: url }, "Bandcamp Stream")
        }
        ParsedSourceUrl::SpotifyTrack { uri } => {
            (AudioSource::Spotify { track_uri: uri }, "Spotify Track")
        }
        ParsedSourceUrl::DirectHttp { url } => {
            (AudioSource::DirectHttp { url }, "Direct HTTP Stream")
        }
        ParsedSourceUrl::YouTubePlaylist { playlist_id } => (
            AudioSource::DirectHttp {
                url: format!("https://www.youtube.com/playlist?list={playlist_id}"),
            },
            "YouTube Playlist",
        ),
    };
    Some(crate::Track::new(
        source,
        crate::TrackMetadata::new(title, "Network Source"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_driven_youtube_normalization_15_shapes() {
        struct TestCase {
            input: &'static str,
            expected_id: &'static str,
            expected_offset_secs: Option<u64>,
            expected_playlist: Option<&'static str>,
        }

        let cases = [
            // 1. Standard watch URL
            TestCase {
                input: "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: None,
                expected_playlist: None,
            },
            // 2. HTTP watch URL
            TestCase {
                input: "http://youtube.com/watch?v=dQw4w9WgXcQ",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: None,
                expected_playlist: None,
            },
            // 3. Watch URL with additional parameters
            TestCase {
                input: "https://www.youtube.com/watch?v=dQw4w9WgXcQ&feature=share",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: None,
                expected_playlist: None,
            },
            // 4. Watch URL with numeric seconds timestamp
            TestCase {
                input: "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: Some(42),
                expected_playlist: None,
            },
            // 5. Watch URL with 's' suffix timestamp
            TestCase {
                input: "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=42s",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: Some(42),
                expected_playlist: None,
            },
            // 6. Watch URL with minute/second timestamp
            TestCase {
                input: "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=1m30s",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: Some(90),
                expected_playlist: None,
            },
            // 7. Watch URL with hour/min/sec timestamp
            TestCase {
                input: "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=1h2m3s",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: Some(3723),
                expected_playlist: None,
            },
            // 8. youtu.be short URL
            TestCase {
                input: "https://youtu.be/dQw4w9WgXcQ",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: None,
                expected_playlist: None,
            },
            // 9. youtu.be with timestamp
            TestCase {
                input: "https://youtu.be/dQw4w9WgXcQ?t=15",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: Some(15),
                expected_playlist: None,
            },
            // 10. YouTube Shorts
            TestCase {
                input: "https://www.youtube.com/shorts/dQw4w9WgXcQ",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: None,
                expected_playlist: None,
            },
            // 11. YouTube Music
            TestCase {
                input: "https://music.youtube.com/watch?v=dQw4w9WgXcQ",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: None,
                expected_playlist: None,
            },
            // 12. Mobile URL
            TestCase {
                input: "https://m.youtube.com/watch?v=dQw4w9WgXcQ",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: None,
                expected_playlist: None,
            },
            // 13. Embed URL
            TestCase {
                input: "https://www.youtube.com/embed/dQw4w9WgXcQ",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: None,
                expected_playlist: None,
            },
            // 14. Live stream URL
            TestCase {
                input: "https://www.youtube.com/live/dQw4w9WgXcQ",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: None,
                expected_playlist: None,
            },
            // 15. Raw 11-char ID
            TestCase {
                input: "dQw4w9WgXcQ",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: None,
                expected_playlist: None,
            },
            // 16. Watch URL with embedded playlist
            TestCase {
                input: "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PL1234567890",
                expected_id: "dQw4w9WgXcQ",
                expected_offset_secs: None,
                expected_playlist: Some("PL1234567890"),
            },
        ];

        for (idx, tc) in cases.iter().enumerate() {
            let res = parse_youtube_url(tc.input)
                .unwrap_or_else(|| panic!("Case {idx} failed to parse: {}", tc.input));
            assert_eq!(res.video_id, tc.expected_id, "Case {idx} video_id mismatch");
            assert_eq!(
                res.start_offset.map(|d| d.as_secs()),
                tc.expected_offset_secs,
                "Case {idx} offset mismatch"
            );
            assert_eq!(
                res.playlist_id.as_deref(),
                tc.expected_playlist,
                "Case {idx} playlist mismatch"
            );
        }
    }

    #[test]
    fn test_parse_source_url_multiprovider() {
        // YouTube
        let yt = parse_source_url("https://youtu.be/dQw4w9WgXcQ");
        assert!(matches!(yt, ParsedSourceUrl::YouTube(ref r) if r.video_id == "dQw4w9WgXcQ"));

        // Bandcamp Track
        let bc_track = parse_source_url("https://aphextwin.bandcamp.com/track/windowlicker");
        assert!(matches!(bc_track, ParsedSourceUrl::BandcampTrack { .. }));

        // Bandcamp Album
        let bc_album =
            parse_source_url("https://aphextwin.bandcamp.com/album/selected-ambient-works-85-92");
        assert!(matches!(bc_album, ParsedSourceUrl::BandcampAlbum { .. }));

        // Spotify Track
        let spot = parse_source_url("https://open.spotify.com/track/4cOdK2wGLETKBW3PvgPWqT");
        assert!(matches!(spot, ParsedSourceUrl::SpotifyTrack { .. }));

        // Direct HTTP
        let http = parse_source_url("https://example.com/audio/stream.mp3");
        assert!(matches!(http, ParsedSourceUrl::DirectHttp { .. }));
    }
}
