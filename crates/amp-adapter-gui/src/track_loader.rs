//! Multi-source audio stream loader and resolver dispatcher.

#![forbid(unsafe_code)]

use amp_adapter_decoder::AudioStreamDecoder;
use amp_domain::{AudioSource, Track};
use amp_ports::StreamResolverPort;
use std::sync::Arc;

static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

/// Returns the shared Tokio multi-thread runtime for background audio stream resolution.
pub fn get_async_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("multiplay-gui-async")
            .build()
            .expect("Failed to initialize multiplay async runtime")
    })
}

/// Safely blocks on an async future without deadlocking runtime workers.
pub fn block_on_future<F: std::future::Future>(future: F) -> F::Output {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        match handle.runtime_flavor() {
            tokio::runtime::RuntimeFlavor::MultiThread => {
                tokio::task::block_in_place(|| handle.block_on(future))
            }
            _ => get_async_runtime().block_on(future),
        }
    } else {
        get_async_runtime().block_on(future)
    }
}

/// Resolves an audio stream and instantiates an AudioStreamDecoder for any supported audio source.
pub fn load_track_decoder(
    track: &Track,
    resolvers: &[Arc<dyn StreamResolverPort>],
) -> Result<AudioStreamDecoder, String> {
    match &track.source {
        AudioSource::LocalFile { path } => AudioStreamDecoder::open_file(path),
        source => {
            for resolver in resolvers {
                if resolver.handles_source(source) {
                    let stream = block_on_future(resolver.resolve_stream(source))
                        .map_err(|e| format!("Failed to resolve stream for {source:?}: {e}"))?;
                    return AudioStreamDecoder::open_stream(stream);
                }
            }
            Err(format!("No stream resolver registered for audio source: {source:?}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::TrackMetadata;
    use amp_ports::{async_trait, AudioSourceStream, ProviderError};
    use std::io::Cursor;

    struct DummyResolver;

    #[async_trait]
    impl StreamResolverPort for DummyResolver {
        fn handles_source(&self, source: &AudioSource) -> bool {
            matches!(source, AudioSource::DirectHttp { .. })
        }

        async fn resolve_stream(
            &self,
            _source: &AudioSource,
        ) -> Result<Box<dyn AudioSourceStream>, ProviderError> {
            Ok(Box::new(Cursor::new(Vec::new())))
        }
    }

    #[test]
    fn test_track_loader_unsupported_source() {
        let meta = TrackMetadata::new("Unknown", "Artist");
        let track = Track::new(AudioSource::Spotify { track_uri: "spotify:track:123".into() }, meta);
        let resolvers: Vec<Arc<dyn StreamResolverPort>> = vec![Arc::new(DummyResolver)];
        let res = load_track_decoder(&track, &resolvers);
        match res {
            Err(err) => assert!(err.contains("No stream resolver registered")),
            Ok(_) => panic!("Expected unsupported source to fail"),
        }
    }
}
