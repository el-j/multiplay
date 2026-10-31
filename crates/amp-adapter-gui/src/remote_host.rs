//! Host side of phone requests (TASK-216): previews with the host's own resolvers, moderation
//! into the playlist or a deck, trusted auto-accept, and status updates back to the phone.
//! Nothing here plays a track; accepted requests only join the playlist.

#![forbid(unsafe_code)]

use crate::app_state::MultiplayGuiApp;
use crate::remote_lookup::{LookupJob, LookupResult, LookupServices, RemoteLookup};
use crate::remote_requests::{
    display_title, library_matches, requested_copy, Moderation, Preview, RequestRow,
};
use amp_adapter_remote::{
    DeviceId, HostRequest, RequestId, RequestStatus, SearchItem, TrackReference, TrackRequest,
    ValidReference,
};
use amp_domain::DeckAssignment;

/// Library hits sent to a phone per search.
const LIBRARY_RESULTS: usize = 12;

impl MultiplayGuiApp {
    /// Per-frame work: takes new requests from phones, applies finished lookups and tells
    /// phones when their accepted track starts playing.
    pub fn handle_remote_requests(&mut self) {
        let generation = self.remote.generation;
        self.remote_requests
            .rows
            .retain(|r| r.generation == generation);
        while let Some(request) = self.remote.poll_request() {
            match request {
                HostRequest::Track(r) => self.on_track_request(r),
                HostRequest::Search { device, query } => self.on_phone_search(device, query),
            }
        }
        let results = self
            .remote_lookup
            .as_ref()
            .map(RemoteLookup::drain)
            .unwrap_or_default();
        for result in results {
            self.on_lookup_result(result);
        }
        self.report_playing();
    }

    /// Applies the host's decision on request `id`. False if the request is unknown, already
    /// answered, or cannot be accepted yet (its preview is not ready).
    pub fn moderate(&mut self, id: RequestId, action: Moderation) -> bool {
        let Some(row) = self.remote_requests.take(id) else {
            return false;
        };
        if action == Moderation::Reject {
            self.remote
                .update_request(id, RequestStatus::Rejected, None);
            return true;
        }
        let Preview::Ready(track) = &row.preview else {
            self.remote_requests.rows.push(row);
            self.remote_requests.rows.sort_by_key(|r| r.id);
            return false;
        };
        let track = (**track).clone();
        let title = display_title(&track);
        match action {
            Moderation::PlayNext if !self.playlist.is_empty() => {
                let at = (self.selected_track_idx + 1).min(self.playlist.len());
                self.playlist.insert(at, track.clone());
                if self
                    .audio_driver
                    .is_playing
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    let upcoming = self.playlist.get(self.selected_track_idx + 1).cloned();
                    self.audio_driver.set_next_track(upcoming);
                }
            }
            Moderation::Deck(deck) => {
                let assign = if deck == 0 {
                    DeckAssignment::A
                } else {
                    DeckAssignment::B
                };
                self.deck_assign.insert(track.id, assign);
                self.playlist.push(track.clone());
            }
            _ => self.playlist.push(track.clone()),
        }
        self.requested_by.insert(track.id, row.device_name);
        self.request_ids.insert(track.id, id);
        self.save_session();
        self.remote
            .update_request(id, RequestStatus::Accepted, Some(title));
        true
    }

    /// Lookup thread, started on first use with the injected or the real providers.
    pub(crate) fn lookup(&mut self) -> &RemoteLookup {
        if self.remote_lookup.is_none() {
            let services = self
                .lookup_services
                .take()
                .unwrap_or_else(LookupServices::online);
            self.remote_lookup = Some(RemoteLookup::spawn(services));
        }
        self.remote_lookup
            .as_ref()
            .expect("lookup thread just started")
    }

    fn providers_enabled(&self) -> bool {
        self.config.streaming.enable_youtube
    }

    fn on_track_request(&mut self, r: TrackRequest) {
        let preview = match &r.reference {
            ValidReference::Library(id) => match self.playlist.iter().find(|t| t.id == *id) {
                Some(t) => Preview::Ready(Box::new(requested_copy(t))),
                None => return self.refuse(r.id),
            },
            ValidReference::Search(q) => match library_matches(&self.playlist, q, 1).first() {
                Some(t) => Preview::Ready(Box::new(requested_copy(t))),
                None if self.providers_enabled() => Preview::Resolving,
                None => return self.refuse(r.id),
            },
            ValidReference::YouTube { .. } => Preview::Resolving,
            ValidReference::Upload { path, name } => {
                Preview::Ready(Box::new(uploaded_track(path, name, &r.device_name)))
            }
        };
        if preview == Preview::Resolving {
            let job = LookupJob::Preview {
                id: r.id,
                reference: r.reference.clone(),
                providers: self.providers_enabled(),
            };
            self.lookup().submit(job);
        }
        let ready = matches!(preview, Preview::Ready(_));
        self.remote_requests.rows.push(RequestRow {
            id: r.id,
            device: r.device,
            device_name: r.device_name,
            reference: r.reference,
            preview,
            auto_accept: r.trusted,
            generation: self.remote.generation,
        });
        if ready && r.trusted {
            self.moderate(r.id, Moderation::AddToEnd);
        }
    }

    fn refuse(&mut self, id: RequestId) {
        self.remote
            .update_request(id, RequestStatus::Rejected, None);
    }

    fn on_phone_search(&mut self, device: DeviceId, query: String) {
        let library: Vec<SearchItem> = library_matches(&self.playlist, &query, LIBRARY_RESULTS)
            .into_iter()
            .map(|t| SearchItem {
                title: t.metadata.title.clone(),
                artist: t.metadata.artist.clone(),
                duration_secs: t.metadata.duration.map(|d| d.as_secs()),
                source: "library".into(),
                reference: TrackReference::Library(t.id),
            })
            .collect();
        if self.providers_enabled() {
            self.lookup().submit(LookupJob::Search {
                device,
                query,
                library,
            });
        } else {
            self.remote.send_search_results(&device, &query, library);
        }
    }

    fn on_lookup_result(&mut self, result: LookupResult) {
        match result {
            LookupResult::Preview { id, result } => {
                let Some(row) = self.remote_requests.get_mut(id) else {
                    return;
                };
                row.preview = match result {
                    Ok(track) => Preview::Ready(track),
                    Err(why) => Preview::Failed(why),
                };
                let auto = row.auto_accept && matches!(row.preview, Preview::Ready(_));
                if auto {
                    self.moderate(id, Moderation::AddToEnd);
                }
            }
            LookupResult::Search {
                device,
                query,
                items,
            } => self.remote.send_search_results(&device, &query, items),
            LookupResult::OmniSearch {
                query,
                tracks,
                failures,
            } => {
                if self.search_query_text.trim() == query {
                    self.search_results = tracks;
                    self.search_status = (!failures.is_empty()).then(|| failures.join("\n"));
                    self.is_searching = false;
                }
            }
        }
    }

    /// Sends "playing" once when a requested track starts.
    fn report_playing(&mut self) {
        let current = self
            .audio_driver
            .current_track
            .lock()
            .ok()
            .and_then(|t| t.as_ref().map(|t| t.id));
        if current == self.last_playing {
            return;
        }
        self.last_playing = current;
        if let Some(request) = current.and_then(|id| self.request_ids.remove(&id)) {
            self.remote
                .update_request(request, RequestStatus::Playing, None);
        }
    }
}

/// Playlist entry for an uploaded file: its own tags when it has them, else the phone's file
/// name (the stored name is random) and the phone as artist.
fn uploaded_track(path: &std::path::Path, name: &str, device: &str) -> amp_domain::Track {
    let stored_stem = path.file_stem().map(|s| s.to_string_lossy().into_owned());
    let title = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let mut track = amp_adapter_storage::extract_track_from_file(path).unwrap_or_else(|| {
        amp_domain::Track::new(
            amp_domain::AudioSource::LocalFile {
                path: path.to_string_lossy().into_owned(),
            },
            amp_domain::TrackMetadata::new(title, device),
        )
    });
    if Some(&track.metadata.title) == stored_stem.as_ref() || track.metadata.title.is_empty() {
        track.metadata.title = title.to_string();
        if track.metadata.artist.is_empty() || track.metadata.artist.starts_with("Unknown") {
            track.metadata.artist = device.to_string();
        }
    }
    track
}
