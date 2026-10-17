//! Pure-Rust real-time audio playback driver.
//! Connects CPAL hardware audio sink, Symphonia decoder, 10-band EQ, and FFT visualizer.

#![forbid(unsafe_code)]

use crate::track_loader::load_track_decoder;
use amp_adapter_cpal::{AudioDeviceDescription, CpalAudioSink};
use amp_adapter_decoder::AudioStreamDecoder;
use amp_core_engine::PlaybackDspChain;
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
    pub volume: Arc<Mutex<f32>>,
    pub eq_gains: Arc<Mutex<[f32; 10]>>,
    pub eq_preamp: Arc<Mutex<f32>>,
    pub eq_enabled: Arc<AtomicBool>,
    pub current_track: Arc<Mutex<Option<Track>>>,
    pub elapsed: Arc<Mutex<Duration>>,
    pub total_duration: Arc<Mutex<Option<Duration>>>,
    pub seek_requested: Arc<Mutex<Option<Duration>>>,
    pub track_finished: Arc<AtomicBool>,
    pub current_device_name: Arc<Mutex<String>>,
    pub available_devices: Vec<AudioDeviceDescription>,
    pub resolvers: Arc<Mutex<Vec<Arc<dyn StreamResolverPort>>>>,
    audio_sink: Arc<Mutex<CpalAudioSink>>,
    producer: Arc<Mutex<Producer<f32>>>,
    track_change_signal: Arc<AtomicBool>,
}

impl AudioPlaybackDriver {
    pub fn new() -> Result<Self, String> {
        let available_devices = CpalAudioSink::list_output_devices();

        let (mut audio_sink, producer) = CpalAudioSink::new()?;
        let sample_rate = audio_sink.sample_rate();
        let device_name = audio_sink.device_name().to_string();
        audio_sink.start()?;

        let yt = Arc::new(amp_adapter_youtube::PureYouTubeAdapter::default()) as Arc<dyn StreamResolverPort>;
        let http = Arc::new(amp_adapter_http::HttpStreamResolver::default()) as Arc<dyn StreamResolverPort>;
        let driver = Self {
            is_playing: Arc::new(AtomicBool::new(false)),
            visualizer_spectrum: Arc::new(Mutex::new([0.0f32; 16])),
            volume: Arc::new(Mutex::new(0.85f32)),
            eq_gains: Arc::new(Mutex::new([0.0f32; 10])),
            eq_preamp: Arc::new(Mutex::new(0.0f32)),
            eq_enabled: Arc::new(AtomicBool::new(true)),
            current_track: Arc::new(Mutex::new(None)),
            elapsed: Arc::new(Mutex::new(Duration::ZERO)),
            total_duration: Arc::new(Mutex::new(None)),
            seek_requested: Arc::new(Mutex::new(None)),
            track_finished: Arc::new(AtomicBool::new(false)),
            current_device_name: Arc::new(Mutex::new(device_name)),
            available_devices,
            resolvers: Arc::new(Mutex::new(vec![yt, http])),
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

        {
            let mut prod = self.producer.lock().unwrap();
            *prod = new_producer;
        }
        {
            let mut sink = self.audio_sink.lock().unwrap();
            let _ = sink.stop();
            *sink = new_sink;
        }
        {
            let mut cur = self.current_device_name.lock().unwrap();
            *cur = device_name.to_string();
        }
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
        let mut spec = self.visualizer_spectrum.lock().unwrap();
        *spec = [0.0; 16];
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

    #[allow(dead_code)]
    pub fn playback_state(&self) -> PlaybackState {
        if self.is_playing.load(Ordering::SeqCst) {
            PlaybackState::Playing
        } else {
            PlaybackState::Paused
        }
    }
}

fn run_playback_loop(driver: AudioPlaybackDriver, sample_rate: u32) {
    let AudioPlaybackDriver {
        producer,
        is_playing,
        visualizer_spectrum,
        volume,
        eq_gains,
        eq_preamp,
        eq_enabled,
        current_track,
        elapsed,
        total_duration,
        seek_requested,
        track_finished,
        track_change_signal,
        resolvers,
        ..
    } = driver;

    let chunk_frames = 512;
    let mut dsp_chain = PlaybackDspChain::new(sample_rate, chunk_frames);
    let mut pcm_buffer = vec![0.0f32; chunk_frames * 2];
    let mut active_decoder: Option<AudioStreamDecoder> = None;

    while !std::thread::panicking() {
        // 1. Handle track change request
        if track_change_signal.swap(false, Ordering::SeqCst) {
            let track_opt = current_track.lock().unwrap().clone();
            active_decoder = None;

            if let Some(track) = track_opt {
                let current_resolvers = resolvers.lock().unwrap().clone();
                match load_track_decoder(&track, &current_resolvers) {
                    Ok(dec) => {
                        let probed_dur = dec.duration().or(track.metadata.duration);
                        *total_duration.lock().unwrap() = probed_dur;
                        if let Some(cur) = current_track.lock().unwrap().as_mut() {
                            if cur.metadata.duration.is_none() {
                                cur.metadata.duration = probed_dur;
                            }
                        }
                        dsp_chain.set_source_sample_rate(dec.sample_rate());
                        active_decoder = Some(dec);
                    }
                    Err(e) => {
                        eprintln!("Failed to load audio track: {e}");
                        track_finished.store(true, Ordering::SeqCst);
                        is_playing.store(false, Ordering::SeqCst);
                    }
                }
            }
        }

        // 2. Handle pause / stopped state
        if !is_playing.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(15));
            continue;
        }

        // 3. Handle seek request
        let seek_opt = seek_requested.lock().unwrap().take();
        if let Some(target) = seek_opt {
            if let Some(ref mut dec) = active_decoder {
                let _ = dec.seek(target);
                *elapsed.lock().unwrap() = dec.elapsed();
            }
        }

        let eq_on = eq_enabled.load(Ordering::Relaxed);
        let preamp = *eq_preamp.lock().unwrap();
        let gains = *eq_gains.lock().unwrap();
        let vol = *volume.lock().unwrap();

        // 4. Decode audio frames from real decoder
        let mut read_samples = 0;
        if let Some(ref mut dec) = active_decoder {
            match dec.read_interleaved_samples(&mut pcm_buffer) {
                Ok(n) if n > 0 => {
                    read_samples = n;
                    *elapsed.lock().unwrap() = dec.elapsed();
                }
                Err(ref e) if e.contains("WouldBlock") => {
                    // Buffer underrun during network precaching: wait for incoming chunks
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Ok(_) => {
                    // EOF reached
                    track_finished.store(true, Ordering::SeqCst);
                    is_playing.store(false, Ordering::SeqCst);
                    active_decoder = None;
                }
                Err(e) => {
                    eprintln!("Decoder frame error: {e}");
                    track_finished.store(true, Ordering::SeqCst);
                    is_playing.store(false, Ordering::SeqCst);
                    active_decoder = None;
                }
            }
        } else {
            // No active file loaded: sleep briefly to avoid busy loop
            std::thread::sleep(Duration::from_millis(15));
            continue;
        }

        if read_samples == 0 {
            continue;
        }

        // 5. Unified DSP processing: Sinc resampling, master volume, 10-band EQ, soft-knee limiter, FFT visualizer
        let (processed, bands) = dsp_chain.process_interleaved(
            &pcm_buffer[..read_samples],
            vol,
            preamp,
            &gains,
            eq_on,
        );

        if let Ok(mut spec) = visualizer_spectrum.try_lock() {
            *spec = bands;
        }

        // 6. Push samples to CPAL lock-free ring buffer with adaptive backpressure
        if let Ok(mut prod) = producer.try_lock() {
            let mut pushed = 0;
            while pushed < processed.len() {
                if !is_playing.load(Ordering::Relaxed) {
                    break;
                }
                match prod.push(processed[pushed]) {
                    Ok(()) => {
                        pushed += 1;
                    }
                    Err(_) => {
                        // Buffer full: sleep 2ms rather than burning CPU in a busy yield loop
                        std::thread::sleep(Duration::from_millis(2));
                    }
                }
            }
        }
    }
}
