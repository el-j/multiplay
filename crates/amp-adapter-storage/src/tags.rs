//! Track metadata from audio files (TASK-230): tags, stream properties, loudness tags and
//! embedded cover art for every playable format.
//!
//! `lofty` reads most formats. Containers it does not know (CAF, Matroska/WebM, ADPCM WAV) fall
//! back to Symphonia's format readers, so every file the player can decode also gets a library
//! entry. Files without tags take "Artist - Title" from their name.

#![forbid(unsafe_code)]

use amp_domain::{AudioSource, Track, TrackId, TrackMetadata};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::PictureType;
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, Tag};
use std::path::Path;
use std::time::Duration;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::TrackType;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::StandardTag;

/// Prefix of `cover_art_uri` for art embedded in the file itself; the rest is the file path.
pub const EMBEDDED_COVER_PREFIX: &str = "embedded:";

/// An image stored inside an audio file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverArt {
    pub mime_type: String,
    pub data: Vec<u8>,
}

/// Reads a library track from `path`, or `None` when the file is not readable audio.
pub fn extract_track_from_file(path: &Path) -> Option<Track> {
    let metadata = read_with_lofty(path).or_else(|| {
        // Only fall back for formats the player can actually decode.
        amp_domain::formats::is_playable_path(path)
            .then(|| read_with_symphonia(path))
            .flatten()
    })?;
    let source = AudioSource::LocalFile {
        path: path.to_string_lossy().to_string(),
    };
    Some(Track {
        id: TrackId::from_source(&source),
        source,
        metadata,
        date_added_epoch: 0,
    })
}

/// The front cover (or first picture) embedded in `path`, if any.
pub fn embedded_cover_art(path: &Path) -> Option<CoverArt> {
    let file = Probe::open(path).ok()?.read().ok()?;
    let picture = file.tags().iter().find_map(|tag| {
        tag.get_picture_type(PictureType::CoverFront)
            .or_else(|| tag.pictures().first())
    })?;
    Some(CoverArt {
        mime_type: picture
            .mime_type()
            .map_or("application/octet-stream", |m| m.as_str())
            .to_string(),
        data: picture.data().to_vec(),
    })
}

fn read_with_lofty(path: &Path) -> Option<TrackMetadata> {
    let file = Probe::open(path).ok()?.read().ok()?;
    let props = file.properties();
    let tag = file.primary_tag().or_else(|| file.first_tag());
    let mut m = TrackMetadata::new("", "");
    m.duration = Some(props.duration());
    m.sample_rate = props.sample_rate();
    m.channels = props.channels().map(u16::from);
    m.bit_depth = props.bit_depth().map(u16::from);
    m.bitrate_kbps = props.audio_bitrate();
    if let Some(t) = tag {
        apply_lofty_tag(&mut m, t);
    }
    if file.tags().iter().any(|t| !t.pictures().is_empty()) {
        m.cover_art_uri = Some(format!("{EMBEDDED_COVER_PREFIX}{}", path.display()));
    }
    name_fallback(&mut m, path);
    Some(m)
}

fn apply_lofty_tag(m: &mut TrackMetadata, t: &Tag) {
    let text = |v: Option<std::borrow::Cow<'_, str>>| v.map(|s| s.trim().to_string());
    m.title = text(t.title()).unwrap_or_default();
    m.artist = text(t.artist()).unwrap_or_default();
    m.album = text(t.album()).filter(|s| !s.is_empty());
    m.genre = text(t.genre()).filter(|s| !s.is_empty());
    m.album_artist = t
        .get_string(ItemKey::AlbumArtist)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    m.track_number = t.track();
    m.disc_number = t.disk();
    m.year = t.date().map(|d| d.year);
    m.replay_gain_mb = t
        .get_string(ItemKey::ReplayGainTrackGain)
        .and_then(parse_gain_db)
        .map(|db| (db * 100.0).round() as i32);
    m.integrated_lufs_mb = t
        .get_string(ItemKey::R128TrackGain)
        .and_then(r128_gain_to_lufs)
        .map(|v| (v * 100.0).round() as i32);
}

/// Stream properties and standard tags through Symphonia, for containers lofty cannot read.
fn read_with_symphonia(path: &Path) -> Option<TrackMetadata> {
    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut reader = symphonia::default::get_probe()
        .probe(&hint, mss, Default::default(), Default::default())
        .ok()?;
    let mut m = TrackMetadata::new("", "");
    {
        let track = reader.first_track(TrackType::Audio)?;
        let audio = track.codec_params.as_ref().and_then(|p| p.audio());
        let rate = audio.and_then(|a| a.sample_rate);
        m.sample_rate = rate;
        m.channels = audio
            .and_then(|a| a.channels.as_ref())
            .map(|c| c.count() as u16);
        m.bit_depth = audio.and_then(|a| a.bits_per_sample).map(|b| b as u16);
        let media = reader.media_info();
        m.duration = track
            .num_frames
            .zip(rate)
            .map(|(n, r)| Duration::from_secs_f64(n as f64 / r as f64))
            .or_else(|| {
                let t = track.time_base?.calc_duration(track.duration?)?;
                Some(Duration::from_secs_f64(t.as_secs_f64()))
            })
            .or_else(|| {
                let t = media.time_base?.calc_duration(media.duration?)?;
                Some(Duration::from_secs_f64(t.as_secs_f64()))
            });
    }
    if let Some(rev) = reader.metadata().skip_to_latest() {
        let tags = rev
            .media
            .tags
            .iter()
            .chain(rev.per_track.iter().flat_map(|t| t.metadata.tags.iter()));
        for tag in tags {
            apply_standard_tag(&mut m, tag.std.as_ref());
            apply_raw_tag(&mut m, &tag.raw.key, &tag.raw.value.to_string());
        }
    }
    // Matroska writers often tag only the album level; its artist is the track's artist then.
    if m.artist.is_empty() {
        if let Some(a) = &m.album_artist {
            m.artist = a.clone();
        }
    }
    name_fallback(&mut m, path);
    Some(m)
}

fn apply_standard_tag(m: &mut TrackMetadata, tag: Option<&StandardTag>) {
    let set = |field: &mut String, v: &str| {
        if field.is_empty() {
            *field = v.trim().to_string();
        }
    };
    match tag {
        Some(StandardTag::TrackTitle(v)) => set(&mut m.title, v),
        Some(StandardTag::Artist(v)) => set(&mut m.artist, v),
        Some(StandardTag::Album(v)) => m.album = m.album.take().or(Some(v.to_string())),
        Some(StandardTag::AlbumArtist(v)) => {
            m.album_artist = m.album_artist.take().or(Some(v.to_string()));
        }
        Some(StandardTag::Genre(v)) => m.genre = m.genre.take().or(Some(v.to_string())),
        Some(StandardTag::TrackNumber(n)) => m.track_number = m.track_number.or(Some(*n as u32)),
        Some(StandardTag::DiscNumber(n)) => m.disc_number = m.disc_number.or(Some(*n as u32)),
        Some(StandardTag::ReleaseYear(y)) => m.year = m.year.or(Some(*y)),
        Some(StandardTag::ReplayGainTrackGain(v)) => {
            m.replay_gain_mb = m
                .replay_gain_mb
                .or_else(|| parse_gain_db(v).map(|db| (db * 100.0).round() as i32));
        }
        _ => {}
    }
}

/// Matroska tags Symphonia does not map, keyed `TARGET@NAME` (e.g. `ALBUM@ALBUM`).
fn apply_raw_tag(m: &mut TrackMetadata, key: &str, value: &str) {
    let name = key.rsplit('@').next().unwrap_or(key);
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    match name.to_ascii_uppercase().as_str() {
        "TITLE" if m.title.is_empty() => m.title = value.to_string(),
        "ALBUM" if m.album.is_none() => m.album = Some(value.to_string()),
        _ => {}
    }
}

/// Fills a missing title/artist from the file name: "Artist - Title.ext", else the bare name.
fn name_fallback(m: &mut TrackMetadata, path: &Path) {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown Title")
        .trim();
    let (artist, title) = match stem.split_once(" - ") {
        Some((a, t)) if !a.trim().is_empty() && !t.trim().is_empty() => (Some(a.trim()), t.trim()),
        _ => (None, stem),
    };
    if m.title.is_empty() {
        m.title = title.to_string();
        if m.artist.is_empty() {
            if let Some(a) = artist {
                m.artist = a.to_string();
            }
        }
    }
    if m.artist.is_empty() {
        m.artist = "Unknown Artist".to_string();
    }
}

fn parse_gain_db(s: &str) -> Option<f32> {
    s.replace("dB", "").trim().parse::<f32>().ok()
}

/// Integrated loudness from a Vorbis `R128_TRACK_GAIN` tag. The tag holds the gain (in
/// Q7.8 fixed point, i.e. 1/256 dB) that brings the track to -23 LUFS, so the track's own
/// loudness is -23 LUFS minus that gain.
fn r128_gain_to_lufs(s: &str) -> Option<f32> {
    let q78 = s.trim().parse::<i32>().ok()?;
    Some(-23.0 - q78 as f32 / 256.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaygain_and_r128_tags_parse() {
        assert_eq!(parse_gain_db("-6.48 dB"), Some(-6.48));
        assert_eq!(parse_gain_db("+2 dB"), Some(2.0));
        assert_eq!(r128_gain_to_lufs("-1280"), Some(-18.0));
        assert_eq!(r128_gain_to_lufs("512"), Some(-25.0));
        assert_eq!(r128_gain_to_lufs("loud"), None);
    }
}
