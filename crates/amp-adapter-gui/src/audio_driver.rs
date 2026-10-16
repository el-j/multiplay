//! Pure-Rust real-time audio playback driver.
//! Connects CPAL hardware audio sink, Symphonia decoder, 10-band EQ, and FFT visualizer.

#![forbid(unsafe_code)]

use amp_adapter_cpal::{AudioDeviceDescription, CpalAudioSink};
use amp_adapter_decoder::AudioStreamDecoder;
use amp_core_engine::{GraphicEqualizer, SpectrumVisualizer};
use amp_domain::{AudioSource, PlaybackState, Track};
use amp_ports::AudioSinkPort;
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

        let is_playing = Arc::new(AtomicBool::new(false));
        let visualizer_spectrum = Arc::new(Mutex::new([0.0f32; 16]));
        let volume = Arc::new(Mutex::new(0.85f32));
        let eq_gains = Arc::new(Mutex::new([0.0f32; 10]));
        let eq_preamp = Arc::new(Mutex::new(0.0f32));
        let eq_enabled = Arc::new(AtomicBool::new(true));
        let current_track = Arc::new(Mutex::new(None));
        let elapsed = Arc::new(Mutex::new(Duration::ZERO));
        let total_duration = Arc::new(Mutex::new(None));
        let seek_requested = Arc::new(Mutex::new(None));
        let track_finished = Arc::new(AtomicBool::new(false));
        let track_change_signal = Arc::new(AtomicBool::new(false));
        let current_device_name = Arc::new(Mutex::new(device_name));

        let producer_arc = Arc::new(Mutex::new(producer));
        let audio_sink_arc = Arc::new(Mutex::new(audio_sink));

        let driver = Self {
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
            current_device_name,
            available_devices,
            audio_sink: audio_sink_arc,
            producer: producer_arc,
            track_change_signal,
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
        ..
    } = driver;

    let mut eq = GraphicEqualizer::new(sample_rate as f32);
    let mut vis = SpectrumVisualizer::new();
    let chunk_frames = 512;
    let mut pcm_buffer = vec![0.0f32; chunk_frames * 2];
    let mut active_decoder: Option<AudioStreamDecoder> = None;

    while !std::thread::panicking() {
        // 1. Handle track change request
        if track_change_signal.swap(false, Ordering::SeqCst) {
            let track_opt = current_track.lock().unwrap().clone();
            active_decoder = None;

            if let Some(track) = track_opt {
                if let AudioSource::LocalFile { ref path } = track.source {
                    if let Ok(dec) = AudioStreamDecoder::open_file(path) {
                        *total_duration.lock().unwrap() = dec.duration().or(track.metadata.duration);
                        active_decoder = Some(dec);
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

        // 4. Update EQ settings (only updates when changed)
        let eq_on = eq_enabled.load(Ordering::Relaxed);
        eq.set_enabled(eq_on);
        let preamp = *eq_preamp.lock().unwrap();
        eq.set_preamp_gain(preamp);
        {
            let gains = *eq_gains.lock().unwrap();
            for (idx, &g) in gains.iter().enumerate() {
                eq.set_band_gain(idx, g);
            }
        }

        let vol = *volume.lock().unwrap();

        // 5. Decode audio frames from real decoder
        let mut read_samples = 0;
        if let Some(ref mut dec) = active_decoder {
            match dec.read_interleaved_samples(&mut pcm_buffer) {
                Ok(n) if n > 0 => {
                    read_samples = n;
                    *elapsed.lock().unwrap() = dec.elapsed();
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

        // 6. Apply master volume
        for s in &mut pcm_buffer[..read_samples] {
            *s *= vol;
        }

        // 7. Apply 10-band equalizer
        eq.process_interleaved(&mut pcm_buffer[..read_samples]);

        // 8. Update FFT spectrum visualizer
        vis.push_samples(&pcm_buffer[..read_samples]);
        let bands = vis.compute_spectrum_bands();
        if let Ok(mut spec) = visualizer_spectrum.try_lock() {
            *spec = bands;
        }

        // 9. Push samples to CPAL lock-free ring buffer
        if let Ok(mut prod) = producer.try_lock() {
            for &s in &pcm_buffer[..read_samples] {
                while prod.push(s).is_err() {
                    if !is_playing.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::yield_now();
                }
            }
        }
    }
}
