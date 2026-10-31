//! Format probing: refuse formats the registry cannot play, build Symphonia hints, and open
//! the first decodable audio track of a source.

#![forbid(unsafe_code)]

use amp_ports::DecodeError;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;

/// A probed audio stream with its decoder.
pub(crate) struct OpenedAudio {
    pub reader: Box<dyn FormatReader>,
    pub decoder: Box<dyn AudioDecoder>,
    pub track_id: u32,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_frames: Option<u64>,
}

/// Refuses formats `amp_domain::formats` does not mark playable (e.g. Opus until TASK-229,
/// see ADR-004), with a message naming the format instead of an opaque probe error.
pub(crate) fn reject_unplayable(extension: Option<&str>, mime_type: Option<&str>) -> Result<(), DecodeError> {
    use amp_domain::formats;
    let format = extension
        .and_then(formats::by_extension)
        .or_else(|| mime_type.and_then(formats::by_mime));
    match format.and_then(formats::unplayable_reason) {
        Some(reason) => Err(DecodeError::UnsupportedFormat(reason)),
        None => Ok(()),
    }
}

/// Builds a Symphonia probe [`Hint`] from an optional file extension and/or MIME type (`TASK-139`).
pub fn build_probe_hint(extension: Option<&str>, mime_type: Option<&str>) -> Hint {
    let mut hint = Hint::new();
    if let Some(ext) = extension {
        let trimmed = ext.trim().trim_start_matches('.');
        if !trimmed.is_empty() {
            hint.with_extension(trimmed);
        }
    }
    if let Some(mime) = mime_type {
        let clean = mime.split(';').next().unwrap_or(mime).trim();
        if !clean.is_empty() {
            hint.mime_type(clean);
        }
    }
    hint
}

/// Probes `source`, picks its first decodable audio track and makes a (gapless) decoder.
pub(crate) fn open_audio<S>(source: S) -> Result<OpenedAudio, String>
where
    S: MediaSource + 'static,
{
    open_audio_with_hint(source, &Hint::new())
}

/// Probes `source` with an explicit [`Hint`], picks its first decodable audio track and makes a decoder.
pub(crate) fn open_audio_with_hint<S>(source: S, hint: &Hint) -> Result<OpenedAudio, String>
where
    S: MediaSource + 'static,
{
    let mss = MediaSourceStream::new(Box::new(source), Default::default());
    let reader = symphonia::default::get_probe()
        .probe(
            hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| format!("Unrecognized audio format: {e}"))?;
    let track = reader
        .first_track_known_codec(TrackType::Audio)
        .ok_or_else(|| "No playable audio tracks found".to_string())?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or_else(|| "No playable audio tracks found".to_string())?
        .clone();
    let (track_id, duration_frames) = (track.id, track.num_frames);
    let decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(|e| format!("Unsupported codec: {e}"))?;
    Ok(OpenedAudio {
        reader,
        decoder,
        track_id,
        sample_rate: params.sample_rate.unwrap_or(44_100),
        channels: params.channels.as_ref().map_or(2, |c| c.count() as u16),
        duration_frames,
    })
}
