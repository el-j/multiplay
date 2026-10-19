//! The audio worker's side of DJ mode: it runs the DJ engine in the worker thread (which owns
//! the sound card queue) and switches between normal playback and the engine without a gap.
//!
//! - **Entering:** normal playback keeps running until deck A's copy of the track is decoded
//!   past the playing position, then deck A continues from exactly that position. The audio
//!   already queued bridges the switch.
//! - **Leaving:** the normal decoder for the on-air deck's track opens on a helper thread while
//!   the decks keep playing; once it is ready it seeks to the deck's position and the worker
//!   crossfades from the decks to it over 20 ms (decoders cannot all seek sample-exactly).

#![forbid(unsafe_code)]

use crate::audio_driver::AudioPlaybackDriver;
use crate::audio_pusher::update_visualizer_and_push;
use crate::dj_link::DjRequest;
use crate::track_loader::load_track_decoder;
use amp_adapter_decoder::AudioStreamDecoder;
use amp_core_engine::dj::engine::{
    route, CueRouting, DjEngine, DjStatus, TrackBuffer, LATENCY_FRAMES,
};
use amp_core_engine::dsp::SpectrumVisualizer;
use amp_core_engine::PlaybackDspChain;
use amp_domain::Track;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

/// Frames rendered per step.
const STEP: usize = 256;
/// Source frames decoded per hand-back step.
const DECODE_FRAMES: usize = 512;
/// Largest step in output frames (a decoded step after resampling up to 4x).
const MAX_FRAMES: usize = DECODE_FRAMES * 4;
/// Crossfade between normal playback and the decks, both ways.
const CROSSFADE_SECS: f64 = 0.02;
/// Audio decoded and discarded before the hand-back blend (well past the resampler's start-up).
const PREROLL_SECS: f64 = 0.05;
/// Audio kept queued in DJ mode (about 50 ms at 48 kHz).
const LEAD_SECS: f64 = 0.05;
/// Deck A must be decoded this far past the playing position before it takes over.
const ENTER_MARGIN_SECS: f64 = 1.0;
/// Output frames rendered and thrown away before the switch, so the limiter's lookahead and
/// the deck's fade-in have settled when the first kept frame plays.
const PRIME_FRAMES: usize = 512;

struct PendingLeave {
    decoder: mpsc::Receiver<Result<AudioStreamDecoder, String>>,
    track: Option<Box<Track>>,
    deck: usize,
    status: Arc<DjStatus>,
    playing: bool,
}

/// DJ state owned by the playback worker.
pub(crate) struct DjHost {
    engine: Option<DjEngine>,
    active: bool,
    enter: Option<Option<Arc<TrackBuffer>>>,
    leave: Option<PendingLeave>,
    lead_frames: usize,
    master: Vec<f32>,
    cue: Vec<f32>,
    main: Vec<f32>,
    phones: Vec<f32>,
    decoded: Vec<f32>,
    handback: Option<Handback>,
    /// Crossfade into the decks after entering: `(frames done, fade length)`.
    fade_in: Option<(usize, usize)>,
    /// A request that waits for the switch in progress.
    deferred: Option<DjRequest>,
    /// Whether normal playback was playing when DJ mode was entered.
    entry_was_playing: bool,
    visualizer: SpectrumVisualizer,
    sample_rate: u32,
}

/// Crossfade from the decks back to normal playback.
struct Handback {
    decoder: AudioStreamDecoder,
    track: Option<Box<Track>>,
    playing: bool,
    done: usize,
    fade: usize,
    /// Output frames of pre-roll still to discard before the blend starts.
    skip: usize,
}

impl DjHost {
    pub(crate) fn new(sample_rate: u32) -> Self {
        Self {
            engine: None,
            active: false,
            enter: None,
            leave: None,
            lead_frames: (f64::from(sample_rate) * LEAD_SECS) as usize,
            master: vec![0.0; MAX_FRAMES * 2],
            cue: vec![0.0; MAX_FRAMES * 2],
            main: vec![0.0; MAX_FRAMES * 2],
            phones: vec![0.0; MAX_FRAMES * 2],
            decoded: vec![0.0; DECODE_FRAMES * 2],
            handback: None,
            fade_in: None,
            deferred: None,
            entry_was_playing: false,
            visualizer: SpectrumVisualizer::new(),
            sample_rate,
        }
    }

    /// True while the engine produces the audio.
    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    /// Picks up a request from the session.
    pub(crate) fn accept(&mut self, driver: &AudioPlaybackDriver) {
        let Some(req) = self.deferred.take().or_else(|| driver.dj.take_request()) else {
            return;
        };
        if self.engine.is_none() {
            self.engine = driver.dj.take_engine();
        }
        let crossfading_in = self.fade_in.is_some_and(|(done, _)| done > 0);
        match req {
            // A switch in progress finishes first; the request waits for it.
            r @ DjRequest::Enter { .. } if self.handback.is_some() || self.leave.is_some() => {
                self.deferred = Some(r);
            }
            r @ DjRequest::Leave { .. } if crossfading_in => self.deferred = Some(r),
            DjRequest::Enter { deck_a } if !self.active => self.enter = Some(deck_a),
            // Leaving before the decks were ever heard: the normal decoder never stopped, so
            // just carry on with it.
            DjRequest::Leave { .. } if self.fade_in.is_some() || self.enter.is_some() => {
                self.enter = None;
                self.fade_in = None;
                self.stop_decks();
                self.deactivate(driver);
                driver
                    .is_playing
                    .store(self.entry_was_playing, Ordering::SeqCst);
            }
            DjRequest::Leave {
                track,
                deck,
                status,
                playing,
            } if self.active => {
                let (tx, rx) = mpsc::channel();
                if let Some(t) = track.clone() {
                    let resolvers = driver
                        .resolvers
                        .lock()
                        .map(|r| r.clone())
                        .unwrap_or_default();
                    std::thread::spawn(move || {
                        let _ = tx.send(load_track_decoder(&t, &resolvers));
                    });
                } else {
                    let _ = tx.send(Err("no track on air".into()));
                }
                self.leave = Some(PendingLeave {
                    decoder: rx,
                    track,
                    deck,
                    status,
                    playing,
                });
            }
            _ => {}
        }
    }

    /// In normal mode: switches to the engine once deck A is decoded past the playing position.
    /// `decoder` is the normal decoder; it is dropped on the switch.
    pub(crate) fn try_enter(
        &mut self,
        driver: &AudioPlaybackDriver,
        decoder: &mut Option<AudioStreamDecoder>,
        dsp: &PlaybackDspChain,
        sample_rate: u32,
    ) -> bool {
        let Some(deck_a) = self.enter.as_ref() else {
            return false;
        };
        // What has reached the queue lags the decoder by the resampler's delay.
        let lag = dsp.resampler_delay_frames() as f64 / f64::from(sample_rate);
        let playing_at = decoder
            .as_ref()
            .map_or(0.0, |d| (d.elapsed().as_secs_f64() - lag).max(0.0));
        let ready = match deck_a {
            None => true,
            Some(buf) if buf.error().is_some() || buf.is_complete() => true,
            Some(buf) => {
                let rate = f64::from(buf.sample_rate().max(1));
                buf.sample_rate() > 0
                    && buf.loaded_frames() as f64 / rate >= playing_at + ENTER_MARGIN_SECS
            }
        };
        let Some(engine) = self.engine.as_mut().filter(|_| ready) else {
            return false;
        };
        let has_track = self.enter.take().flatten().is_some();
        let was_playing = driver.is_playing.load(Ordering::SeqCst) && decoder.is_some();
        self.entry_was_playing = driver.is_playing.load(Ordering::SeqCst);
        engine.apply_pending();
        let play = was_playing && has_track;
        // Start early by the frames that will be thrown away (minus the engine's own latency),
        // so the first frame that reaches the sound card is exactly where playback was.
        let early = (PRIME_FRAMES - LATENCY_FRAMES) as f64 / f64::from(sample_rate);
        let start = if play {
            (playing_at - early).max(0.0)
        } else {
            playing_at
        };
        engine.seek_now(0, start);
        engine.params().deck(0).set_playing(play);
        if play {
            let mut scratch = vec![0.0f32; PRIME_FRAMES * 2];
            let mut cue = vec![0.0f32; PRIME_FRAMES * 2];
            engine.render(&mut scratch, &mut cue);
        }
        if play {
            // Crossfade from the normal decoder to deck A (absorbs sub-millisecond offsets).
            self.fade_in = Some((0, (f64::from(sample_rate) * CROSSFADE_SECS) as usize));
        } else {
            *decoder = None;
        }
        driver.is_playing.store(true, Ordering::SeqCst);
        self.active = true;
        driver.dj.set_active(true);
        true
    }

    /// In DJ mode: renders while the queue is below the lead. When a leave request's decoder is
    /// ready it crossfades from the decks to it and then returns it to normal playback.
    pub(crate) fn tick(
        &mut self,
        driver: &AudioPlaybackDriver,
        dsp: &mut PlaybackDspChain,
        normal: &mut Option<AudioStreamDecoder>,
    ) -> Option<AudioStreamDecoder> {
        self.start_handback(driver, dsp);
        if self.engine.is_none() || queued_frames(driver) >= self.lead_frames {
            std::thread::sleep(Duration::from_millis(1));
            return None;
        }
        if self.handback.is_some() {
            return self.handback_step(driver, dsp);
        }
        if let (Some((done, fade)), Some(dec)) = (self.fade_in, normal.as_mut()) {
            let (frames, read) = self.blend_step(driver, dsp, dec, &mut 0, (done, fade), true);
            if done + frames >= fade || read == 0 {
                self.fade_in = None;
                *normal = None;
            } else {
                self.fade_in = Some((done + frames, fade));
            }
            return None;
        }
        self.fade_in = None;
        self.render_dj(driver, STEP);
        let volume = volume(driver);
        let out: Vec<f32> = self.main[..STEP * 2].iter().map(|s| s * volume).collect();
        self.visualizer.push_samples(&out);
        let bands = self.visualizer.compute_spectrum_bands();
        update_visualizer_and_push(driver, &out, bands);
        None
    }

    /// Renders `frames` frames of the engine into `self.main` (stereo, routed) and feeds the
    /// headphone device when the cue goes there.
    fn render_dj(&mut self, driver: &AudioPlaybackDriver, frames: usize) {
        let Some(engine) = self.engine.as_mut() else {
            return;
        };
        let n = frames * 2;
        engine.render(&mut self.master[..n], &mut self.cue[..n]);
        let params = engine.params();
        let mut phones = driver.dj.phones().try_lock().ok();
        let routing = match params.cue_routing() {
            CueRouting::SecondDevice if phones.as_ref().is_none_or(|p| p.is_none()) => {
                CueRouting::Off
            }
            // The main queue is stereo; 3/4 routing needs a 4-channel device queue.
            CueRouting::ExtraChannels => CueRouting::Off,
            r => r,
        };
        route(
            routing,
            &self.master[..n],
            &self.cue[..n],
            params.cue_mix(),
            2,
            &mut self.main[..n],
            &mut self.phones[..n],
        );
        if routing == CueRouting::SecondDevice {
            if let Some(Some(p)) = phones.as_deref_mut() {
                if (p.buffer().capacity() - p.slots()) / 2 < self.lead_frames * 2 {
                    for &s in &self.phones[..n] {
                        let _ = p.push(s);
                    }
                }
            }
        }
    }

    /// When the leave request's decoder has opened: seek it to where the on-air deck is (ahead by
    /// the resampler's delay, so both line up) and start the crossfade.
    fn start_handback(&mut self, driver: &AudioPlaybackDriver, dsp: &mut PlaybackDspChain) {
        let Some(pending) = self.leave.as_ref() else {
            return;
        };
        let result = match pending.decoder.try_recv() {
            Ok(r) => r,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => Err("decoder thread gone".into()),
        };
        let Some(pending) = self.leave.take() else {
            return;
        };
        let Ok(mut decoder) = result else {
            // Nothing to hand back to: stop like the end of a track.
            self.stop_decks();
            self.deactivate(driver);
            driver.is_playing.store(false, Ordering::SeqCst);
            return;
        };
        dsp.set_source_sample_rate(decoder.sample_rate());
        // Line the two up: the decks are heard `LATENCY_FRAMES` behind their playhead (the
        // limiter's lookahead), and the fresh resampler delays the decoder by its own delay.
        let rate = f64::from(self.sample_rate);
        let delay = dsp.resampler_delay_frames() as f64 / rate;
        let engine_lag = LATENCY_FRAMES as f64 / rate;
        let position = pending.status.deck(pending.deck).position_secs();
        let target = (position - engine_lag + delay).max(0.0);
        // Pre-roll: start earlier and throw that audio away, so the fresh resampler has real
        // audio in its history (it starts with its delay as silence) when the blend begins.
        let start = (target - PREROLL_SECS).max(0.0);
        let skip = ((target - start) * rate).round() as usize;
        let _ = decoder.seek(Duration::from_secs_f64(start));
        self.handback = Some(Handback {
            decoder,
            track: pending.track,
            playing: pending.playing,
            done: 0,
            fade: (f64::from(self.sample_rate) * CROSSFADE_SECS) as usize,
            skip,
        });
    }

    /// One crossfade step from the decks to the normal decoder; returns the decoder when done.
    fn handback_step(
        &mut self,
        driver: &AudioPlaybackDriver,
        dsp: &mut PlaybackDspChain,
    ) -> Option<AudioStreamDecoder> {
        let mut hb = self.handback.take()?;
        let (frames, read) = self.blend_step(
            driver,
            dsp,
            &mut hb.decoder,
            &mut hb.skip,
            (hb.done, hb.fade),
            false,
        );
        hb.done += frames;
        if hb.done < hb.fade && read > 0 {
            self.handback = Some(hb);
            return None;
        }
        self.stop_decks();
        self.deactivate(driver);
        if let Ok(mut t) = driver.current_track.lock() {
            *t = hb.track.map(|b| *b);
        }
        if let Ok(mut e) = driver.elapsed.lock() {
            *e = hb.decoder.elapsed();
        }
        if let Ok(mut d) = driver.total_duration.lock() {
            *d = hb.decoder.duration();
        }
        driver.is_playing.store(hb.playing, Ordering::SeqCst);
        Some(hb.decoder)
    }

    /// Decodes one chunk of normal playback, renders as many frames of the decks, and pushes
    /// their blend: the decks' share rises from `done / fade` when `to_decks`, else falls.
    /// Returns the frames pushed and the samples decoded (0 at the end of the track).
    fn blend_step(
        &mut self,
        driver: &AudioPlaybackDriver,
        dsp: &mut PlaybackDspChain,
        decoder: &mut AudioStreamDecoder,
        skip: &mut usize,
        (done, fade): (usize, usize),
        to_decks: bool,
    ) -> (usize, usize) {
        let read = decoder
            .read_interleaved_samples(&mut self.decoded)
            .unwrap_or(0);
        let gains = driver.eq_gains.lock().map(|g| *g).unwrap_or([0.0; 10]);
        let preamp = driver.eq_preamp.lock().map(|p| *p).unwrap_or(0.0);
        let eq_on = driver.eq_enabled.load(Ordering::Relaxed);
        let vol = volume(driver);
        let (processed, bands) =
            dsp.process_interleaved(&self.decoded[..read], vol, preamp, &gains, eq_on);
        // Drop pre-roll first (nothing is pushed while it lasts).
        let drop = (*skip).min(processed.len() / 2);
        *skip -= drop;
        let normal = &processed[drop * 2..];
        if normal.is_empty() && read > 0 {
            return (0, read);
        }
        let frames = (normal.len() / 2).min(self.main.len() / 2);
        self.render_dj(driver, frames);
        let mut out = Vec::with_capacity(frames * 2);
        for f in 0..frames {
            let ramp = ((done + f) as f32 / fade.max(1) as f32).min(1.0);
            let decks = if to_decks { ramp } else { 1.0 - ramp };
            for c in 0..2 {
                let i = f * 2 + c;
                out.push(self.main[i] * vol * decks + normal[i] * (1.0 - decks));
            }
        }
        update_visualizer_and_push(driver, &out, bands);
        (frames, read)
    }

    fn stop_decks(&self) {
        if let Some(engine) = self.engine.as_ref() {
            for d in 0..2 {
                engine.params().deck(d).set_playing(false);
            }
        }
    }

    fn deactivate(&mut self, driver: &AudioPlaybackDriver) {
        self.active = false;
        driver.dj.set_active(false);
    }
}

fn queued_frames(driver: &AudioPlaybackDriver) -> usize {
    driver
        .producer
        .try_lock()
        .map(|p| (p.buffer().capacity() - p.slots()) / 2)
        .unwrap_or(usize::MAX)
}

fn volume(driver: &AudioPlaybackDriver) -> f32 {
    driver.volume.lock().map(|v| *v).unwrap_or(1.0)
}
