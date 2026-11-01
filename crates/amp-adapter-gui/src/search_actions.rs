//! Omnibox and link actions of the search window (`TASK-153`): searching, adding links, and
//! playing or queueing results.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use amp_domain::Track;

impl MultiplayGuiApp {
    /// Adds a pasted link (YouTube, Bandcamp, Spotify, stream URL). Ignored if it is not one.
    pub fn ingest_url(&mut self, url: &str) {
        if let Some(track) = amp_domain::source_url::track_from_link(url) {
            self.playlist.push(track);
            self.save_session();
        }
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
        if amp_domain::source_url::looks_like_link(&q) {
            self.ingest_url(&q);
            self.search_query_text.clear();
            self.search_results.clear();
            self.search_status = None;
            self.is_searching = false;
            return;
        }
        let enabled = self.enabled_providers();
        if enabled.is_empty() {
            self.search_results.clear();
            self.is_searching = false;
            self.search_status = Some(
                "Online search is off. Switch on YouTube or Bandcamp in the Accounts tab of Preferences."
                    .into(),
            );
            return;
        }
        self.is_searching = true;
        self.search_status = None;
        let job = crate::remote_lookup::LookupJob::OmniSearch { query: q, enabled };
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
