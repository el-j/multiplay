//! Pure-Rust real-time audio playback driver.
//! Connects CPAL hardware audio sink, lock-free ring buffer, 10-band EQ, and FFT visualizer.

#![forbid(unsafe_code)]

use amp_adapter_cpal::{AudioDeviceDescription, CpalAudioSink};
use amp_core_engine::{GraphicEqualizer, SpectrumVisualizer};
use amp_domain::{PlaybackState, Track};
use amp_ports::AudioSinkPort;
use rtrb::Producer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct AudioPlaybackDriver {
    pub is_playing: Arc<AtomicBool>,
    pub visualizer_spectrum: Arc<Mutex<[f32; 16]>>,
    pub volume: Arc<Mutex<f32>>,
    pub eq_gains: Arc<Mutex<[f32; 10]>>,
    pub eq_enabled: Arc<AtomicBool>,
    pub current_track: Arc<Mutex<Option<Track>>>,
    pub elapsed: Arc<Mutex<Duration>>,
    pub current_device_name: Arc<Mutex<String>>,
    pub available_devices: Vec<AudioDeviceDescription>,
    audio_sink: Arc<Mutex<CpalAudioSink>>,
    producer: Arc<Mutex<Producer<f32>>>,
}

impl AudioPlaybackDriver {
    pub fn new() -> Result<Self, String> {
        let available_devices = CpalAudioSink::list_output_devices();

        // Default to system default device, or MacBook Pro Speakers if available
        let (mut audio_sink, producer) = CpalAudioSink::new()?;
        let sample_rate = audio_sink.sample_rate();
        let device_name = audio_sink.device_name().to_string();
        audio_sink.start()?;

        let is_playing = Arc::new(AtomicBool::new(false));
        let visualizer_spectrum = Arc::new(Mutex::new([0.0f32; 16]));
        let volume = Arc::new(Mutex::new(0.85f32));
        let eq_gains = Arc::new(Mutex::new([0.0f32; 10]));
        let eq_enabled = Arc::new(AtomicBool::new(true));
        let current_track = Arc::new(Mutex::new(None));
        let elapsed = Arc::new(Mutex::new(Duration::ZERO));
        let current_device_name = Arc::new(Mutex::new(device_name));

        let producer_arc = Arc::new(Mutex::new(producer));
        let audio_sink_arc = Arc::new(Mutex::new(audio_sink));

        let playing_flag = Arc::clone(&is_playing);
        let spec_out = Arc::clone(&visualizer_spectrum);
        let vol_ref = Arc::clone(&volume);
        let eq_ref = Arc::clone(&eq_gains);
        let eq_on_ref = Arc::clone(&eq_enabled);
        let elapsed_ref = Arc::clone(&elapsed);
        let prod_ref = Arc::clone(&producer_arc);

        // Spawn background audio synthesis and streaming thread
        std::thread::Builder::new()
            .name("multiplay-audio-worker".to_string())
            .spawn(move || {
                run_playback_loop(
                    prod_ref,
                    playing_flag,
                    spec_out,
                    vol_ref,
                    eq_ref,
                    eq_on_ref,
                    elapsed_ref,
                    sample_rate,
                );
            })
            .map_err(|e| format!("Failed to spawn audio worker: {e}"))?;

        Ok(Self {
            is_playing,
            visualizer_spectrum,
            volume,
            eq_gains,
            eq_enabled,
            current_track,
            elapsed,
            current_device_name,
            available_devices,
            audio_sink: audio_sink_arc,
            producer: producer_arc,
        })
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
        self.is_playing.store(true, Ordering::SeqCst);
    }

    pub fn toggle_play_pause(&self) {
        let current = self.is_playing.load(Ordering::SeqCst);
        self.is_playing.store(!current, Ordering::SeqCst);
    }

    pub fn stop(&self) {
        self.is_playing.store(false, Ordering::SeqCst);
        *self.elapsed.lock().unwrap() = Duration::ZERO;
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

#[allow(clippy::too_many_arguments)]
fn run_playback_loop(
    producer: Arc<Mutex<Producer<f32>>>,
    is_playing: Arc<AtomicBool>,
    visualizer_spectrum: Arc<Mutex<[f32; 16]>>,
    volume: Arc<Mutex<f32>>,
    eq_gains: Arc<Mutex<[f32; 10]>>,
    eq_enabled: Arc<AtomicBool>,
    elapsed: Arc<Mutex<Duration>>,
    sample_rate: u32,
) {
    let mut eq = GraphicEqualizer::new(sample_rate as f32);
    let mut vis = SpectrumVisualizer::new();
    let chunk_frames = 512;
    let mut pcm_buffer = vec![0.0f32; chunk_frames * 2];
    let mut phase: f32 = 0.0;
    let mut phase2: f32 = 0.0;
    let mut sub_phase: f32 = 0.0;

    // Musical scale frequencies for synthetic melodic playback (Aphex / Kraftwerk style)
    let notes = [220.0, 261.63, 329.63, 392.0, 440.0, 523.25, 659.25, 783.99];

    while !std::thread::panicking() {
        if !is_playing.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(15));
            continue;
        }

        let vol = *volume.lock().unwrap();
        let eq_on = eq_enabled.load(Ordering::Relaxed);
        eq.set_enabled(eq_on);
        {
            let gains = *eq_gains.lock().unwrap();
            for (idx, &g) in gains.iter().enumerate() {
                eq.set_band_gain(idx, g);
            }
        }

        let el_secs = elapsed.lock().unwrap().as_secs_f32();
        let note_idx = ((el_secs * 3.0) as usize) % notes.len();
        let target_freq = notes[note_idx];

        // Synthesize rich ambient stereo chords
        #[allow(clippy::chunks_exact_to_as_chunks)]
        for frame in pcm_buffer.chunks_exact_mut(2) {
            let osc1 = (phase * 2.0 * std::f32::consts::PI).sin() * 0.25;
            let osc2 = (phase2 * 2.0 * std::f32::consts::PI).sin() * 0.15;
            let sub = (sub_phase * 2.0 * std::f32::consts::PI).sin() * 0.35;

            let sample_l = (osc1 + osc2 + sub) * vol;
            let sample_r = (osc1 * 0.8 + osc2 * 1.2 + sub) * vol;

            frame[0] = sample_l;
            frame[1] = sample_r;

            phase += target_freq / sample_rate as f32;
            if phase > 1.0 { phase -= 1.0; }

            phase2 += (target_freq * 1.5) / sample_rate as f32;
            if phase2 > 1.0 { phase2 -= 1.0; }

            sub_phase += (target_freq * 0.5) / sample_rate as f32;
            if sub_phase > 1.0 { sub_phase -= 1.0; }
        }

        // Apply 10-band equalizer
        eq.process_interleaved(&mut pcm_buffer);

        // Update FFT spectrum visualizer
        vis.push_samples(&pcm_buffer);
        let bands = vis.compute_spectrum_bands();
        if let Ok(mut spec) = visualizer_spectrum.try_lock() {
            *spec = bands;
        }

        // Push samples to CPAL lock-free ring buffer
        if let Ok(mut prod) = producer.try_lock() {
            for &s in &pcm_buffer {
                while prod.push(s).is_err() {
                    if !is_playing.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::yield_now();
                }
            }
        }

        // Advance elapsed duration
        let chunk_dur = Duration::from_secs_f32(chunk_frames as f32 / sample_rate as f32);
        if let Ok(mut el) = elapsed.try_lock() {
            *el += chunk_dur;
        }
    }
}
