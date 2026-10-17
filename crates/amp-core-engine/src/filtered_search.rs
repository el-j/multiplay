//! Parallel multi-source search coordinator with dynamic source exclusion filters and timeouts.

#![forbid(unsafe_code)]

use amp_domain::source_badge::SourceKind;
use amp_domain::Track;
use amp_ports::driven::ProviderSearchPort;
use amp_ports::driving::{FilteredSearchPort, SearchQueryOptions};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
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
        let mut join_set = JoinSet::new();

        // Spawn parallel search tasks ONLY for enabled sources
        for (kind, provider) in &self.providers {
            if !options.filter_mask.is_enabled(*kind) {
                continue; // Skip excluded provider
            }

            let p = Arc::clone(provider);
            let q = options.text.clone();
            let k = *kind;

            join_set.spawn(async move {
                let results = p.query(&q).await.unwrap_or_default();
                (k, results)
            });
        }

        let mut aggregated = Vec::new();
        let timeout_sleep = tokio::time::sleep(Duration::from_millis(options.timeout_millis));
        tokio::pin!(timeout_sleep);

        loop {
            tokio::select! {
                next_res = join_set.join_next() => {
                    match next_res {
                        Some(Ok((_kind, tracks))) => {
                            aggregated.extend(tracks);
                        }
                        Some(Err(join_err)) => {
                            eprintln!("Search provider task error: {join_err}");
                        }
                        None => break, // All spawned tasks completed
                    }
                }
                _ = &mut timeout_sleep => {
                    join_set.abort_all();
                    break;
                }
            }
        }

        aggregated
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
            AudioSource::Spotify {
                track_uri: "spot1".into(),
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
            SourceKind::Spotify,
            Arc::new(MockProvider {
                id: "Spotify",
                results: vec![track_spot],
                delay_ms: 500, // Slow provider
            }),
        );

        // 1. Search with Spotify excluded
        let mut filter = SourceFilterMask::default();
        filter.toggle(SourceKind::Spotify);

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
                timeout_millis: 50, // Shorter than Spotify's 500ms
            })
            .await;

        assert_eq!(res2.len(), 1);
        assert_eq!(res2[0].metadata.title, "YT Song");
    }
}
