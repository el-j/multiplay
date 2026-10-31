//! Omnibox and link actions of the search window (`TASK-153`): searching, adding links, and
//! playing or queueing results.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use amp_domain::{AudioSource, Track};

impl MultiplayGuiApp {
    /// Ingests a stream URL (e.g. YouTube, Spotify, HTTP stream).
    pub fn ingest_url(&mut self, url: &str) {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return;
        }

        let (source, title) = match amp_domain::parse_source_url(trimmed) {
            amp_domain::ParsedSourceUrl::YouTube(yt) => (
                AudioSource::YouTube {
                    video_id: yt.video_id,
                },
                "YouTube Stream",
            ),
            amp_domain::ParsedSourceUrl::BandcampTrack { url }
            | amp_domain::ParsedSourceUrl::BandcampAlbum { url } => {
                (AudioSource::Bandcamp { stream_url: url }, "Bandcamp Stream")
            }
            amp_domain::ParsedSourceUrl::SpotifyTrack { uri } => {
                (AudioSource::Spotify { track_uri: uri }, "Spotify Track")
            }
            amp_domain::ParsedSourceUrl::DirectHttp { url } => {
                (AudioSource::DirectHttp { url }, "Direct HTTP Stream")
            }
            amp_domain::ParsedSourceUrl::YouTubePlaylist { playlist_id } => (
                AudioSource::DirectHttp {
                    url: format!("https://www.youtube.com/playlist?list={playlist_id}"),
                },
                "YouTube Playlist",
            ),
        };

        let meta = amp_domain::TrackMetadata::new(title, "Network Source");
        self.playlist.push(Track::new(source, meta));
        self.save_session();
    }

    /// Triggers background omnibox search across YouTube and Bandcamp (`TASK-153`).
    pub fn trigger_gui_search(&mut self) {
        let q = self.search_query_text.trim().to_string();
        if q.is_empty() {
            self.search_results.clear();
            self.is_searching = false;
            return;
        }
        // A pasted link is added directly; searching for it would find nothing useful.
        if looks_like_link(&q) {
            self.ingest_url(&q);
            self.search_query_text.clear();
            self.search_results.clear();
            self.search_status = None;
            self.is_searching = false;
            return;
        }
        self.is_searching = true;
        self.search_status = None;
        let job = crate::remote_lookup::LookupJob::OmniSearch { query: q };
        self.lookup().submit(job);
    }

    /// Adds a searched track to playlist and immediately begins playback.
    pub fn play_search_result(&mut self, track: Track) {
        self.playlist.push(track.clone());
        self.selected_track_idx = self.playlist.len() - 1;
        self.audio_driver.play(track);
        let upcoming = self.playlist.get(self.selected_track_idx + 1).cloned();
        self.audio_driver.set_next_track(upcoming);
        self.save_session();
    }

    /// Appends a searched track to the end of the playlist.
    pub fn add_search_result(&mut self, track: Track) {
        self.playlist.push(track);
        self.save_session();
    }
}
/// Whether omnibox text is a link to add rather than words to search for.
fn looks_like_link(text: &str) -> bool {
    let t = text.trim();
    if t.contains(char::is_whitespace) {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || [
            "youtube.com/",
            "youtu.be/",
            ".bandcamp.com/",
            "open.spotify.com/",
        ]
        .iter()
        .any(|host| lower.contains(host))
}
