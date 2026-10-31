//! Background analysis with a cache: reuse a stored result of the current version, otherwise
//! wait for the deck's track to finish decoding, analyse it and store the result.

#![forbid(unsafe_code)]

use super::analyze_track_buffer;
use crate::dj::engine::TrackBuffer;
use amp_domain::{TrackAnalysis, ANALYSIS_VERSION};
use amp_ports::AnalysisCachePort;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// Starts analysing `track` on a background thread. The returned cell is filled when the result
/// is known (from `cache` under `key`, or freshly analysed and then stored). It stays empty when
/// loading failed or the track was replaced first.
pub fn spawn_analysis(
    key: String,
    track: Arc<TrackBuffer>,
    cache: Option<Arc<dyn AnalysisCachePort>>,
) -> Arc<OnceLock<TrackAnalysis>> {
    let cell = Arc::new(OnceLock::new());
    let out = Arc::clone(&cell);
    let _ = std::thread::Builder::new()
        .name("multiplay-dj-analysis".into())
        .spawn(move || {
            if let Some(hit) = cache
                .as_ref()
                .and_then(|c| c.load(&key))
                .filter(|a| a.version == ANALYSIS_VERSION)
            {
                let _ = out.set(hit);
                return;
            }
            while !track.is_complete() {
                if track.is_cancelled() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            if track.error().is_some() && track.loaded_frames() == 0 {
                return;
            }
            if let Some(a) = analyze_track_buffer(&track) {
                if let Some(c) = &cache {
                    let _ = c.store(&key, &a);
                }
                let _ = out.set(a);
            }
        });
    cell
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::time::Instant;

    #[derive(Default)]
    struct MemoryCache {
        map: Mutex<HashMap<String, TrackAnalysis>>,
        stores: AtomicUsize,
    }

    impl AnalysisCachePort for MemoryCache {
        fn load(&self, key: &str) -> Option<TrackAnalysis> {
            self.map.lock().ok()?.get(key).cloned()
        }
        fn store(&self, key: &str, a: &TrackAnalysis) -> Result<(), amp_ports::StorageError> {
            self.stores.fetch_add(1, Ordering::Relaxed);
            self.map
                .lock()
                .map_err(|e| amp_ports::StorageError::TransactionFailed(e.to_string()))?
                .insert(key.into(), a.clone());
            Ok(())
        }
    }

    fn wait(cell: &OnceLock<TrackAnalysis>) -> Option<&TrackAnalysis> {
        let start = Instant::now();
        while cell.get().is_none() && start.elapsed() < Duration::from_secs(20) {
            std::thread::sleep(Duration::from_millis(10));
        }
        cell.get()
    }

    fn silence() -> Arc<TrackBuffer> {
        TrackBuffer::from_samples(44_100, &vec![0.0; 44_100 * 2])
    }

    #[test]
    fn dj_analysis_is_stored_once_and_then_read_from_the_cache() {
        let cache = Arc::new(MemoryCache::default());
        let dyn_cache: Arc<dyn AnalysisCachePort> = cache.clone();
        let first = spawn_analysis("file:/a".into(), silence(), Some(Arc::clone(&dyn_cache)));
        assert!(wait(&first).is_some());
        assert_eq!(cache.stores.load(Ordering::Relaxed), 1);
        let again = spawn_analysis("file:/a".into(), silence(), Some(dyn_cache));
        assert_eq!(wait(&again), wait(&first));
        assert_eq!(
            cache.stores.load(Ordering::Relaxed),
            1,
            "cache hit, no re-analysis"
        );
    }

    #[test]
    fn dj_analysis_of_an_older_version_is_redone() {
        let cache = Arc::new(MemoryCache::default());
        let mut old = super::super::analyze(44_100, &[0.0; 64]);
        old.version = ANALYSIS_VERSION - 1;
        cache.store("file:/b", &old).unwrap();
        let dyn_cache: Arc<dyn AnalysisCachePort> = cache.clone();
        let cell = spawn_analysis("file:/b".into(), silence(), Some(dyn_cache));
        assert_eq!(wait(&cell).map(|a| a.version), Some(ANALYSIS_VERSION));
        assert_eq!(cache.stores.load(Ordering::Relaxed), 2);
    }
}
