//! Radio playlist parser for `.pls`, `.m3u` / `.m3u8`, and `.xspf` (`TASK-137`).

#![forbid(unsafe_code)]

/// Supported radio playlist formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioPlaylistKind {
    Pls,
    M3u,
    Xspf,
}

/// Detects whether a URL or HTTP `Content-Type` header represents a radio playlist.
pub fn detect_playlist_kind(url: &str, content_type: Option<&str>) -> Option<RadioPlaylistKind> {
    if let Some(ct) = content_type {
        let lower = ct.to_ascii_lowercase();
        let mime = lower.split(';').next().unwrap_or("").trim();
        match mime {
            "audio/x-scpls" | "application/pls+xml" => return Some(RadioPlaylistKind::Pls),
            "audio/mpegurl"
            | "audio/x-mpegurl"
            | "application/mpegurl"
            | "application/x-mpegurl"
            | "application/vnd.apple.mpegurl" => return Some(RadioPlaylistKind::M3u),
            "application/xspf+xml" => return Some(RadioPlaylistKind::Xspf),
            _ => {}
        }
    }

    let path = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    if path.ends_with(".pls") {
        Some(RadioPlaylistKind::Pls)
    } else if path.ends_with(".m3u") || path.ends_with(".m3u8") {
        Some(RadioPlaylistKind::M3u)
    } else if path.ends_with(".xspf") {
        Some(RadioPlaylistKind::Xspf)
    } else {
        None
    }
}

/// Parses a playlist body into an ordered list of candidate stream URLs.
pub fn parse_playlist_urls(kind: RadioPlaylistKind, body: &str) -> Vec<String> {
    match kind {
        RadioPlaylistKind::Pls => parse_pls(body),
        RadioPlaylistKind::M3u => parse_m3u(body),
        RadioPlaylistKind::Xspf => parse_xspf(body),
    }
}

fn parse_pls(body: &str) -> Vec<String> {
    let mut entries: Vec<(usize, String)> = Vec::new();
    for raw_line in body.lines() {
        let line = raw_line.trim();
        if line.is_empty()
            || line.starts_with(';')
            || line.starts_with('#')
            || line.starts_with('[')
        {
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            let key_trim = key.trim();
            if let Some(num_str) = key_trim
                .strip_prefix("File")
                .or_else(|| key_trim.strip_prefix("file"))
                .or_else(|| key_trim.strip_prefix("FILE"))
            {
                let idx = num_str.parse::<usize>().unwrap_or(entries.len() + 1);
                let url = val.trim();
                if !url.is_empty() {
                    entries.push((idx, url.to_string()));
                }
            }
        }
    }
    entries.sort_by_key(|(idx, _)| *idx);
    entries.into_iter().map(|(_, url)| url).collect()
}

fn parse_m3u(body: &str) -> Vec<String> {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToString::to_string)
        .collect()
}

fn parse_xspf(body: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut rest = body;
    let open_tag = "<location>";
    let close_tag = "</location>";
    while let Some(start_idx) = rest.find(open_tag) {
        let after_open = &rest[start_idx + open_tag.len()..];
        let Some(end_idx) = after_open.find(close_tag) else {
            break;
        };
        let raw_url = after_open[..end_idx].trim();
        let unescaped = raw_url
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'");
        if !unescaped.is_empty() {
            urls.push(unescaped);
        }
        rest = &after_open[end_idx + close_tag.len()..];
    }
    urls
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_somafm_pls_with_multiple_failover_servers() {
        let pls = r#"
[playlist]
numberofentries=2
File1=https://ice2.somafm.com/groovesalad-128-mp3
Title1=SomaFM: Groove Salad (#1): Ambient/downtempo
Length1=-1
File2=https://ice4.somafm.com/groovesalad-128-mp3
Title2=SomaFM: Groove Salad (#2): Ambient/downtempo
Length2=-1
Version=2
"#;
        let kind = detect_playlist_kind(
            "https://somafm.com/groovesalad.pls",
            Some("audio/x-scpls; charset=utf-8"),
        );
        assert_eq!(kind, Some(RadioPlaylistKind::Pls));
        let urls = parse_playlist_urls(kind.unwrap(), pls);
        assert_eq!(
            urls,
            vec![
                "https://ice2.somafm.com/groovesalad-128-mp3".to_string(),
                "https://ice4.somafm.com/groovesalad-128-mp3".to_string(),
            ]
        );
    }

    #[test]
    fn parses_m3u8_and_xspf_radio_playlists() {
        let m3u = "#EXTM3U\n#EXTINF:-1,Radio Paradise\nhttps://stream.radioparadise.com/aac-320\nhttps://stream-uk1.radioparadise.com/aac-320\n";
        let kind_m3u = detect_playlist_kind("https://example.com/rp.m3u8", None).unwrap();
        let urls_m3u = parse_playlist_urls(kind_m3u, m3u);
        assert_eq!(urls_m3u.len(), 2);
        assert_eq!(urls_m3u[0], "https://stream.radioparadise.com/aac-320");

        let xspf = r#"<?xml version="1.0" encoding="UTF-8"?>
<playlist version="1" xmlns="http://xspf.org/ns/0/">
  <trackList>
    <track><location>https://ice1.somafm.com/dronezone-128-mp3?foo=1&amp;bar=2</location></track>
    <track><location>https://ice2.somafm.com/dronezone-128-mp3</location></track>
  </trackList>
</playlist>"#;
        let kind_xspf =
            detect_playlist_kind("https://somafm.com/stream", Some("application/xspf+xml"))
                .unwrap();
        let urls_xspf = parse_playlist_urls(kind_xspf, xspf);
        assert_eq!(
            urls_xspf,
            vec![
                "https://ice1.somafm.com/dronezone-128-mp3?foo=1&bar=2".to_string(),
                "https://ice2.somafm.com/dronezone-128-mp3".to_string(),
            ]
        );
    }
}
