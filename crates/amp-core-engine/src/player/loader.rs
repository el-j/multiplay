//! Non-blocking stream resolution, cancellation, and lookahead preloading (`TASK-129`).
//!
//! Resolves `AudioSource` streams on Tokio tasks with per-provider timeouts, cancellation
//! handles, and bounded exponential backoff for lookahead preloading.

#![forbid(unsafe_code)]

use crate::events::{PlayerErrorKind, PlayerEvent};
use crate::EventBus;
use amp_domain::{AudioSource, Track};
use amp_ports::{
    DecodeError, DecoderFactoryPort, PcmStreamPort, ProviderError, StreamResolverPort,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::task::AbortHandle;

/// Default per-provider stream resolution timeout (10 seconds).
pub const DEFAULT_RESOLVE_TIMEOUT: Duration = Duration::from_secs(10);

/// Default maximum retries for lookahead preloading on transient network errors.
pub const DEFAULT_PRELOAD_MAX_RETRIES: u32 = 3;

/// Shared fallback Tokio runtime used only when callers construct `PlayerEngine` outside an
/// active Tokio context (e.g. synchronous unit tests). Composition roots pass their own handle.
static FALLBACK_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub fn default_runtime_handle() -> tokio::runtime::Handle {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        return handle;
    }
    FALLBACK_RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .thread_name("multiplay-engine-rt")
                .build()
                .expect("Failed to build fallback Tokio runtime")
        })
        .handle()
        .clone()
}

/// Outcome of an asynchronous track or preload open operation.
pub struct OpenedTrack {
    pub track: Track,
    pub decoder: Box<dyn PcmStreamPort>,
    pub duration: Option<Duration>,
}

/// Cancellation-aware handle to an in-flight asynchronous track or preload load task.
pub struct PendingLoad {
    pub track: Track,
    cancelled: Arc<AtomicBool>,
    abort_handle: Option<AbortHandle>,
    result: Arc<Mutex<Option<Result<OpenedTrack, DecodeError>>>>,
}

impl PendingLoad {
    /// Cancels the in-flight resolution/open task immediately.
    pub fn cancel(&mut self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(handle) = self.abort_handle.take() {
            handle.abort();
        }
    }

    /// Polls whether the background task has finished without blocking the audio worker.
    pub fn try_take(&mut self) -> Option<Result<OpenedTrack, DecodeError>> {
        if self.cancelled.load(Ordering::Acquire) {
            return None;
        }
        self.result.try_lock().ok()?.take()
    }

    /// Blocks until the load completes or `timeout` elapses (used in deterministic tests).
    pub fn wait_timeout(&mut self, timeout: Duration) -> Option<Result<OpenedTrack, DecodeError>> {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            if self.cancelled.load(Ordering::Acquire) {
                return None;
            }
            if let Ok(mut slot) = self.result.lock() {
                if let Some(res) = slot.take() {
                    return Some(res);
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        None
    }
}

impl Drop for PendingLoad {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Spawns an asynchronous task to resolve and open `track` without blocking the audio worker.
pub fn spawn_track_load(
    rt: &tokio::runtime::Handle,
    track: Track,
    decoder_factory: Arc<dyn DecoderFactoryPort>,
    resolvers: Vec<Arc<dyn StreamResolverPort>>,
    resolve_timeout: Duration,
    max_retries: u32,
    event_bus: Option<Arc<EventBus>>,
) -> PendingLoad {
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancelled_task = Arc::clone(&cancelled);
    let result: Arc<Mutex<Option<Result<OpenedTrack, DecodeError>>>> = Arc::new(Mutex::new(None));
    let result_task = Arc::clone(&result);
    let track_for_task = track.clone();

    let join = rt.spawn(async move {
        let outcome = resolve_and_open_with_retries(
            track_for_task.clone(),
            decoder_factory,
            resolvers,
            resolve_timeout,
            max_retries,
            &cancelled_task,
        )
        .await;

        if cancelled_task.load(Ordering::Acquire) {
            return;
        }

        if let Err(ref err) = outcome {
            if let Some(bus) = &event_bus {
                bus.emit_event(PlayerEvent::Error {
                    kind: PlayerErrorKind::Decode,
                    source: Some(track_for_task.source.clone()),
                    message: err.to_string(),
                    recoverable: true,
                });
                bus.emit_event(PlayerEvent::ErrorNotification(err.to_string()));
            }
        }

        if let Ok(mut slot) = result_task.lock() {
            *slot = Some(outcome);
        }
    });

    PendingLoad {
        track,
        cancelled,
        abort_handle: Some(join.abort_handle()),
        result,
    }
}

/// Synchronously opens a track stream using `decoder_factory` and `resolvers` (used by DJ
/// handback / deck loader helper threads).
pub fn open_track_blocking(
    rt: &tokio::runtime::Handle,
    track: &Track,
    decoder_factory: &Arc<dyn DecoderFactoryPort>,
    resolvers: &[Arc<dyn StreamResolverPort>],
    resolve_timeout: Duration,
) -> Result<Box<dyn PcmStreamPort>, DecodeError> {
    let decoder = open_source_blocking(rt, track, decoder_factory, resolvers, resolve_timeout)?;
    super::segment::apply_segment(track, decoder)
}

fn open_source_blocking(
    rt: &tokio::runtime::Handle,
    track: &Track,
    decoder_factory: &Arc<dyn DecoderFactoryPort>,
    resolvers: &[Arc<dyn StreamResolverPort>],
    resolve_timeout: Duration,
) -> Result<Box<dyn PcmStreamPort>, DecodeError> {
    match &track.source {
        AudioSource::LocalFile { path } => decoder_factory.open_file(path),
        source => {
            for resolver in resolvers {
                if resolver.handles_source(source) {
                    let resolver = Arc::clone(resolver);
                    let src = source.clone();
                    let stream_res = if let Ok(current) = tokio::runtime::Handle::try_current() {
                        match current.runtime_flavor() {
                            tokio::runtime::RuntimeFlavor::MultiThread => {
                                tokio::task::block_in_place(|| {
                                    current.block_on(tokio::time::timeout(
                                        resolver.resolve_timeout().unwrap_or(resolve_timeout),
                                        resolver.resolve_stream(&src),
                                    ))
                                })
                            }
                            _ => rt.block_on(tokio::time::timeout(
                                resolver.resolve_timeout().unwrap_or(resolve_timeout),
                                resolver.resolve_stream(&src),
                            )),
                        }
                    } else {
                        rt.block_on(tokio::time::timeout(
                            resolver.resolve_timeout().unwrap_or(resolve_timeout),
                            resolver.resolve_stream(&src),
                        ))
                    };
                    let stream = match stream_res {
                        Ok(Ok(s)) => s,
                        Ok(Err(e)) => {
                            return Err(DecodeError::OpenFailed(format!(
                                "Failed to resolve stream for {source:?}: {e}"
                            )))
                        }
                        Err(_) => {
                            return Err(DecodeError::OpenFailed(format!(
                                "Timed out resolving stream for {source:?}"
                            )))
                        }
                    };
                    return decoder_factory.open_stream(stream);
                }
            }
            Err(no_resolver(source))
        }
    }
}

async fn resolve_and_open_with_retries(
    mut track: Track,
    decoder_factory: Arc<dyn DecoderFactoryPort>,
    resolvers: Vec<Arc<dyn StreamResolverPort>>,
    resolve_timeout: Duration,
    max_retries: u32,
    cancelled: &Arc<AtomicBool>,
) -> Result<OpenedTrack, DecodeError> {
    let mut attempt = 0u32;
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(DecodeError::OpenFailed("Track load cancelled".into()));
        }

        let res = resolve_and_open_once(
            &track,
            &decoder_factory,
            &resolvers,
            resolve_timeout,
            cancelled,
        )
        .await;

        match res {
            Ok(decoder) => {
                // A CUE-sheet track plays only its part of the file (TASK-232).
                let decoder = super::segment::apply_segment(&track, decoder)?;
                let duration = decoder.duration().or(track.metadata.duration);
                if track.metadata.duration.is_none() {
                    track.metadata.duration = duration;
                }
                return Ok(OpenedTrack {
                    track,
                    decoder,
                    duration,
                });
            }
            Err((err, retryable)) => {
                if !retryable || attempt >= max_retries || cancelled.load(Ordering::Acquire) {
                    return Err(err);
                }
                let backoff_ms = 50u64.saturating_mul(1u64 << attempt.min(5));
                attempt += 1;
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }
        }
    }
}

async fn resolve_and_open_once(
    track: &Track,
    decoder_factory: &Arc<dyn DecoderFactoryPort>,
    resolvers: &[Arc<dyn StreamResolverPort>],
    resolve_timeout: Duration,
    cancelled: &Arc<AtomicBool>,
) -> Result<Box<dyn PcmStreamPort>, (DecodeError, bool)> {
    match &track.source {
        AudioSource::LocalFile { path } => {
            let path = path.clone();
            let factory = Arc::clone(decoder_factory);
            tokio::task::spawn_blocking(move || factory.open_file(&path))
                .await
                .map_err(|e| (DecodeError::OpenFailed(e.to_string()), false))?
                .map_err(|e| (e, false))
        }
        source => {
            for resolver in resolvers {
                if resolver.handles_source(source) {
                    let timed = tokio::time::timeout(
                        resolver.resolve_timeout().unwrap_or(resolve_timeout),
                        resolver.resolve_stream(source),
                    )
                    .await;
                    if cancelled.load(Ordering::Acquire) {
                        return Err((DecodeError::OpenFailed("Cancelled".into()), false));
                    }
                    let stream = match timed {
                        Ok(Ok(s)) => s,
                        Ok(Err(ProviderError::Network(msg))) => {
                            return Err((
                                DecodeError::OpenFailed(format!(
                                    "Failed to resolve stream for {source:?}: Network error: {msg}"
                                )),
                                true,
                            ))
                        }
                        Ok(Err(ProviderError::RateLimited)) => {
                            return Err((
                                DecodeError::OpenFailed(format!(
                                    "Failed to resolve stream for {source:?}: Rate limit exceeded"
                                )),
                                true,
                            ))
                        }
                        Ok(Err(e)) => {
                            return Err((
                                DecodeError::OpenFailed(format!(
                                    "Failed to resolve stream for {source:?}: {e}"
                                )),
                                false,
                            ))
                        }
                        Err(_) => {
                            return Err((
                                DecodeError::OpenFailed(format!(
                                    "Timed out resolving stream for {source:?}"
                                )),
                                true,
                            ))
                        }
                    };
                    let factory = Arc::clone(decoder_factory);
                    return tokio::task::spawn_blocking(move || factory.open_stream(stream))
                        .await
                        .map_err(|e| (DecodeError::OpenFailed(e.to_string()), false))?
                        .map_err(|e| (e, false));
                }
            }
            Err((
                DecodeError::UnsupportedFormat(format!(
                    "No stream resolver registered for audio source: {source:?}"
                )),
                false,
            ))
        }
    }
}

/// The error for a source no resolver handles; Spotify entries from older versions say why.
fn no_resolver(source: &amp_domain::AudioSource) -> DecodeError {
    if matches!(source, amp_domain::AudioSource::Spotify { .. }) {
        return DecodeError::UnsupportedFormat(
            "Spotify isn't supported: this playlist entry can't be played (ADR-007)".into(),
        );
    }
    DecodeError::UnsupportedFormat(format!(
        "No stream resolver registered for audio source: {source:?}"
    ))
}
