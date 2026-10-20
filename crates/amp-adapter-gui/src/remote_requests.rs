//! Requests from paired phones waiting for the host (TASK-216): what was asked, by whom, and
//! the host's own preview of what it would play.

#![forbid(unsafe_code)]

use amp_adapter_remote::{DeviceId, RequestId, ValidReference};
use amp_domain::{AudioSource, Track};

/// The host's view of what a request would add.
#[derive(Debug, Clone, PartialEq)]
pub enum Preview {
    /// The host's resolver is looking it up.
    Resolving,
    /// What accepting adds to the playlist.
    Ready(Box<Track>),
    /// The host could not find it; the request can only be rejected.
    Failed(String),
}

/// One request row in the Remote window.
#[derive(Debug, Clone, PartialEq)]
pub struct RequestRow {
    pub id: RequestId,
    pub device: DeviceId,
    pub device_name: String,
    pub reference: ValidReference,
    pub preview: Preview,
    /// The device is trusted: accept as soon as the preview is ready.
    pub auto_accept: bool,
    /// Daemon run the request came from; ids restart when the daemon does.
    pub generation: u64,
}

impl RequestRow {
    /// What the phone asked for, before the host knows the title.
    #[must_use]
    pub fn asked_for(&self) -> String {
        match &self.reference {
            ValidReference::Library(_) => "a track from your library".to_string(),
            ValidReference::YouTube { video_id } => format!("youtu.be/{video_id}"),
            ValidReference::Search(q) => format!("“{q}”"),
            ValidReference::Upload { name, .. } => format!("uploaded file {name}"),
        }
    }
}

/// What the host does with a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moderation {
    /// Add to the end of the playlist.
    AddToEnd,
    /// Insert right after the current track.
    PlayNext,
    /// Add to the end and assign to deck 0 (A) or 1 (B).
    Deck(usize),
    /// Turn it down; the playlist is not touched.
    Reject,
}

/// Requests waiting for an answer, oldest first.
#[derive(Debug, Default)]
pub struct RequestQueue {
    pub rows: Vec<RequestRow>,
}

impl RequestQueue {
    pub fn get_mut(&mut self, id: RequestId) -> Option<&mut RequestRow> {
        self.rows.iter_mut().find(|r| r.id == id)
    }

    /// Removes and returns request `id`.
    pub fn take(&mut self, id: RequestId) -> Option<RequestRow> {
        let idx = self.rows.iter().position(|r| r.id == id)?;
        Some(self.rows.remove(idx))
    }
}

/// Playlist tracks whose "artist title" contains every word of `query` (case-insensitive).
#[must_use]
pub fn library_matches<'a>(playlist: &'a [Track], query: &str, limit: usize) -> Vec<&'a Track> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return Vec::new();
    }
    playlist
        .iter()
        .filter(|t| {
            let hay = format!("{} {}", t.metadata.artist, t.metadata.title).to_lowercase();
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .take(limit)
        .collect()
}

/// A fresh playlist entry for `track` (its own id, so deck and requester data stay separate).
#[must_use]
pub fn requested_copy(track: &Track) -> Track {
    Track::new(track.source.clone(), track.metadata.clone())
}

/// The allow-listed link a phone can request `source` with, if there is one.
#[must_use]
pub fn shareable_link(source: &AudioSource) -> Option<String> {
    match source {
        AudioSource::YouTube { video_id } if video_id.len() == 11 => {
            Some(format!("https://www.youtube.com/watch?v={video_id}"))
        }
        _ => None,
    }
}

/// "title - artist", as shown on the phone.
#[must_use]
pub fn display_title(track: &Track) -> String {
    format!("{} - {}", track.metadata.title, track.metadata.artist)
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::TrackMetadata;

    fn track(title: &str, artist: &str) -> Track {
        Track::new(
            AudioSource::LocalFile {
                path: format!("/m/{title}.flac"),
            },
            TrackMetadata::new(title, artist),
        )
    }

    #[test]
    fn library_search_needs_every_word() {
        let list = vec![
            track("One More Time", "Daft Punk"),
            track("Around", "Daft Punk"),
        ];
        assert_eq!(library_matches(&list, "daft time", 10).len(), 1);
        assert_eq!(library_matches(&list, "DAFT", 10).len(), 2);
        assert!(library_matches(&list, "  ", 10).is_empty());
        assert_eq!(library_matches(&list, "punk", 1).len(), 1);
    }

    #[test]
    fn copies_get_their_own_id_and_links_are_canonical() {
        let t = track("A", "B");
        assert_ne!(requested_copy(&t).id, t.id);
        let yt = AudioSource::YouTube {
            video_id: "dQw4w9WgXcQ".into(),
        };
        assert_eq!(
            shareable_link(&yt).as_deref(),
            Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        );
        assert_eq!(shareable_link(&t.source), None);
    }
}
