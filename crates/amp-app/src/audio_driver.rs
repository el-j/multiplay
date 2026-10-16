//! Pure-Rust real-time audio playback driver for CLI and TUI.
//! Connects CPAL hardware audio sink, Symphonia decoder, 10-band EQ, and FFT visualizer.

#![forbid(unsafe_code)]

use amp_adapter_cpal::CpalAudioSink;
use amp_adapter_decoder::AudioStreamDecoder;
use amp_core_engine::{GraphicEqualizer, SpectrumVisualizer};
use amp_domain::{AudioSource, PlaybackState, Track};
use amp_ports::AudioSinkPort;
use rtrb::Producer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Real-time audio playback driver orchestrating decoder, DSP, and hardware sink.
pub struct AudioPlaybackDriver {
    pub is_playing: Arc<AtomicBool>,
    pub visualizer_spectrum: Arc<Mutex<[f32; 16]>>,
    pub volume: Arc<Mutex<f32>>,
    pub eq_gains: Arc<Mutex<[f32; 10]>>,
    pub current_track: Arc<Mutex<Option<Track>>>,
    pub elapsed: Arc<Mutex<Duration>>,
    pub total_duration: Arc<Mutex<Option<Duration>>>,
    pub seek_requested: Arc<Mutex<Option<Duration>>>,
    pub track_finished: Arc<AtomicBool>,
    #[allow(dead_code)]
    audio_sink: Arc<Mutex<CpalAudioSink>>,
    #[allow(dead_code)]
    producer: Arc<Mutex<Producer<f32>>>,
    track_change_signal: Arc<AtomicBool>,
}

impl AudioPlaybackDriver {
    /// Creates and initializes the playback driver with default audio output.
    pub fn new() -> Result<Self, String> {
        let (mut audio_sink, producer) = CpalAudioSink::new()?;
        let sample_rate = audio_sink.sample_rate();
        audio_sink.start()?;

        let is_playing = Arc::new(AtomicBool::new(false));
        let visualizer_spectrum = Arc::new(Mutex::new([0.0f32; 16]));
        let volume = Arc::new(Mutex::new(0.85f32));
        let eq_gains = Arc::new(Mutex::new([0.0f32; 10]));
        let current_track = Arc::new(Mutex::new(None));
        let elapsed = Arc::new(Mutex::new(Duration::ZERO));
        let total_duration = Arc::new(Mutex::new(None));
        let seek_requested = Arc::new(Mutex::new(None));
        let track_finished = Arc::new(AtomicBool::new(false));
        let track_change_signal = Arc::new(AtomicBool::new(false));

        let producer_arc = Arc::new(Mutex::new(producer));
        let audio_sink_arc = Arc::new(Mutex::new(audio_sink));

        let playing_flag = Arc::clone(&is_playing);
        let spec_out = Arc::clone(&visualizer_spectrum);
        let vol_ref = Arc::clone(&volume);
        let eq_ref = Arc::clone(&eq_gains);
        let track_ref = Arc::clone(&current_track);
        let elapsed_ref = Arc::clone(&elapsed);
        let total_dur_ref = Arc::clone(&total_duration);
        let seek_ref = Arc::clone(&seek_requested);
        let finished_ref = Arc::clone(&track_finished);
        let change_ref = Arc::clone(&track_change_signal);
        let prod_ref = Arc::clone(&producer_arc);

        std::thread::Builder::new()
            .name("multiplay-tui-audio".to_string())
            .spawn(move || {
                run_playback_loop(
                    prod_ref,
                    playing_flag,
                    spec_out,
                    vol_ref,
                    eq_ref,
                    track_ref,
                    elapsed_ref,
                    total_dur_ref,
                    seek_ref,
                    finished_ref,
                    change_ref,
                    sample_rate,
                );
            })
            .map_err(|e| format!("Failed to spawn audio worker: {e}"))?;

        Ok(Self {
            is_playing,
            visualizer_spectrum,
            volume,
            eq_gains,
            current_track,
            elapsed,
            total_duration,
            seek_requested,
            track_finished,
            audio_sink: audio_sink_arc,
            producer: producer_arc,
            track_change_signal,
        })
    }

    /// Begins playback of a specific track entity.
    pub fn play(&self, track: Track) {
        *self.current_track.lock().unwrap() = Some(track);
        *self.elapsed.lock().unwrap() = Duration::ZERO;
        self.track_finished.store(false, Ordering::SeqCst);
        self.track_change_signal.store(true, Ordering::SeqCst);
        self.is_playing.store(true, Ordering::SeqCst);
    }

    /// Toggles play/pause state.
    pub fn toggle_play_pause(&self) {
        let current = self.is_playing.load(Ordering::SeqCst);
        self.is_playing.store(!current, Ordering::SeqCst);
    }

    /// Stops playback and resets elapsed position.
    pub fn stop(&self) {
        self.is_playing.store(false, Ordering::SeqCst);
        *self.elapsed.lock().unwrap() = Duration::ZERO;
        *self.seek_requested.lock().unwrap() = Some(Duration::ZERO);
        let mut spec = self.visualizer_spectrum.lock().unwrap();
        *spec = [0.0; 16];
    }

    /// Sets playback volume (0.0 to 1.0).
    pub fn set_volume(&self, vol: f32) {
        *self.volume.lock().unwrap() = vol.clamp(0.0, 1.0);
    }

    /// Returns current playback transport state.
    pub fn playback_state(&self) -> PlaybackState {
        if self.is_playing.load(Ordering::SeqCst) {
            PlaybackState::Playing
        } else {
            PlaybackState::Paused
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_playback_loop(
    producer: Arc<Mutex<Producer<f32>>>,
    is_playing: Arc<AtomicBool>,
    visualizer_spectrum: Arc<Mutex<[f32; 16]>>,
    volume: Arc<Mutex<f32>>,
    eq_gains: Arc<Mutex<[f32; 10]>>,
    current_track: Arc<Mutex<Option<Track>>>,
    elapsed: Arc<Mutex<Duration>>,
    total_duration: Arc<Mutex<Option<Duration>>>,
    seek_requested: Arc<Mutex<Option<Duration>>>,
    track_finished: Arc<AtomicBool>,
    track_change_signal: Arc<AtomicBool>,
    sample_rate: u32,
) {
    let mut eq = GraphicEqualizer::new(sample_rate as f32);
    let mut vis = SpectrumVisualizer::new();
    let chunk_frames = 512;
    let mut pcm_buffer = vec![0.0f32; chunk_frames * 2];
    let mut active_decoder: Option<AudioStreamDecoder> = None;

    while !std::thread::panicking() {
        if track_change_signal.swap(false, Ordering::SeqCst) {
            let track_opt = current_track.lock().unwrap().clone();
            active_decoder = None;

            if let Some(track) = track_opt {
                if let AudioSource::LocalFile { path } = &track.source {
                    if let Ok(decoder) = AudioStreamDecoder::open_file(path) {
                        *total_duration.lock().unwrap() = decoder.duration();
                        active_decoder = Some(decoder);
                    }
                }
            }
        }

        if let Some(target_pos) = seek_requested.lock().unwrap().take() {
            if let Some(ref mut dec) = active_decoder {
                let _ = dec.seek(target_pos);
                *elapsed.lock().unwrap() = target_pos;
            }
        }

        if !is_playing.load(Ordering::SeqCst) {
            let mut spec = visualizer_spectrum.lock().unwrap();
            *spec = [0.0; 16];
            std::thread::sleep(Duration::from_millis(15));
            continue;
        }

        let mut read_samples = 0;
        if let Some(ref mut dec) = active_decoder {
            match dec.read_interleaved_samples(&mut pcm_buffer) {
                Ok(n) if n > 0 => {
                    read_samples = n;
                    *elapsed.lock().unwrap() = dec.elapsed();
                }
                _ => {
                    track_finished.store(true, Ordering::SeqCst);
                    is_playing.store(false, Ordering::SeqCst);
                    active_decoder = None;
                }
            }
        } else {
            std::thread::sleep(Duration::from_millis(15));
            continue;
        }

        if read_samples == 0 {
            continue;
        }

        let vol = *volume.lock().unwrap();
        for s in &mut pcm_buffer[..read_samples] {
            *s *= vol;
        }

        let gains = *eq_gains.lock().unwrap();
        for (i, &g) in gains.iter().enumerate() {
            eq.set_band_gain(i, g);
        }
        eq.process_interleaved(&mut pcm_buffer[..read_samples]);

        vis.push_samples(&pcm_buffer[..read_samples]);
        let bands = vis.compute_spectrum_bands();
        if let Ok(mut spec) = visualizer_spectrum.try_lock() {
            *spec = bands;
        }

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
