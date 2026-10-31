//! Unified Core Playback Engine (`PlayerEngine`, `PLAN-037`).
//!
//! Single playback orchestrator inside `amp-core-engine` implementing `PlayerControlPort`
//! and `PlaylistManagementPort`, consuming `PlayerCommand`, and emitting `PlayerEvent`.
//! Zero concrete adapter imports: communicates exclusively via `amp-ports`.

#![forbid(unsafe_code)]

pub mod dj_host;
pub mod dj_link;
pub mod loader;
pub mod ports;
pub mod pusher;
pub mod queue;
pub mod segment;
pub mod worker;

pub use dj_link::{DjLink, DjRequest};
pub use loader::{default_runtime_handle, open_track_blocking, DEFAULT_RESOLVE_TIMEOUT};
pub use queue::{EngineQueue, PreviousAction, PREVIOUS_RESTART_THRESHOLD};

use crate::events::{PlayerCommand, PlayerEvent};
use crate::playback::{RepeatMode, ShuffleMode};
use crate::EventBus;
use amp_domain::{PlaybackState, Track};
use amp_ports::{
    AudioSinkError, AudioSinkPort, DecodeError, DecoderFactoryPort, OutputClock, PcmStreamPort,
    StreamResolverPort,
};
use rtrb::Producer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;

/// Metadata for an enumerated output audio device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

/// Factory callback injected by composition roots to open a new hardware output device by name.
pub type DeviceSwitcherFn = Arc<
    dyn Fn(
            &str,
        )
            -> Result<(Box<dyn AudioSinkPort>, Producer<f32>, Arc<OutputClock>), AudioSinkError>
        + Send
        + Sync,
>;

struct WorkerLifecycle {
    shutdown: Arc<AtomicBool>,
    wake: Arc<(Mutex<u64>, Condvar)>,
    join_handle: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for WorkerLifecycle {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let (lock, cvar) = &*self.wake;
        if let Ok(mut g) = lock.lock() {
            *g = g.wrapping_add(1);
            cvar.notify_all();
        }
        if let Ok(mut slot) = self.join_handle.lock() {
            if let Some(handle) = slot.take() {
                if handle.thread().id() != std::thread::current().id() {
                    let _ = handle.join();
                }
            }
        }
    }
}

/// Configuration bundle used to construct a [`PlayerEngine`].
pub struct PlayerEngineConfig {
    pub sample_rate: u32,
    pub device_name: String,
    pub available_devices: Vec<OutputDeviceInfo>,
    pub audio_sink: Box<dyn AudioSinkPort>,
    pub producer: Producer<f32>,
    pub clock: Arc<OutputClock>,
    pub decoder_factory: Arc<dyn DecoderFactoryPort>,
    pub resolvers: Vec<Arc<dyn StreamResolverPort>>,
    pub device_switcher: Option<DeviceSwitcherFn>,
    pub runtime_handle: tokio::runtime::Handle,
    pub event_bus: Option<Arc<EventBus>>,
    pub cmd_rx: Option<UnboundedReceiver<PlayerCommand>>,
    pub resolve_timeout: Duration,
}

/// Unified core playback engine (`ADR-001`).
#[derive(Clone)]
pub struct PlayerEngine {
    pub is_playing: Arc<AtomicBool>,
    pub visualizer_spectrum: Arc<Mutex<[f32; 16]>>,
    pub visualizer_waveform: Arc<Mutex<[f32; 64]>>,
    pub volume: Arc<Mutex<f32>>,
    pub eq_gains: Arc<Mutex<[f32; 10]>>,
    pub eq_preamp: Arc<Mutex<f32>>,
    pub eq_enabled: Arc<AtomicBool>,
    pub current_track: Arc<Mutex<Option<Track>>>,
    pub next_track: Arc<Mutex<Option<Track>>>,
    pub elapsed: Arc<Mutex<Duration>>,
    pub total_duration: Arc<Mutex<Option<Duration>>>,
    pub seek_requested: Arc<Mutex<Option<Duration>>>,
    pub track_finished: Arc<AtomicBool>,
    pub track_advanced_signal: Arc<AtomicBool>,
    pub crossfade_seconds: Arc<Mutex<f32>>,
    pub current_device_name: Arc<Mutex<String>>,
    pub available_devices: Vec<OutputDeviceInfo>,
    pub resolvers: Arc<Mutex<Vec<Arc<dyn StreamResolverPort>>>>,
    pub dj: Arc<DjLink>,
    pub queue: Arc<Mutex<EngineQueue>>,
    pub(crate) clock: Arc<OutputClock>,
    pub(crate) decoder_factory: Arc<dyn DecoderFactoryPort>,
    pub(crate) runtime_handle: tokio::runtime::Handle,
    pub(crate) event_bus: Option<Arc<EventBus>>,
    pub(crate) producer: Arc<Mutex<Producer<f32>>>,
    pub(crate) track_change_signal: Arc<AtomicBool>,
    state: Arc<Mutex<PlaybackState>>,
    sample_rate: u32,
    resolve_timeout_dur: Duration,
    audio_sink: Arc<Mutex<Box<dyn AudioSinkPort>>>,
    device_switcher: Option<DeviceSwitcherFn>,
    cmd_rx: Arc<Mutex<Option<UnboundedReceiver<PlayerCommand>>>>,
    shutdown: Arc<AtomicBool>,
    wake: Arc<(Mutex<u64>, Condvar)>,
    lifecycle: Option<Arc<WorkerLifecycle>>,
}

impl PlayerEngine {
    /// Creates and starts the unified `PlayerEngine` worker thread.
    pub fn new_with_config(mut cfg: PlayerEngineConfig) -> Result<Self, AudioSinkError> {
        cfg.audio_sink.start()?;
        cfg.clock.pause_immediately();
        let sample_rate = cfg.sample_rate;
        let shutdown = Arc::new(AtomicBool::new(false));
        let wake = Arc::new((Mutex::new(0u64), Condvar::new()));
        let lifecycle = Arc::new(WorkerLifecycle {
            shutdown: Arc::clone(&shutdown),
            wake: Arc::clone(&wake),
            join_handle: Mutex::new(None),
        });

        let engine = Self {
            is_playing: Arc::new(AtomicBool::new(false)),
            visualizer_spectrum: Arc::new(Mutex::new([0.0f32; 16])),
            visualizer_waveform: Arc::new(Mutex::new([0.0f32; 64])),
            volume: Arc::new(Mutex::new(0.85f32)),
            eq_gains: Arc::new(Mutex::new([0.0f32; 10])),
            eq_preamp: Arc::new(Mutex::new(0.0f32)),
            eq_enabled: Arc::new(AtomicBool::new(true)),
            current_track: Arc::new(Mutex::new(None)),
            next_track: Arc::new(Mutex::new(None)),
            elapsed: Arc::new(Mutex::new(Duration::ZERO)),
            total_duration: Arc::new(Mutex::new(None)),
            seek_requested: Arc::new(Mutex::new(None)),
            track_finished: Arc::new(AtomicBool::new(false)),
            track_advanced_signal: Arc::new(AtomicBool::new(false)),
            crossfade_seconds: Arc::new(Mutex::new(0.0f32)),
            current_device_name: Arc::new(Mutex::new(cfg.device_name)),
            available_devices: cfg.available_devices,
            resolvers: Arc::new(Mutex::new(cfg.resolvers)),
            dj: Arc::new(DjLink::default()),
            queue: Arc::new(Mutex::new(EngineQueue::new())),
            clock: cfg.clock,
            decoder_factory: cfg.decoder_factory,
            runtime_handle: cfg.runtime_handle,
            event_bus: cfg.event_bus,
            producer: Arc::new(Mutex::new(cfg.producer)),
            track_change_signal: Arc::new(AtomicBool::new(false)),
            state: Arc::new(Mutex::new(PlaybackState::Stopped)),
            sample_rate,
            resolve_timeout_dur: cfg.resolve_timeout,
            audio_sink: Arc::new(Mutex::new(cfg.audio_sink)),
            device_switcher: cfg.device_switcher,
            cmd_rx: Arc::new(Mutex::new(cfg.cmd_rx)),
            shutdown,
            wake,
            lifecycle: Some(Arc::clone(&lifecycle)),
        };

        let mut worker_clone = engine.clone();
        worker_clone.lifecycle = None;

        let handle = std::thread::Builder::new()
            .name("multiplay-player-engine".to_string())
            .spawn(move || worker::run_playback_loop(worker_clone, sample_rate))
            .map_err(|e| AudioSinkError::StreamError(format!("Failed to spawn worker: {e}")))?;

        *lifecycle.join_handle.lock().unwrap() = Some(handle);
        Ok(engine)
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Events (track started, errors, ...) when the engine was built with an event bus.
    pub fn subscribe_events(&self) -> Option<tokio::sync::broadcast::Receiver<PlayerEvent>> {
        self.event_bus.as_ref().map(|bus| bus.subscribe())
    }
    pub fn clock(&self) -> Arc<OutputClock> {
        Arc::clone(&self.clock)
    }
    pub fn decoder_factory(&self) -> Arc<dyn DecoderFactoryPort> {
        Arc::clone(&self.decoder_factory)
    }
    pub fn resolve_timeout(&self) -> Duration {
        self.resolve_timeout_dur
    }
    pub fn dj_active(&self) -> bool {
        self.dj.is_active()
    }
    pub fn playback_state(&self) -> PlaybackState {
        *self.state.lock().unwrap()
    }

    /// Synchronously resolves and opens a `PcmStreamPort` for `track` (used by DJ deck loaders).
    pub fn open_track_stream(&self, track: &Track) -> Result<Box<dyn PcmStreamPort>, DecodeError> {
        let resolvers = self.resolvers.lock().unwrap().clone();
        open_track_blocking(
            &self.runtime_handle,
            track,
            &self.decoder_factory,
            &resolvers,
            self.resolve_timeout_dur,
        )
    }

    /// Switches hardware output device during runtime without losing playback state.
    pub fn switch_device(&self, device_name: &str) -> Result<(), String> {
        let switcher = self
            .device_switcher
            .as_ref()
            .ok_or_else(|| "Device switching not configured on this sink".to_string())?;
        let (mut new_sink, new_producer, _new_clock) =
            switcher(device_name).map_err(|e| e.to_string())?;
        new_sink.start().map_err(|e| e.to_string())?;

        *self.producer.lock().unwrap() = new_producer;
        let mut sink = self.audio_sink.lock().unwrap();
        let _ = sink.stop();
        *sink = new_sink;
        *self.current_device_name.lock().unwrap() = device_name.to_string();
        self.notify_worker();
        Ok(())
    }

    pub fn play(&self, track: Track) {
        self.queue.lock().unwrap().select_track(&track);
        *self.current_track.lock().unwrap() = Some(track);
        *self.elapsed.lock().unwrap() = Duration::ZERO;
        self.track_finished.store(false, Ordering::SeqCst);
        self.clock.flush_and_reset(Duration::ZERO);
        self.clock.resume();
        self.is_playing.store(true, Ordering::SeqCst);
        self.set_state_and_emit(PlaybackState::Loading);
        self.track_change_signal.store(true, Ordering::SeqCst);
        self.notify_worker();
    }

    pub fn pause(&self) {
        if self.is_playing.swap(false, Ordering::SeqCst) {
            let fade_frames = ((self.sample_rate * 5) / 1000).max(32);
            self.clock.request_pause(fade_frames);
            self.set_state_and_emit(PlaybackState::Paused);
            self.notify_worker();
        }
    }

    pub fn resume(&self) {
        if !self.is_playing.swap(true, Ordering::SeqCst) {
            self.clock.resume();
            self.set_state_and_emit(PlaybackState::Playing);
            self.notify_worker();
        }
    }

    pub fn toggle_play_pause(&self) {
        if self.is_playing.load(Ordering::SeqCst) {
            self.pause();
        } else {
            self.resume();
        }
    }

    pub fn stop(&self) {
        self.is_playing.store(false, Ordering::SeqCst);
        self.clock.pause_immediately();
        self.clock.flush_and_reset(Duration::ZERO);
        *self.elapsed.lock().unwrap() = Duration::ZERO;
        *self.seek_requested.lock().unwrap() = Some(Duration::ZERO);
        *self.visualizer_spectrum.lock().unwrap() = [0.0; 16];
        *self.visualizer_waveform.lock().unwrap() = [0.0; 64];
        self.set_state_and_emit(PlaybackState::Stopped);
        self.notify_worker();
    }

    pub fn seek(&self, target: Duration) {
        self.clock.flush_and_reset(target);
        *self.elapsed.lock().unwrap() = target;
        *self.seek_requested.lock().unwrap() = Some(target);
        self.notify_worker();
    }

    pub fn seek_relative(&self, offset: Duration, forward: bool) {
        let cur = *self.elapsed.lock().unwrap();
        self.seek(if forward {
            cur.saturating_add(offset)
        } else {
            cur.saturating_sub(offset)
        });
    }

    pub fn set_volume(&self, vol: f32) {
        *self.volume.lock().unwrap() = vol.clamp(0.0, 1.0);
    }

    pub fn set_eq_band(&self, band: usize, gain_db: f32) {
        if band < 10 {
            self.eq_gains.lock().unwrap()[band] = gain_db.clamp(-12.0, 12.0);
        }
    }

    pub fn set_eq_preamp(&self, gain_db: f32) {
        *self.eq_preamp.lock().unwrap() = gain_db.clamp(-12.0, 12.0);
    }

    pub fn toggle_eq(&self) {
        let cur = self.eq_enabled.load(Ordering::SeqCst);
        self.eq_enabled.store(!cur, Ordering::SeqCst);
    }

    pub fn register_resolver(&self, resolver: Arc<dyn StreamResolverPort>) {
        self.resolvers.lock().unwrap().insert(0, resolver);
    }

    pub fn set_next_track(&self, track: Option<Track>) {
        *self.next_track.lock().unwrap() = track;
    }

    pub fn set_crossfade_seconds(&self, secs: f32) {
        *self.crossfade_seconds.lock().unwrap() = secs.clamp(0.0, 15.0);
    }

    pub fn set_repeat_mode(&self, mode: RepeatMode) {
        self.queue.lock().unwrap().set_repeat(mode);
    }
    pub fn repeat_mode(&self) -> RepeatMode {
        self.queue.lock().unwrap().repeat_mode()
    }
    pub fn set_shuffle_mode(&self, mode: ShuffleMode) {
        self.queue.lock().unwrap().set_shuffle(mode);
    }
    pub fn shuffle_mode(&self) -> ShuffleMode {
        self.queue.lock().unwrap().shuffle_mode()
    }
    pub fn register_known_tracks(&self, tracks: impl IntoIterator<Item = Track>) {
        self.queue.lock().unwrap().register_known_tracks(tracks);
    }

    pub fn next_in_queue(&self) -> Option<Track> {
        let next = self.queue.lock().unwrap().next_track(true)?;
        self.play(next.clone());
        Some(next)
    }

    pub fn previous_in_queue(&self) -> PreviousAction {
        let cur = self.current_track.lock().unwrap().clone();
        let elapsed = *self.elapsed.lock().unwrap();
        let action = self
            .queue
            .lock()
            .unwrap()
            .previous_track(cur.as_ref(), elapsed);
        match &action {
            PreviousAction::RestartCurrent(_) => self.seek(Duration::ZERO),
            PreviousAction::PlayTrack(t) => self.play(t.clone()),
            PreviousAction::None => {}
        }
        action
    }

    pub(crate) fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }
    pub(crate) fn notify_worker(&self) {
        let (lock, cvar) = &*self.wake;
        if let Ok(mut g) = lock.lock() {
            *g = g.wrapping_add(1);
            cvar.notify_all();
        }
    }
    pub(crate) fn wait_for_wake(&self, timeout: Duration) {
        let (lock, cvar) = &*self.wake;
        if let Ok(g) = lock.lock() {
            let _ = cvar.wait_timeout(g, timeout);
        }
    }
    pub(crate) fn set_state_and_emit(&self, new_state: PlaybackState) {
        let changed = {
            let mut guard = self.state.lock().unwrap();
            if *guard != new_state {
                *guard = new_state;
                true
            } else {
                false
            }
        };
        if changed {
            self.emit(PlayerEvent::PlaybackStateChanged(new_state));
        }
    }
    pub(crate) fn emit(&self, event: PlayerEvent) {
        if let Some(bus) = &self.event_bus {
            bus.emit_event(event);
        }
    }
    pub(crate) fn drain_commands(&self) {
        let mut cmds = Vec::new();
        if let Ok(mut guard) = self.cmd_rx.try_lock() {
            if let Some(rx) = guard.as_mut() {
                while let Ok(cmd) = rx.try_recv() {
                    cmds.push(cmd);
                }
            }
        }
        for cmd in cmds {
            self.dispatch_command(cmd);
        }
    }
    pub(crate) fn peek_or_next_preload_track(&self) -> Option<Track> {
        self.next_track
            .lock()
            .unwrap()
            .clone()
            .or_else(|| self.queue.lock().unwrap().peek_upcoming())
    }
    pub(crate) fn take_next_track_for_handoff(&self) -> Option<Track> {
        self.next_track
            .lock()
            .unwrap()
            .take()
            .or_else(|| self.queue.lock().unwrap().next_track(false))
    }
    pub(crate) fn take_next_track_on_eof(&self) -> Option<Track> {
        self.next_track
            .lock()
            .unwrap()
            .take()
            .or_else(|| self.queue.lock().unwrap().next_track(false))
    }
}
