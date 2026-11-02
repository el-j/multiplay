//! Parallel multi-source search coordinator with dynamic source exclusion filters and timeouts.

#![forbid(unsafe_code)]

use amp_domain::source_badge::SourceKind;
use amp_domain::Track;
use amp_ports::driven::{ProviderError, ProviderSearchPort};
use amp_ports::driving::{FilteredSearchPort, ProviderOutcome, SearchQueryOptions};
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;

pub struct FilteredSearchCoordinator {
    providers: Vec<(SourceKind, Arc<dyn ProviderSearchPort>)>,
}

impl FilteredSearchCoordinator {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Registers a provider with its associated SourceKind badge.
    pub fn register_provider(&mut self, kind: SourceKind, provider: Arc<dyn ProviderSearchPort>) {
        self.providers.push((kind, provider));
    }

    pub fn providers_count(&self) -> usize {
        self.providers.len()
    }
}

impl Default for FilteredSearchCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FilteredSearchPort for FilteredSearchCoordinator {
    async fn search(&self, options: SearchQueryOptions) -> Vec<Track> {
        let outcomes = self.search_outcomes(options).await;
        let mut aggregated = Vec::new();
        for outcome in outcomes {
            if let Ok(tracks) = outcome.result {
                aggregated.extend(tracks);
            }
        }
        aggregated
    }

    async fn search_outcomes(&self, options: SearchQueryOptions) -> Vec<ProviderOutcome> {
        let mut join_set = JoinSet::new();
        let mut pending_kinds = Vec::new();

        // Spawn parallel search tasks ONLY for enabled sources
        for (kind, provider) in &self.providers {
            if !options.filter_mask.is_enabled(*kind) {
                continue;
            }

            let p = Arc::clone(provider);
            let q = options.text.clone();
            let k = *kind;
            pending_kinds.push(k);

            join_set.spawn(async move {
                let start = Instant::now();
                let result = p.query(&q).await;
                let latency = start.elapsed();
                (k, result, latency)
            });
        }

        let mut outcomes = Vec::new();
        let mut completed_kinds = HashSet::new();
        let timeout_dur = Duration::from_millis(options.timeout_millis);
        let timeout_sleep = tokio::time::sleep(timeout_dur);
        tokio::pin!(timeout_sleep);

        loop {
            tokio::select! {
                next_res = join_set.join_next() => {
                    match next_res {
                        Some(Ok((kind, result, latency))) => {
                            completed_kinds.insert(kind);
                            outcomes.push(ProviderOutcome {
                                kind,
                                result,
                                latency,
                            });
                        }
                        Some(Err(_join_err)) => {}
                        None => break,
                    }
                }
                _ = &mut timeout_sleep => {
                    join_set.abort_all();
                    break;
                }
            }
        }

        // Flag any unfinished provider as timed out
        for kind in pending_kinds {
            if !completed_kinds.contains(&kind) {
                outcomes.push(ProviderOutcome {
                    kind,
                    result: Err(ProviderError::Timeout),
                    latency: timeout_dur,
                });
            }
        }

        outcomes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::{AudioSource, SourceFilterMask, TrackMetadata};
    use amp_ports::driven::ProviderError;

    struct MockProvider {
        id: &'static str,
        results: Vec<Track>,
        delay_ms: u64,
    }

    #[async_trait]
    impl ProviderSearchPort for MockProvider {
        fn provider_id(&self) -> &'static str {
            self.id
        }

        async fn query(&self, _query: &str) -> Result<Vec<Track>, ProviderError> {
            if self.delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
            }
            Ok(self.results.clone())
        }
    }

    #[tokio::test]
    async fn test_filtered_search_exclusion_and_timeout() {
        let mut coord = FilteredSearchCoordinator::new();

        let track_yt = Track::new(
            AudioSource::YouTube {
                video_id: "yt1".into(),
            },
            TrackMetadata::new("YT Song", "YT Artist"),
        );
        let track_spot = Track::new(
            AudioSource::SoundCloud {
                track_url: "spot1".into(),
            },
            TrackMetadata::new("Spot Song", "Spot Artist"),
        );

        coord.register_provider(
            SourceKind::YouTube,
            Arc::new(MockProvider {
                id: "YouTube",
                results: vec![track_yt],
                delay_ms: 10,
            }),
        );
        coord.register_provider(
            SourceKind::SoundCloud,
            Arc::new(MockProvider {
                id: "SoundCloud",
                results: vec![track_spot],
                delay_ms: 500, // Slow provider
            }),
        );

        // 1. Search with SoundCloud excluded
        let mut filter = SourceFilterMask::default();
        filter.toggle(SourceKind::SoundCloud);

        let res = coord
            .search(SearchQueryOptions {
                text: "test".into(),
                filter_mask: filter,
                timeout_millis: 200,
            })
            .await;

        assert_eq!(res.len(), 1);
        assert_eq!(res[0].metadata.title, "YT Song");

        // 2. Search with fast timeout that aborts slow provider
        let full_filter = SourceFilterMask::default();
        let res2 = coord
            .search(SearchQueryOptions {
                text: "test".into(),
                filter_mask: full_filter,
                timeout_millis: 50, // Shorter than SoundCloud's 500ms
            })
            .await;

        assert_eq!(res2.len(), 1);
        assert_eq!(res2[0].metadata.title, "YT Song");
    }

    struct FailingProvider {
        error: ProviderError,
    }

    #[async_trait]
    impl ProviderSearchPort for FailingProvider {
        fn provider_id(&self) -> &'static str {
            "failing"
        }

        async fn query(&self, _query: &str) -> Result<Vec<Track>, ProviderError> {
            Err(self.error.clone())
        }
    }

    #[tokio::test]
    async fn test_failing_provider_surfaces_error_without_hiding_other_results() {
        let mut coord = FilteredSearchCoordinator::new();

        let track_yt = Track::new(
            AudioSource::YouTube {
                video_id: "good_yt".into(),
            },
            TrackMetadata::new("Good Song", "Good Artist"),
        );

        coord.register_provider(
            SourceKind::YouTube,
            Arc::new(MockProvider {
                id: "YouTube",
                results: vec![track_yt],
                delay_ms: 5,
            }),
        );
        coord.register_provider(
            SourceKind::SoundCloud,
            Arc::new(FailingProvider {
                error: ProviderError::RateLimited,
            }),
        );
        coord.register_provider(
            SourceKind::Bandcamp,
            Arc::new(MockProvider {
                id: "Bandcamp",
                results: vec![],
                delay_ms: 200, // Timeout trigger
            }),
        );

        let outcomes = coord
            .search_outcomes(SearchQueryOptions {
                text: "test".into(),
                filter_mask: SourceFilterMask::default(),
                timeout_millis: 50,
            })
            .await;

        assert_eq!(outcomes.len(), 3);

        let yt_outcome = outcomes
            .iter()
            .find(|o| o.kind == SourceKind::YouTube)
            .unwrap();
        assert!(yt_outcome.result.is_ok());
        assert_eq!(yt_outcome.status_chip().0, "✓");

        let sc_outcome = outcomes
            .iter()
            .find(|o| o.kind == SourceKind::SoundCloud)
            .unwrap();
        assert_eq!(sc_outcome.result, Err(ProviderError::RateLimited));
        assert_eq!(sc_outcome.status_chip(), ("⚠", "Rate limited"));

        let bc_outcome = outcomes
            .iter()
            .find(|o| o.kind == SourceKind::Bandcamp)
            .unwrap();
        assert_eq!(bc_outcome.result, Err(ProviderError::Timeout));
        assert_eq!(bc_outcome.status_chip(), ("⏳", "Timed out"));

        // `search()` flattens successfully returned results without erroring
        let tracks = coord
            .search(SearchQueryOptions {
                text: "test".into(),
                filter_mask: SourceFilterMask::default(),
                timeout_millis: 50,
            })
            .await;
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].metadata.title, "Good Song");
    }
}
