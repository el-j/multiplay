//! Composition factory wiring `PlayerEngine` with concrete audio sink, decoder factory,
//! and stream resolvers for the GUI adapter.

#![forbid(unsafe_code)]

use amp_adapter_bandcamp::PureBandcampAdapter;
use amp_adapter_cpal::{CapturedOutput, CpalAudioSink};
use amp_adapter_decoder::SymphoniaDecoderFactory;
use amp_adapter_http::HttpStreamResolver;
use amp_adapter_youtube::PureYouTubeAdapter;
use amp_core_engine::player::{
    default_runtime_handle, OutputDeviceInfo, PlayerEngine, PlayerEngineConfig,
    DEFAULT_RESOLVE_TIMEOUT,
};
use amp_core_engine::EventBus;
use amp_ports::{AudioSinkError, AudioSinkPort, StreamResolverPort};
use std::sync::Arc;

fn default_resolvers() -> Vec<Arc<dyn StreamResolverPort>> {
    let mut resolvers: Vec<Arc<dyn StreamResolverPort>> = Vec::new();
    if let Ok(yt) = PureYouTubeAdapter::new() {
        resolvers.push(Arc::new(yt));
    }
    if let Ok(bc) = PureBandcampAdapter::new() {
        resolvers.push(Arc::new(bc));
    }
    if let Ok(http) = HttpStreamResolver::new() {
        resolvers.push(Arc::new(http));
    }
    resolvers.push(crate::soulseek_setup::soulseek());
    resolvers
}

/// Bus the GUI listens on for playback events such as "this track could not be played".
fn gui_event_bus() -> Option<Arc<EventBus>> {
    let (bus, _commands) = EventBus::new(64);
    Some(Arc::new(bus))
}

/// Creates a [`PlayerEngine`] connected to the default OS audio output device.
pub fn create_player_engine() -> Result<PlayerEngine, String> {
    let (sink, producer) =
        CpalAudioSink::new().map_err(|e| format!("Failed to initialize audio output: {e}"))?;
    let sample_rate = sink.sample_rate();
    let device_name = sink.device_name().to_string();
    let clock = sink.clock();

    let available_devices = CpalAudioSink::list_output_devices()
        .into_iter()
        .map(|d| OutputDeviceInfo {
            name: d.name,
            is_default: d.is_default,
        })
        .collect();

    let switcher = Arc::new(move |target_device: &str| {
        let (new_sink, new_prod) = CpalAudioSink::new_with_device(Some(target_device))
            .map_err(AudioSinkError::DeviceNotFound)?;
        let new_clock = new_sink.clock();
        let boxed: Box<dyn AudioSinkPort> = Box::new(new_sink);
        Ok((boxed, new_prod, new_clock))
    });

    PlayerEngine::new_with_config(PlayerEngineConfig {
        sample_rate,
        device_name,
        available_devices,
        audio_sink: Box::new(sink),
        producer,
        clock,
        decoder_factory: Arc::new(SymphoniaDecoderFactory),
        resolvers: default_resolvers(),
        device_switcher: Some(switcher),
        runtime_handle: default_runtime_handle(),
        event_bus: gui_event_bus(),
        cmd_rx: None,
        resolve_timeout: DEFAULT_RESOLVE_TIMEOUT,
    })
    .map_err(|e| e.to_string())
}

/// Creates a hardware-free [`PlayerEngine`] backed by a null audio sink for tests and CI.
pub fn create_headless_engine() -> Result<PlayerEngine, String> {
    let (sink, producer) = CpalAudioSink::new_null(48_000, 2);
    let sample_rate = sink.sample_rate();
    let device_name = sink.device_name().to_string();
    let clock = sink.clock();

    PlayerEngine::new_with_config(PlayerEngineConfig {
        sample_rate,
        device_name: device_name.clone(),
        available_devices: vec![OutputDeviceInfo {
            name: device_name,
            is_default: true,
        }],
        audio_sink: Box::new(sink),
        producer,
        clock,
        decoder_factory: Arc::new(SymphoniaDecoderFactory),
        resolvers: default_resolvers(),
        device_switcher: None,
        runtime_handle: default_runtime_handle(),
        event_bus: gui_event_bus(),
        cmd_rx: None,
        resolve_timeout: DEFAULT_RESOLVE_TIMEOUT,
    })
    .map_err(|e| e.to_string())
}

/// Creates a hardware-free [`PlayerEngine`] whose rendered samples are captured for assertions.
pub fn create_capture_engine() -> Result<(PlayerEngine, CapturedOutput), String> {
    let (sink, producer, captured) = CpalAudioSink::new_capture(48_000, 2);
    let sample_rate = sink.sample_rate();
    let device_name = sink.device_name().to_string();
    let clock = sink.clock();

    let engine = PlayerEngine::new_with_config(PlayerEngineConfig {
        sample_rate,
        device_name: device_name.clone(),
        available_devices: vec![OutputDeviceInfo {
            name: device_name,
            is_default: true,
        }],
        audio_sink: Box::new(sink),
        producer,
        clock,
        decoder_factory: Arc::new(SymphoniaDecoderFactory),
        resolvers: default_resolvers(),
        device_switcher: None,
        runtime_handle: default_runtime_handle(),
        event_bus: gui_event_bus(),
        cmd_rx: None,
        resolve_timeout: DEFAULT_RESOLVE_TIMEOUT,
    })
    .map_err(|e| e.to_string())?;

    Ok((engine, captured))
}
