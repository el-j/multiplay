//! Pure-Rust real-time audio playback driver.
//! Connects CPAL hardware audio sink, Symphonia decoder, 10-band EQ, and FFT visualizer.

#![forbid(unsafe_code)]

use crate::dj_audio::DjAudioSharedState;
use crate::playback_worker::run_playback_loop;
use amp_adapter_cpal::{AudioDeviceDescription, CpalAudioSink};
use amp_domain::{PlaybackState, Track};
use amp_ports::{AudioSinkPort, StreamResolverPort};
use rtrb::Producer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub struct AudioPlaybackDriver {
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
    pub available_devices: Vec<AudioDeviceDescription>,
    pub resolvers: Arc<Mutex<Vec<Arc<dyn StreamResolverPort>>>>,
    pub dj_audio: Arc<Mutex<DjAudioSharedState>>,
    audio_sink: Arc<Mutex<CpalAudioSink>>,
    pub(crate) producer: Arc<Mutex<Producer<f32>>>,
    pub(crate) track_change_signal: Arc<AtomicBool>,
}

impl AudioPlaybackDriver {
    pub fn new() -> Result<Self, String> {
        let available_devices = CpalAudioSink::list_output_devices();
        let (mut audio_sink, producer) = CpalAudioSink::new()?;
        let sample_rate = audio_sink.sample_rate();
        let device_name = audio_sink.device_name().to_string();
        audio_sink.start()?;

        let yt = Arc::new(amp_adapter_youtube::PureYouTubeAdapter::default())
            as Arc<dyn StreamResolverPort>;
        let http = Arc::new(amp_adapter_http::HttpStreamResolver::default())
            as Arc<dyn StreamResolverPort>;
        let driver = Self {
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
            current_device_name: Arc::new(Mutex::new(device_name)),
            available_devices,
            resolvers: Arc::new(Mutex::new(vec![yt, http])),
            dj_audio: Arc::new(Mutex::new(DjAudioSharedState::default())),
            audio_sink: Arc::new(Mutex::new(audio_sink)),
            producer: Arc::new(Mutex::new(producer)),
            track_change_signal: Arc::new(AtomicBool::new(false)),
        };

        let worker_driver = driver.clone();
        std::thread::Builder::new()
            .name("multiplay-audio-worker".to_string())
            .spawn(move || run_playback_loop(worker_driver, sample_rate))
            .map_err(|e| format!("Failed to spawn audio worker: {e}"))?;

        Ok(driver)
    }

    /// Dynamically switches hardware output device during runtime.
    pub fn switch_device(&self, device_name: &str) -> Result<(), String> {
        let (mut new_sink, new_producer) = CpalAudioSink::new_with_device(Some(device_name))?;
        new_sink.start()?;

        *self.producer.lock().unwrap() = new_producer;
        let mut sink = self.audio_sink.lock().unwrap();
        let _ = sink.stop();
        *sink = new_sink;
        *self.current_device_name.lock().unwrap() = device_name.to_string();
        Ok(())
    }

    pub fn play(&self, track: Track) {
        *self.current_track.lock().unwrap() = Some(track);
        *self.elapsed.lock().unwrap() = Duration::ZERO;
        self.track_finished.store(false, Ordering::SeqCst);
        self.track_change_signal.store(true, Ordering::SeqCst);
        self.is_playing.store(true, Ordering::SeqCst);
    }

    pub fn seek(&self, target: Duration) {
        *self.seek_requested.lock().unwrap() = Some(target);
    }

    pub fn toggle_play_pause(&self) {
        let current = self.is_playing.load(Ordering::SeqCst);
        self.is_playing.store(!current, Ordering::SeqCst);
    }

    pub fn stop(&self) {
        self.is_playing.store(false, Ordering::SeqCst);
        *self.elapsed.lock().unwrap() = Duration::ZERO;
        *self.seek_requested.lock().unwrap() = Some(Duration::ZERO);
        *self.visualizer_spectrum.lock().unwrap() = [0.0; 16];
        *self.visualizer_waveform.lock().unwrap() = [0.0; 64];
    }

    pub fn set_volume(&self, vol: f32) {
        *self.volume.lock().unwrap() = vol.clamp(0.0, 1.0);
    }

    pub fn set_eq_band(&self, band: usize, gain_db: f32) {
        if band < 10 {
            let mut gains = self.eq_gains.lock().unwrap();
            gains[band] = gain_db.clamp(-12.0, 12.0);
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

    #[allow(dead_code)]
    pub fn playback_state(&self) -> PlaybackState {
        if self.is_playing.load(Ordering::SeqCst) {
            PlaybackState::Playing
        } else {
            PlaybackState::Paused
        }
    }
}


