//! Track references a phone may send with a request (TASK-215).
//!
//! A reference names *what* to play, never *how* to open it: a track the host already has, a
//! link to an allow-listed provider the host can resolve itself, or a search query. File paths,
//! arbitrary URLs, IP addresses and unknown hosts are refused here, before the host sees them.

#![forbid(unsafe_code)]

use amp_domain::TrackId;
use serde::{Deserialize, Serialize};
use url::{Host, Url};

/// Longest link a phone may send, in bytes.
pub const MAX_LINK_LEN: usize = 512;
/// Longest search query a phone may send, in characters.
pub const MAX_QUERY_LEN: usize = 200;

/// Hosts whose links the host's own resolvers can play. Only YouTube has a resolver today;
/// Bandcamp and SoundCloud join the list when theirs exist.
const YOUTUBE_HOSTS: [&str; 4] = [
    "youtube.com",
    "www.youtube.com",
    "m.youtube.com",
    "music.youtube.com",
];
const YOUTUBE_SHORT_HOST: &str = "youtu.be";

/// What a phone asks the host to play, as sent on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum TrackReference {
    /// A track the host already holds (from search results or the status broadcast).
    Library(TrackId),
    /// A provider link, for example a YouTube watch URL.
    Link(String),
    /// Free text the host searches for.
    Search(String),
}

/// A reference that passed validation, in the normalized form the host acts on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidReference {
    /// A track the host already holds.
    Library(TrackId),
    /// A YouTube video, by its 11-character id.
    YouTube { video_id: String },
    /// A search query with surrounding and repeated whitespace collapsed.
    Search(String),
    /// An audio file a phone uploaded (TASK-218), stored by the daemon under a generated name.
    /// Never produced from a phone's message, only by the upload endpoint.
    Upload {
        path: std::path::PathBuf,
        /// The phone's file name, cleaned for display only.
        name: String,
    },
}

/// Why a reference was refused. The text is shown on the phone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusedReference(pub &'static str);

impl std::fmt::Display for RefusedReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl TrackReference {
    /// Checks the reference and returns the form the host may act on. Never panics.
    pub fn validate(&self) -> Result<ValidReference, RefusedReference> {
        match self {
            Self::Library(id) => Ok(ValidReference::Library(*id)),
            Self::Link(link) => validate_link(link),
            Self::Search(query) => validate_query(query).map(ValidReference::Search),
        }
    }
}

/// Normalizes a search query: trimmed, whitespace collapsed, printable, 1..=200 characters.
pub fn validate_query(query: &str) -> Result<String, RefusedReference> {
    if query.chars().any(|c| c.is_control() && !c.is_whitespace()) {
        return Err(RefusedReference("The search contains control characters."));
    }
    let normalized = query.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Err(RefusedReference("The search is empty."));
    }
    if normalized.chars().count() > MAX_QUERY_LEN {
        return Err(RefusedReference("The search is too long."));
    }
    Ok(normalized)
}

fn validate_link(link: &str) -> Result<ValidReference, RefusedReference> {
    let link = link.trim();
    if link.len() > MAX_LINK_LEN {
        return Err(RefusedReference("The link is too long."));
    }
    let url = Url::parse(link).map_err(|_| RefusedReference("That is not a web link."))?;
    if !matches!(url.scheme(), "https" | "http") {
        return Err(RefusedReference("Only web links can be requested."));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(RefusedReference("Links with a login are not accepted."));
    }
    if url.port().is_some() {
        return Err(RefusedReference(
            "Links with a port number are not accepted.",
        ));
    }
    let host = match url.host() {
        Some(Host::Domain(d)) => d.to_ascii_lowercase(),
        _ => return Err(RefusedReference("Links to addresses are not accepted.")),
    };
    let video_id = if host == YOUTUBE_SHORT_HOST {
        url.path_segments()
            .and_then(|mut s| s.next())
            .map(str::to_owned)
    } else if YOUTUBE_HOSTS.contains(&host.as_str()) {
        youtube_id_from_long_url(&url)
    } else {
        return Err(RefusedReference(
            "Only YouTube links can be requested right now.",
        ));
    };
    match video_id {
        Some(id) if is_video_id(&id) => Ok(ValidReference::YouTube { video_id: id }),
        _ => Err(RefusedReference("That YouTube link has no video in it.")),
    }
}

fn youtube_id_from_long_url(url: &Url) -> Option<String> {
    let mut segments = url.path_segments()?;
    match segments.next()? {
        "watch" => url
            .query_pairs()
            .find(|(k, _)| k == "v")
            .map(|(_, v)| v.into_owned()),
        "shorts" | "embed" | "live" => segments.next().map(str::to_owned),
        _ => None,
    }
}

/// YouTube video ids are exactly 11 characters from `[A-Za-z0-9_-]`.
fn is_video_id(id: &str) -> bool {
    id.len() == 11
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// One search result sent to the phone. It carries a reference the phone can request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchItem {
    pub title: String,
    pub artist: String,
    pub duration_secs: Option<u64>,
    /// Where the result comes from: `library` or a provider id such as `youtube`.
    pub source: String,
    pub reference: TrackReference,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_is_normalized() {
        assert_eq!(validate_query("  a \t b  "), Ok("a b".into()));
        assert!(validate_query("").is_err());
        assert_eq!(validate_query("a\nb"), Ok("a b".into()));
        assert!(validate_query("a\u{7}b").is_err());
        assert!(validate_query(&"é".repeat(200)).is_ok());
        assert!(validate_query(&"é".repeat(201)).is_err());
    }

    #[test]
    fn video_ids_are_strict() {
        assert!(is_video_id("dQw4w9WgXcQ"));
        assert!(!is_video_id("dQw4w9WgXc"));
        assert!(!is_video_id("../../etc/x"));
    }

    #[test]
    fn library_ids_pass_through() {
        let id = TrackId::new();
        assert_eq!(
            TrackReference::Library(id).validate(),
            Ok(ValidReference::Library(id))
        );
    }
}
