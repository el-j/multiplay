//! CUE sheets (TASK-232): turn a single-file album (`album.flac` + `album.cue`) into one track
//! per song. Each track plays its range of the file through `TrackMetadata::segment`.
//!
//! Real-world sheets are often Latin-1 with Windows line endings, and often name `album.wav`
//! for a file that was later converted to FLAC; both are handled.

#![forbid(unsafe_code)]

use crate::tags::extract_track_from_file;
use amp_domain::{AudioSource, Track, TrackId, TrackSegment};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// CD frames per second in `INDEX mm:ss:ff`.
const FRAMES_PER_SECOND: f64 = 75.0;

/// A parsed CUE sheet.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CueSheet {
    pub title: Option<String>,
    pub performer: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u16>,
    pub files: Vec<CueFile>,
}

/// One `FILE` entry and the tracks inside it.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CueFile {
    pub name: String,
    pub tracks: Vec<CueTrack>,
}

/// One `TRACK`: where it starts (`INDEX 01`) and its own tags.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CueTrack {
    pub number: u32,
    pub title: Option<String>,
    pub performer: Option<String>,
    pub start: Option<Duration>,
}

/// Text of a sheet: UTF-8 (with or without BOM), otherwise Latin-1.
pub fn decode_cue_text(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    }
}

/// Parses the commands multiplay uses; unknown commands are ignored.
pub fn parse_cue(text: &str) -> CueSheet {
    let mut sheet = CueSheet::default();
    for line in text.lines() {
        let line = line.trim();
        let (cmd, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim();
        let track = sheet.files.last_mut().and_then(|f| f.tracks.last_mut());
        match cmd.to_ascii_uppercase().as_str() {
            "FILE" => sheet.files.push(CueFile {
                name: first_value(rest),
                tracks: Vec::new(),
            }),
            "TRACK" => {
                if let Some(file) = sheet.files.last_mut() {
                    let number = rest.split_whitespace().next().and_then(|n| n.parse().ok());
                    file.tracks.push(CueTrack {
                        number: number.unwrap_or(file.tracks.len() as u32 + 1),
                        ..CueTrack::default()
                    });
                }
            }
            "TITLE" => match track {
                Some(t) => t.title = Some(first_value(rest)),
                None => sheet.title = Some(first_value(rest)),
            },
            "PERFORMER" => match track {
                Some(t) => t.performer = Some(first_value(rest)),
                None => sheet.performer = Some(first_value(rest)),
            },
            "INDEX" => {
                let mut parts = rest.split_whitespace();
                if let (Some("01"), Some(time), Some(t)) = (parts.next(), parts.next(), track) {
                    t.start = parse_msf(time);
                }
            }
            "REM" => {
                let (key, value) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
                match key.to_ascii_uppercase().as_str() {
                    "GENRE" => sheet.genre = Some(first_value(value.trim())),
                    "DATE" => {
                        sheet.year = first_value(value.trim())
                            .get(..4)
                            .and_then(|y| y.parse().ok())
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    sheet
}

/// A quoted string, or the first word.
fn first_value(s: &str) -> String {
    match s.strip_prefix('"') {
        Some(inner) => inner.split('"').next().unwrap_or("").to_string(),
        None => s.split_whitespace().next().unwrap_or("").to_string(),
    }
}

/// `mm:ss:ff` (75 frames per second).
fn parse_msf(s: &str) -> Option<Duration> {
    let mut it = s.split(':').map(|p| p.parse::<u64>().ok());
    let (m, sec, f) = (it.next()??, it.next()??, it.next()??);
    Some(Duration::from_secs_f64(
        (m * 60 + sec) as f64 + f as f64 / FRAMES_PER_SECOND,
    ))
}

/// The audio file a `FILE` entry names, or a file with the same name and another playable
/// extension (sheets often still say `.wav` after a conversion to FLAC).
pub fn resolve_cue_file(cue_dir: &Path, name: &str) -> Option<PathBuf> {
    let named = cue_dir.join(name.replace('\\', "/"));
    if named.is_file() {
        return Some(named);
    }
    let stem = named.file_stem()?.to_owned();
    amp_domain::formats::playable_extensions()
        .map(|ext| cue_dir.join(&stem).with_extension(ext))
        .find(|p| p.is_file())
}

/// Audio files a sheet covers, so a folder scan can play them through the sheet instead.
pub fn cue_audio_files(cue_path: &Path) -> Vec<PathBuf> {
    let Ok(bytes) = std::fs::read(cue_path) else {
        return Vec::new();
    };
    let dir = cue_path.parent().unwrap_or(Path::new("."));
    parse_cue(&decode_cue_text(&bytes))
        .files
        .iter()
        .filter_map(|f| resolve_cue_file(dir, &f.name))
        .collect()
}

/// One library track per song of the sheet at `cue_path`.
pub fn import_cue(cue_path: &Path) -> Result<Vec<Track>, String> {
    let bytes = std::fs::read(cue_path).map_err(|e| format!("{}: {e}", cue_path.display()))?;
    let sheet = parse_cue(&decode_cue_text(&bytes));
    let dir = cue_path.parent().unwrap_or(Path::new("."));
    let mut tracks = Vec::new();
    for file in &sheet.files {
        let audio = resolve_cue_file(dir, &file.name).ok_or_else(|| {
            format!(
                "{}: audio file {:?} not found",
                cue_path.display(),
                file.name
            )
        })?;
        let base = extract_track_from_file(&audio)
            .ok_or_else(|| format!("{}: cannot read {}", cue_path.display(), audio.display()))?;
        let file_len = base.metadata.duration;
        for (i, song) in file.tracks.iter().enumerate() {
            let start = song.start.unwrap_or_default();
            let end = file.tracks.get(i + 1).and_then(|next| next.start);
            let mut m = base.metadata.clone();
            m.title = song
                .title
                .clone()
                .unwrap_or_else(|| format!("Track {:02}", song.number));
            m.artist = song
                .performer
                .clone()
                .or_else(|| sheet.performer.clone())
                .unwrap_or(m.artist);
            m.album = sheet.title.clone().or(m.album);
            m.album_artist = sheet.performer.clone().or(m.album_artist);
            m.genre = sheet.genre.clone().or(m.genre);
            m.year = sheet.year.or(m.year);
            m.track_number = Some(song.number);
            m.segment = Some(TrackSegment { start, end });
            m.duration = end.or(file_len).map(|e| e.saturating_sub(start));
            let source = AudioSource::LocalFile {
                path: audio.to_string_lossy().into_owned(),
            };
            // Songs share the file, so the id also names the song.
            let id = TrackId::from_source(&AudioSource::LocalFile {
                path: format!("{}#{}", audio.to_string_lossy(), song.number),
            });
            tracks.push(Track {
                id,
                source,
                metadata: m,
                date_added_epoch: 0,
            });
        }
    }
    Ok(tracks)
}
