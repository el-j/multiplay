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
    /// The host's real providers (YouTube today).
    #[must_use]
    pub fn online() -> Self {
        let yt = Arc::new(amp_adapter_youtube::PureYouTubeAdapter::default());
        Self {
            lookups: vec![yt.clone()],
            providers: vec![yt],
        }
    }
}

/// Work for the lookup thread.
#[derive(Debug, Clone)]
pub enum LookupJob {
    /// Find what request `id` would add. `providers` allows searching providers for a query.
    Preview {
        id: RequestId,
        reference: ValidReference,
        providers: bool,
    },
    /// Answer a phone's search: `library` hits first, then provider results.
    Search {
        device: DeviceId,
        query: String,
        library: Vec<SearchItem>,
    },
}

/// A finished job.
#[derive(Debug)]
pub enum LookupResult {
    Preview {
        id: RequestId,
        result: Result<Track, String>,
    },
    Search {
        device: DeviceId,
        query: String,
        items: Vec<SearchItem>,
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
            providers,
        } => LookupResult::Preview {
            id,
            result: preview(services, &reference, providers).await,
        },
        LookupJob::Search {
            device,
            query,
            mut library,
        } => {
            for (provider, track) in search_providers(services, &query).await {
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
    }
}

async fn preview(
    services: &LookupServices,
    reference: &ValidReference,
    providers: bool,
) -> Result<Track, String> {
    match reference {
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
        ValidReference::Search(query) if providers => search_providers(services, query)
            .await
            .into_iter()
            .map(|(_, t)| t)
            .next()
            .ok_or_else(|| format!("Nothing found for “{query}”")),
        ValidReference::Search(query) => Err(format!("Nothing in the library matches “{query}”")),
        ValidReference::Library(_) | ValidReference::Upload { .. } => {
            Err("Not in the library".into())
        }
    }
}

/// Results of every provider, tagged with the provider's id.
async fn search_providers(services: &LookupServices, query: &str) -> Vec<(&'static str, Track)> {
    let mut out = Vec::new();
    for provider in &services.providers {
        if let Ok(Ok(tracks)) = tokio::time::timeout(LOOKUP_TIMEOUT, provider.query(query)).await {
            let id = provider.provider_id();
            out.extend(tracks.into_iter().take(PROVIDER_RESULTS).map(|t| (id, t)));
        }
    }
    out
}
