//! Cross-source search ranking and deduplication (`TASK-155`, `SRC-07`).
//!
//! Normalizes artist and title strings (stripping feature tags, remaster labels,
//! official video tags), groups duplicate tracks, and ranks candidate sources
//! favoring local files > lossless/high-bitrate > streaming providers.

#![forbid(unsafe_code)]

use amp_domain::{AudioSource, Track};
use std::collections::HashMap;

/// Normalizes a track or artist string for fuzzy matching and deduplication.
///
/// Strips noise such as `(feat. ...)`, `[remastered]`, `(official video)`, etc.
pub fn normalize_string(input: &str) -> String {
    let lower = input.to_lowercase();
    let mut cleaned = String::with_capacity(lower.len());

    let mut in_paren = false;
    let mut in_bracket = false;

    // Filter out parenthetical noise that frequently denotes remasters or video metadata
    for ch in lower.chars() {
        match ch {
            '(' => in_paren = true,
            ')' => in_paren = false,
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            _ if in_paren || in_bracket => {
                // Skip content in parentheses / brackets
            }
            _ if ch.is_alphanumeric() || ch.is_whitespace() => {
                cleaned.push(ch);
            }
            _ => {
                // Replace punctuation with space
                cleaned.push(' ');
            }
        }
    }

    // Collapse multiple consecutive spaces and trim
    let mut words = Vec::new();
    for word in cleaned.split_whitespace() {
        match word {
            "feat" | "ft" | "featuring" | "remaster" | "remastered" | "official" | "video"
            | "audio" | "hq" | "hd" | "lyrics" | "deluxe" | "version" => {}
            w => words.push(w),
        }
    }

    words.join(" ")
}

/// Evaluates a quality/preference score for a track candidate (higher is better).
///
/// Hierarchy: Local file (1000) > Bandcamp (500) > YouTube (400) > Spotify (300) > P2P/Other (100)
/// Lossless audio receives a +200 boost; higher bitrates receive an additional bonus.
pub fn score_track(track: &Track) -> u32 {
    let source_score = match &track.source {
        AudioSource::LocalFile { .. } => 1000,
        AudioSource::Bandcamp { .. } => 500,
        AudioSource::YouTube { .. } => 400,
        AudioSource::Spotify { .. } => 300,
        AudioSource::Soulseek { .. } => 200,
        AudioSource::P2PShared { .. } => 150,
        AudioSource::DirectHttp { .. } => 100,
        AudioSource::NetworkSmb { .. } => 800,
        AudioSource::SoundCloud { .. } => 350,
    };

    let mut quality_bonus = 0;
    if let Some(bps) = track.metadata.bit_depth {
        if bps > 16 {
            quality_bonus += 100;
        }
    }
    if let Some(rate) = track.metadata.sample_rate {
        if rate > 44100 {
            quality_bonus += 100;
        }
    }
    if let Some(kbps) = track.metadata.bitrate_kbps {
        quality_bonus += (kbps / 10).min(50);
    }

    source_score + quality_bonus
}

/// Deduplicates and ranks multi-provider search results.
///
/// Duplicate tracks across providers (e.g. YouTube video vs local FLAC vs Bandcamp)
/// are grouped by normalized `(artist, title)` signature. The highest-scoring candidate
/// is retained and prioritized.
pub fn rank_and_deduplicate(tracks: Vec<Track>) -> Vec<Track> {
    let mut groups: HashMap<(String, String), Vec<Track>> = HashMap::new();

    for track in tracks {
        let artist_norm = normalize_string(&track.metadata.artist);
        let title_norm = normalize_string(&track.metadata.title);
        let key = (artist_norm, title_norm);

        groups.entry(key).or_default().push(track);
    }

    let mut ranked_tracks = Vec::new();

    for (_key, mut candidates) in groups {
        // Sort candidates within group descending by quality score
        candidates.sort_by_key(|a| std::cmp::Reverse(score_track(a)));
        if let Some(best) = candidates.into_iter().next() {
            ranked_tracks.push(best);
        }
    }

    // Sort final results descending by score
    ranked_tracks.sort_by_key(|a| std::cmp::Reverse(score_track(a)));
    ranked_tracks
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::TrackMetadata;
    use std::time::Duration;

    #[test]
    fn test_string_normalization() {
        assert_eq!(normalize_string("Aphex Twin (feat. Someone)"), "aphex twin");
        assert_eq!(
            normalize_string("Windowlicker [Remastered 2021]"),
            "windowlicker"
        );
        assert_eq!(
            normalize_string("Windowlicker (Official Music Video) [HD]"),
            "windowlicker"
        );
        assert_eq!(
            normalize_string("Boards of Canada - Music Is Math (Deluxe Version)"),
            "boards of canada music is math"
        );
    }

    #[test]
    fn test_golden_cross_source_ranking_and_deduplication() {
        let mut meta_local = TrackMetadata::new("Windowlicker", "Aphex Twin");
        meta_local.bit_depth = Some(24);
        meta_local.sample_rate = Some(96000);
        meta_local.bitrate_kbps = Some(2400);
        let local_track = Track::new(
            AudioSource::LocalFile {
                path: "/music/aphex_windowlicker.flac".into(),
            },
            meta_local,
        );

        let mut meta_bc = TrackMetadata::new("Windowlicker (Original Mix)", "Aphex Twin");
        meta_bc.bitrate_kbps = Some(320);
        let bc_track = Track::new(
            AudioSource::Bandcamp {
                stream_url: "https://aphex.bandcamp.com/track/windowlicker".into(),
            },
            meta_bc,
        );

        let mut meta_yt = TrackMetadata::new("Windowlicker [Official Video]", "Aphex Twin");
        meta_yt.duration = Some(Duration::from_secs(365));
        let yt_track = Track::new(
            AudioSource::YouTube {
                video_id: "dQw4w9WgXcQ".into(),
            },
            meta_yt,
        );

        let other_track = Track::new(
            AudioSource::YouTube {
                video_id: "other_id".into(),
            },
            TrackMetadata::new("Selected Ambient Works", "Aphex Twin"),
        );

        let input = vec![yt_track, bc_track, local_track, other_track];
        let ranked = rank_and_deduplicate(input);

        // Deduplication collapses the 3 "Windowlicker" tracks into the best one (Local 24/96 FLAC)
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].metadata.title, "Windowlicker");
        assert!(matches!(ranked[0].source, AudioSource::LocalFile { .. }));
        assert_eq!(ranked[1].metadata.title, "Selected Ambient Works");
    }
}
