//! The search window's omnibox: a provider that fails says so instead of looking like "no
//! results", and a link typed or pasted into the search box is added, not searched for.

#![forbid(unsafe_code)]

use amp_adapter_gui::remote_lookup::LookupServices;
use amp_adapter_gui::MultiplayGuiApp;
use amp_domain::{AudioSource, Track, TrackMetadata};
use amp_ports::{async_trait, ProviderError, ProviderSearchPort};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Works;

#[async_trait]
impl ProviderSearchPort for Works {
    fn provider_id(&self) -> &'static str {
        "youtube"
    }
    async fn query(&self, _query: &str) -> Result<Vec<Track>, ProviderError> {
        Ok(vec![Track::new(
            AudioSource::YouTube {
                video_id: "dQw4w9WgXcQ".into(),
            },
            TrackMetadata::new("Never Gonna Give You Up", "Rick Astley"),
        )])
    }
}

struct Broken;

#[async_trait]
impl ProviderSearchPort for Broken {
    fn provider_id(&self) -> &'static str {
        "bandcamp"
    }
    async fn query(&self, _query: &str) -> Result<Vec<Track>, ProviderError> {
        Err(ProviderError::Unavailable(
            "Bandcamp search error: bad function".into(),
        ))
    }
}

struct Root(PathBuf);

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn app() -> (Root, MultiplayGuiApp) {
    let dir = std::env::temp_dir().join(format!("multiplay-omnibox-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut app = MultiplayGuiApp::headless(&dir).expect("headless app");
    app.playlist.clear();
    app.lookup_services = Some(LookupServices {
        lookups: vec![],
        providers: vec![Arc::new(Works), Arc::new(Broken)],
    });
    (Root(dir), app)
}

fn wait_for_search(app: &mut MultiplayGuiApp) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.is_searching && Instant::now() < deadline {
        app.handle_remote_requests();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!app.is_searching, "search never finished");
}

#[test]
fn a_failing_provider_is_reported_next_to_the_other_results() {
    let (_root, mut app) = app();
    app.search_query_text = "never gonna".into();
    app.trigger_gui_search();
    wait_for_search(&mut app);
    assert_eq!(
        app.search_results.len(),
        1,
        "working providers still answer"
    );
    let status = app.search_status.clone().unwrap_or_default();
    assert!(
        status.contains("Bandcamp") && status.contains("bad function"),
        "status: {status:?}"
    );
}

#[test]
fn a_link_in_the_search_box_is_added_instead_of_searched() {
    let (_root, mut app) = app();
    app.search_query_text = "  https://www.youtube.com/watch?v=dQw4w9WgXcQ  ".into();
    app.trigger_gui_search();
    assert!(!app.is_searching, "a link is not a search");
    assert_eq!(
        app.playlist.last().map(|t| t.source.clone()),
        Some(AudioSource::YouTube {
            video_id: "dQw4w9WgXcQ".into()
        })
    );
    assert!(
        app.search_query_text.is_empty(),
        "the box is ready for the next link"
    );
}

#[test]
fn plain_words_are_still_searched() {
    let (_root, mut app) = app();
    app.search_query_text = "daft punk".into();
    app.trigger_gui_search();
    assert!(app.is_searching);
    assert!(app.playlist.is_empty());
}
