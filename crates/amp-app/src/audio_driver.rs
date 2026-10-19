//! Pure-Rust real-time audio playback driver for CLI and TUI.
//! Connects CPAL hardware audio sink, Symphonia decoder, 10-band EQ, and FFT visualizer.

#![forbid(unsafe_code)]

use crate::track_loader::load_track_decoder;
use amp_adapter_cpal::{CapturedOutput, CpalAudioSink};
use amp_adapter_decoder::AudioStreamDecoder;
use amp_core_engine::PlaybackDspChain;
use amp_domain::{PlaybackState, Track};
use amp_ports::{AudioSinkPort, StreamResolverPort};
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
    pub resolvers: Arc<Mutex<Vec<Arc<dyn StreamResolverPort>>>>,
    #[allow(dead_code)]
    audio_sink: Arc<Mutex<CpalAudioSink>>,
    #[allow(dead_code)]
    producer: Arc<Mutex<Producer<f32>>>,
    track_change_signal: Arc<AtomicBool>,
}

impl AudioPlaybackDriver {
    /// Creates and initializes the playback driver with default audio output.
    pub fn new() -> Result<Self, String> {
        let (audio_sink, producer) = CpalAudioSink::new()?;
        Self::with_sink(audio_sink, producer)
    }

    /// Plays into a null sink: no audio hardware is opened (tests, CI, headless).
    pub fn new_headless() -> Result<Self, String> {
        let (audio_sink, producer) = CpalAudioSink::new_null(48_000, 2);
        Self::with_sink(audio_sink, producer)
    }

    /// Plays into a capture sink that records the rendered samples (tests).
    pub fn new_capture() -> Result<(Self, CapturedOutput), String> {
        let (audio_sink, producer, captured) = CpalAudioSink::new_capture(48_000, 2);
        Ok((Self::with_sink(audio_sink, producer)?, captured))
    }

    fn with_sink(mut audio_sink: CpalAudioSink, producer: Producer<f32>) -> Result<Self, String> {
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

        let yt = Arc::new(amp_adapter_youtube::PureYouTubeAdapter::default())
            as Arc<dyn StreamResolverPort>;
        let http = Arc::new(amp_adapter_http::HttpStreamResolver::default())
            as Arc<dyn StreamResolverPort>;
        let resolvers = Arc::new(Mutex::new(vec![yt, http]));

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
        let res_ref = Arc::clone(&resolvers);

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
                    res_ref,
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
            resolvers,
            audio_sink: audio_sink_arc,
            producer: producer_arc,
            track_change_signal,
        })
    }

    /// Registers a custom or remote stream resolver for network audio playback.
    pub fn register_resolver(&self, resolver: Arc<dyn StreamResolverPort>) {
        self.resolvers.lock().unwrap().insert(0, resolver);
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
    resolvers: Arc<Mutex<Vec<Arc<dyn StreamResolverPort>>>>,
    sample_rate: u32,
) {
    let chunk_frames = 512;
    let mut dsp_chain = PlaybackDspChain::new(sample_rate, chunk_frames);
    let mut pcm_buffer = vec![0.0f32; chunk_frames * 2];
    let mut active_decoder: Option<AudioStreamDecoder> = None;

    while !std::thread::panicking() {
        if track_change_signal.swap(false, Ordering::SeqCst) {
            let track_opt = current_track.lock().unwrap().clone();
            active_decoder = None;

            if let Some(track) = track_opt {
                let current_resolvers = resolvers.lock().unwrap().clone();
                match load_track_decoder(&track, &current_resolvers) {
                    Ok(decoder) => {
                        let probed_dur = decoder.duration().or(track.metadata.duration);
                        *total_duration.lock().unwrap() = probed_dur;
                        if let Some(cur) = current_track.lock().unwrap().as_mut() {
                            if cur.metadata.duration.is_none() {
                                cur.metadata.duration = probed_dur;
                            }
                        }
                        dsp_chain.set_source_sample_rate(decoder.sample_rate());
                        active_decoder = Some(decoder);
                    }
                    Err(e) => {
                        eprintln!("Failed to load audio track: {e}");
                        track_finished.store(true, Ordering::SeqCst);
                        is_playing.store(false, Ordering::SeqCst);
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
            // Release the lock before sleeping, or controls such as stop() starve.
            *visualizer_spectrum.lock().unwrap() = [0.0; 16];
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
                Err(ref e) if e.contains("WouldBlock") => {
                    // Buffer underrun during network precaching: wait for incoming chunks
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
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
        let gains = *eq_gains.lock().unwrap();

        let (processed, bands) =
            dsp_chain.process_interleaved(&pcm_buffer[..read_samples], vol, 0.0, &gains, true);

        if let Ok(mut spec) = visualizer_spectrum.try_lock() {
            *spec = bands;
        }

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
                        std::thread::sleep(Duration::from_millis(2));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn controls_stay_responsive_while_paused() {
        // The idle worker must not hold shared state locks while it sleeps, or callers
        // such as stop() starve (minutes on Linux CI).
        let driver = AudioPlaybackDriver::new_headless().expect("headless driver");
        std::thread::sleep(Duration::from_millis(50));
        let started = Instant::now();
        for _ in 0..100 {
            driver.stop();
        }
        let took = started.elapsed();
        assert!(
            took < Duration::from_millis(500),
            "100 stop() calls took {took:?}"
        );
    }
}
