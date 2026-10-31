//! The one list of audio file formats (TASK-225, PLAN-058). The scanner, file dialogs, drag and
//! drop, phone uploads and decoder hints all ask this registry, so a format becomes playable in
//! one place and an unplayable file can say why it was skipped.

#![forbid(unsafe_code)]

use std::path::Path;

/// Whether multiplay can play a format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatStatus {
    /// Decodes with the pure-Rust decoder set today.
    Playable,
    /// Known and scheduled; `task` adds it.
    Planned { task: &'static str },
    /// No pure-Rust decoder is available; `reason` is shown to the user.
    Unsupported { reason: &'static str },
}

/// One audio file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    /// Name shown to people, e.g. "FLAC".
    pub name: &'static str,
    /// Lower-case file extensions without the dot; the first one is the usual one.
    pub extensions: &'static [&'static str],
    /// MIME types, for streams and uploads.
    pub mime_types: &'static [&'static str],
    pub status: FormatStatus,
}

const fn playable(
    name: &'static str,
    extensions: &'static [&'static str],
    mime_types: &'static [&'static str],
) -> AudioFormat {
    AudioFormat {
        name,
        extensions,
        mime_types,
        status: FormatStatus::Playable,
    }
}

const fn unsupported(
    name: &'static str,
    extensions: &'static [&'static str],
    reason: &'static str,
) -> AudioFormat {
    AudioFormat {
        name,
        extensions,
        mime_types: &[],
        status: FormatStatus::Unsupported { reason },
    }
}

/// Every audio format multiplay knows about.
pub const AUDIO_FORMATS: &[AudioFormat] = &[
    playable("MP3", &["mp3"], &["audio/mpeg", "audio/mp3"]),
    playable("FLAC", &["flac"], &["audio/flac", "audio/x-flac"]),
    playable(
        "WAV",
        &["wav", "wave"],
        &["audio/wav", "audio/x-wav", "audio/wave"],
    ),
    playable(
        "Ogg Vorbis",
        &["ogg", "oga"],
        &["audio/ogg", "audio/vorbis"],
    ),
    playable(
        "AAC / ALAC (M4A)",
        &["m4a", "m4b", "aac"],
        &["audio/mp4", "audio/aac", "audio/x-m4a"],
    ),
    playable(
        "AIFF",
        &["aif", "aiff", "aifc"],
        &["audio/aiff", "audio/x-aiff"],
    ),
    playable("Core Audio (CAF)", &["caf"], &["audio/x-caf"]),
    playable(
        "Matroska / WebM audio",
        &["mka", "webm"],
        &["audio/webm", "audio/x-matroska"],
    ),
    playable("MPEG Layer I/II", &["mp1", "mp2"], &["audio/mpeg"]),
    playable("Opus", &["opus"], &["audio/opus"]),
    playable("DSD", &["dsf", "dff"], &["audio/dsd"]),
    playable("WavPack", &["wv"], &["audio/wavpack", "audio/x-wavpack"]),
    unsupported(
        "Monkey's Audio",
        &["ape"],
        "a pure-Rust decoder is being evaluated (ADR-006)",
    ),
    unsupported(
        "Musepack",
        &["mpc", "mp+"],
        "no mature pure-Rust decoder yet (ADR-006)",
    ),
    unsupported("TTA", &["tta"], "no mature pure-Rust decoder yet (ADR-006)"),
    unsupported(
        "Windows Media Audio",
        &["wma"],
        "no mature pure-Rust decoder yet (ADR-006)",
    ),
];

fn normalize(ext: &str) -> String {
    ext.trim().trim_start_matches('.').to_ascii_lowercase()
}

/// The format that uses `ext` (case and a leading dot are ignored).
pub fn by_extension(ext: &str) -> Option<&'static AudioFormat> {
    let ext = normalize(ext);
    AUDIO_FORMATS
        .iter()
        .find(|f| f.extensions.contains(&ext.as_str()))
}

/// The format with MIME type `mime` (parameters such as `; codecs=...` and case are ignored).
pub fn by_mime(mime: &str) -> Option<&'static AudioFormat> {
    let mime = mime
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    AUDIO_FORMATS
        .iter()
        .find(|f| f.mime_types.contains(&mime.as_str()))
}

/// Why `format` cannot be played, or `None` when it can.
pub fn unplayable_reason(format: &AudioFormat) -> Option<String> {
    match format.status {
        FormatStatus::Playable => None,
        FormatStatus::Planned { .. } => Some(format!("{} is not supported yet", format.name)),
        FormatStatus::Unsupported { reason } => Some(format!("{}: {reason}", format.name)),
    }
}

/// The format of `path`, judged by its extension.
pub fn by_path(path: &Path) -> Option<&'static AudioFormat> {
    path.extension()
        .and_then(|e| e.to_str())
        .and_then(by_extension)
}

/// Whether files with extension `ext` can be played.
pub fn is_playable_extension(ext: &str) -> bool {
    by_extension(ext).is_some_and(|f| f.status == FormatStatus::Playable)
}

/// Whether `path` looks like a playable audio file.
pub fn is_playable_path(path: &Path) -> bool {
    by_path(path).is_some_and(|f| f.status == FormatStatus::Playable)
}

/// All playable extensions, in registry order.
pub fn playable_extensions() -> impl Iterator<Item = &'static str> {
    AUDIO_FORMATS
        .iter()
        .filter(|f| f.status == FormatStatus::Playable)
        .flat_map(|f| f.extensions.iter().copied())
}

/// Names of the playable formats, for hints like "MP3 · FLAC · WAV".
pub fn playable_names() -> impl Iterator<Item = &'static str> {
    AUDIO_FORMATS
        .iter()
        .filter(|f| f.status == FormatStatus::Playable)
        .map(|f| f.name)
}

/// Why an audio file at `path` cannot be played, or `None` when it can (or is not audio).
pub fn skip_reason(path: &Path) -> Option<String> {
    unplayable_reason(by_path(path)?)
}
