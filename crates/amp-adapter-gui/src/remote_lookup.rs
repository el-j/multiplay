//! Background lookups for phone requests (TASK-216/217): previews of requested links and
//! provider searches run on the host's own resolvers, off the UI thread.

#![forbid(unsafe_code)]

use amp_adapter_remote::{DeviceId, RequestId, SearchItem, TrackReference, ValidReference};
use amp_domain::{AudioSource, Track};
use amp_ports::{ProviderSearchPort, TrackLookupPort};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

/// Longest a single lookup or provider search may take.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(15);
/// Provider results sent to a phone per search.
const PROVIDER_RESULTS: usize = 8;

/// The resolvers lookups use. Tests swap in fixtures; the app uses the real providers.
#[derive(Clone)]
pub struct LookupServices {
    /// Metadata lookups for requested links.
    pub lookups: Vec<Arc<dyn TrackLookupPort>>,
    /// Providers searched for phone searches and unmatched search requests.
    pub providers: Vec<Arc<dyn ProviderSearchPort>>,
}

impl LookupServices {
    /// The host's real providers (YouTube and Bandcamp).
    #[must_use]
    pub fn online() -> Self {
        let yt = Arc::new(amp_adapter_youtube::PureYouTubeAdapter::default());
        let bc = Arc::new(amp_adapter_bandcamp::PureBandcampAdapter::default());
        Self {
            lookups: vec![yt.clone(), bc.clone()],
            providers: vec![yt, bc],
        }
    }
}

/// Work for the lookup thread.
#[derive(Debug, Clone)]
pub enum LookupJob {
    /// Find what request `id` would add, asking only the `enabled` providers.
    Preview {
        id: RequestId,
        reference: ValidReference,
        enabled: Vec<&'static str>,
    },
    /// Answer a phone's search: `library` hits first, then results of the `enabled` providers.
    Search {
        device: DeviceId,
        query: String,
        library: Vec<SearchItem>,
        enabled: Vec<&'static str>,
    },
    /// The search window: every `enabled` provider (switched on in the Accounts tab of Preferences).
    OmniSearch {
        query: String,
        enabled: Vec<&'static str>,
    },
}

/// A finished job.
#[derive(Debug)]
pub enum LookupResult {
    Preview {
        id: RequestId,
        /// Boxed: a track is large next to the other results.
        result: Result<Box<Track>, String>,
    },
    Search {
        device: DeviceId,
        query: String,
        items: Vec<SearchItem>,
    },
    OmniSearch {
        query: String,
        tracks: Vec<Track>,
        /// One line per provider that failed, e.g. "Bandcamp: \<reason\>".
        failures: Vec<String>,
    },
}

/// Handle to the lookup thread; dropping it ends the thread.
pub struct RemoteLookup {
    jobs: Sender<LookupJob>,
    results: Receiver<LookupResult>,
}

impl RemoteLookup {
    /// Starts the lookup thread.
    #[must_use]
    pub fn spawn(services: LookupServices) -> Self {
        let (jobs, job_rx) = mpsc::channel::<LookupJob>();
        let (result_tx, results) = mpsc::channel();
        let _ = std::thread::Builder::new()
            .name("multiplay-remote-lookup".into())
            .spawn(move || {
                let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                for job in job_rx {
                    let result = rt.block_on(run(&services, job));
                    if result_tx.send(result).is_err() {
                        return;
                    }
                }
            });
        Self { jobs, results }
    }

    pub fn submit(&self, job: LookupJob) {
        let _ = self.jobs.send(job);
    }

    /// Finished jobs since the last call.
    #[must_use]
    pub fn drain(&self) -> Vec<LookupResult> {
        self.results.try_iter().collect()
    }
}

async fn run(services: &LookupServices, job: LookupJob) -> LookupResult {
    match job {
        LookupJob::Preview {
            id,
            reference,
            enabled,
        } => LookupResult::Preview {
            id,
            result: preview(services, &reference, &enabled).await.map(Box::new),
        },
        LookupJob::Search {
            device,
            query,
            mut library,
            enabled,
        } => {
            for (provider, track) in search_providers(services, &query, &enabled).await {
                if let Some(link) = crate::remote_requests::shareable_link(&track.source) {
                    library.push(SearchItem {
                        title: track.metadata.title.clone(),
                        artist: track.metadata.artist.clone(),
                        duration_secs: track.metadata.duration.map(|d| d.as_secs()),
                        source: provider.to_string(),
                        reference: TrackReference::Link(link),
                    });
                }
            }
            LookupResult::Search {
                device,
                query,
                items: library,
            }
        }
        LookupJob::OmniSearch { query, enabled } => {
            let (hits, failures) = search_each_provider(services, &query, &enabled).await;
            let tracks = hits.into_iter().map(|(_, t)| t).collect();
            LookupResult::OmniSearch {
                query,
                tracks,
                failures,
            }
        }
    }
}

async fn preview(
    services: &LookupServices,
    reference: &ValidReference,
    enabled: &[&'static str],
) -> Result<Track, String> {
    match reference {
        ValidReference::YouTube { .. } if !enabled.contains(&"youtube") => {
            Err("YouTube is switched off in the Accounts tab of Preferences".into())
        }
        ValidReference::YouTube { video_id } => {
            let source = AudioSource::YouTube {
                video_id: video_id.clone(),
            };
            let lookup = services
                .lookups
                .iter()
                .find(|l| l.can_lookup(&source))
                .ok_or("No resolver for this link")?;
            match tokio::time::timeout(LOOKUP_TIMEOUT, lookup.lookup(&source)).await {
                Ok(Ok(metadata)) => Ok(Track::new(source, metadata)),
                Ok(Err(e)) => Err(format!("Not found: {e}")),
                Err(_) => Err("The lookup timed out".into()),
            }
        }
        ValidReference::Search(query) if !enabled.is_empty() => {
            search_providers(services, query, enabled)
                .await
                .into_iter()
                .map(|(_, t)| t)
                .next()
                .ok_or_else(|| format!("Nothing found for “{query}”"))
        }
        ValidReference::Search(query) => Err(format!("Nothing in the library matches “{query}”")),
        ValidReference::Library(_) | ValidReference::Upload { .. } => {
            Err("Not in the library".into())
        }
    }
}

/// Results of every provider, tagged with the provider's id.
async fn search_providers(
    services: &LookupServices,
    query: &str,
    enabled: &[&'static str],
) -> Vec<(&'static str, Track)> {
    search_each_provider(services, query, enabled).await.0
}

/// Results of every provider, plus a line for each one that failed or timed out.
async fn search_each_provider(
    services: &LookupServices,
    query: &str,
    enabled: &[&'static str],
) -> (Vec<(&'static str, Track)>, Vec<String>) {
    let mut hits = Vec::new();
    let mut failures = Vec::new();
    for provider in &services.providers {
        let id = provider.provider_id();
        if !enabled.contains(&id) {
            continue;
        }
        match tokio::time::timeout(LOOKUP_TIMEOUT, provider.query(query)).await {
            Ok(Ok(tracks)) => {
                hits.extend(tracks.into_iter().take(PROVIDER_RESULTS).map(|t| (id, t)));
            }
            Ok(Err(e)) => {
                tracing::warn!(provider = id, "search failed: {e}");
                failures.push(format!("{}: {e}", provider_name(id)));
            }
            Err(_) => {
                tracing::warn!(provider = id, "search timed out after {LOOKUP_TIMEOUT:?}");
                failures.push(format!("{}: no answer in time", provider_name(id)));
            }
        }
    }
    (hits, failures)
}

/// "bandcamp" -> "Bandcamp", for messages shown to the user.
fn provider_name(id: &str) -> String {
    match id {
        "youtube" => "YouTube".into(),
        other => {
            let mut c = other.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect())
                .unwrap_or_default()
        }
    }
}
