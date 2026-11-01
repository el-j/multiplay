//! The audio worker's side of DJ mode: runs the DJ engine in the `PlayerEngine` worker thread
//! and switches between normal playback and the DJ decks without a gap.
//!
//! - **Entering:** normal playback keeps running until deck A's copy of the track is decoded
//!   past the playing position, then deck A continues from that exact position.
//! - **Leaving:** the normal decoder (`Box<dyn PcmStreamPort>`) for the on-air deck's track opens
//!   on a helper thread while the decks keep playing; once ready it seeks to the deck's position
//!   and crossfades from the decks to normal playback over 20 ms.

#![forbid(unsafe_code)]

use super::dj_link::DjRequest;
use super::loader::open_track_blocking;
use super::pusher::push_samples_with_clock;
use super::PlayerEngine;
use crate::dj::engine::{route, CueRouting, DjEngine, DjStatus, TrackBuffer, LATENCY_FRAMES};
use crate::dsp::SpectrumVisualizer;
use crate::PlaybackDspChain;
use amp_domain::Track;
use amp_ports::{DecodeError, PcmStreamPort};
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

const STEP: usize = 256;
const DECODE_FRAMES: usize = 512;
const MAX_FRAMES: usize = DECODE_FRAMES * 4;
const CROSSFADE_SECS: f64 = 0.02;
const PREROLL_SECS: f64 = 0.05;
const LEAD_SECS: f64 = 0.05;
const ENTER_MARGIN_SECS: f64 = 1.0;
const PRIME_FRAMES: usize = 512;

struct PendingLeave {
    decoder: mpsc::Receiver<Result<Box<dyn PcmStreamPort>, DecodeError>>,
    track: Option<Box<Track>>,
    deck: usize,
    status: Arc<DjStatus>,
    playing: bool,
}

/// Crossfade from the decks back to normal playback.
struct Handback {
    decoder: Box<dyn PcmStreamPort>,
    track: Option<Box<Track>>,
    playing: bool,
    done: usize,
    fade: usize,
    skip: usize,
}

/// DJ state owned by the `PlayerEngine` playback worker.
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
    fade_in: Option<(usize, usize)>,
    deferred: Option<DjRequest>,
    entry_was_playing: bool,
    visualizer: SpectrumVisualizer,
    sample_rate: u32,
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

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn accept(&mut self, driver: &PlayerEngine) {
        let Some(req) = self.deferred.take().or_else(|| driver.dj.take_request()) else {
            return;
        };
        if self.engine.is_none() {
            self.engine = driver.dj.take_engine();
        }
        let crossfading_in = self.fade_in.is_some_and(|(done, _)| done > 0);
        match req {
            r @ DjRequest::Enter { .. } if self.handback.is_some() || self.leave.is_some() => {
                self.deferred = Some(r);
            }
            r @ DjRequest::Leave { .. } if crossfading_in => self.deferred = Some(r),
            DjRequest::Enter { deck_a } if !self.active => self.enter = Some(deck_a),
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
                    let factory = Arc::clone(&driver.decoder_factory);
                    let rt = driver.runtime_handle.clone();
                    let timeout = driver.resolve_timeout();
                    std::thread::spawn(move || {
                        let _ =
                            tx.send(open_track_blocking(&rt, &t, &factory, &resolvers, timeout));
                    });
                } else {
                    let _ = tx.send(Err(DecodeError::OpenFailed("no track on air".into())));
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

    pub(crate) fn try_enter(
        &mut self,
        driver: &PlayerEngine,
        decoder: &mut Option<Box<dyn PcmStreamPort>>,
        dsp: &PlaybackDspChain,
        sample_rate: u32,
    ) -> bool {
        let Some(deck_a) = self.enter.as_ref() else {
            return false;
        };
        let lag = dsp.resampler_delay_frames() / f64::from(sample_rate);
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
            self.fade_in = Some((0, (f64::from(sample_rate) * CROSSFADE_SECS) as usize));
        } else {
            *decoder = None;
        }
        driver.is_playing.store(true, Ordering::SeqCst);
        driver.clock.resume();
        self.active = true;
        driver.dj.set_active(true);
        true
    }

    pub(crate) fn tick(
        &mut self,
        driver: &PlayerEngine,
        dsp: &mut PlaybackDspChain,
        normal: &mut Option<Box<dyn PcmStreamPort>>,
    ) -> Option<Box<dyn PcmStreamPort>> {
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
        let vol = volume(driver);
        let out: Vec<f32> = self.main[..STEP * 2].iter().map(|s| s * vol).collect();
        self.visualizer.push_samples(&out);
        let bands = self.visualizer.spectrum_bands_throttled();
        push_samples_with_clock(driver, &out, bands);
        None
    }

    fn render_dj(&mut self, driver: &PlayerEngine, frames: usize) {
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

    fn start_handback(&mut self, driver: &PlayerEngine, dsp: &mut PlaybackDspChain) {
        let Some(pending) = self.leave.as_ref() else {
            return;
        };
        let result = match pending.decoder.try_recv() {
            Ok(r) => r,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                Err(DecodeError::OpenFailed("decoder thread gone".into()))
            }
        };
        let Some(pending) = self.leave.take() else {
            return;
        };
        let Ok(mut decoder) = result else {
            self.stop_decks();
            self.deactivate(driver);
            driver.is_playing.store(false, Ordering::SeqCst);
            return;
        };
        dsp.set_source_sample_rate(decoder.sample_rate());
        let rate = f64::from(self.sample_rate);
        let delay = dsp.resampler_delay_frames() / rate;
        let engine_lag = LATENCY_FRAMES as f64 / rate;
        let position = pending.status.deck(pending.deck).position_secs();
        let target = (position - engine_lag + delay).max(0.0);
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

    fn handback_step(
        &mut self,
        driver: &PlayerEngine,
        dsp: &mut PlaybackDspChain,
    ) -> Option<Box<dyn PcmStreamPort>> {
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
        let landed_elapsed = hb.decoder.elapsed();
        driver.clock.mark_seamless_transition(landed_elapsed);
        if let Ok(mut e) = driver.elapsed.lock() {
            *e = landed_elapsed;
        }
        if let Ok(mut d) = driver.total_duration.lock() {
            *d = hb.decoder.duration();
        }
        driver.is_playing.store(hb.playing, Ordering::SeqCst);
        if !hb.playing {
            driver.clock.request_pause((self.sample_rate * 5) / 1000);
        }
        Some(hb.decoder)
    }

    fn blend_step(
        &mut self,
        driver: &PlayerEngine,
        dsp: &mut PlaybackDspChain,
        decoder: &mut Box<dyn PcmStreamPort>,
        skip: &mut usize,
        (done, fade): (usize, usize),
        to_decks: bool,
    ) -> (usize, usize) {
        let read = decoder.read_interleaved(&mut self.decoded).unwrap_or(0);
        let gains = driver.eq_gains.lock().map(|g| *g).unwrap_or([0.0; 10]);
        let preamp = driver.eq_preamp.lock().map(|p| *p).unwrap_or(0.0);
        let eq_on = driver.eq_enabled.load(Ordering::Relaxed);
        let vol = volume(driver);
        let (processed, bands) =
            dsp.process_interleaved(&self.decoded[..read], vol, preamp, &gains, eq_on);
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
        push_samples_with_clock(driver, &out, bands);
        (frames, read)
    }

    fn stop_decks(&self) {
        if let Some(engine) = self.engine.as_ref() {
            for d in 0..2 {
                engine.params().deck(d).set_playing(false);
            }
        }
    }

    fn deactivate(&mut self, driver: &PlayerEngine) {
        self.active = false;
        driver.dj.set_active(false);
    }
}

fn queued_frames(driver: &PlayerEngine) -> usize {
    driver
        .producer
        .try_lock()
        .map(|p| (p.buffer().capacity() - p.slots()) / 2)
        .unwrap_or(usize::MAX)
}

fn volume(driver: &PlayerEngine) -> f32 {
    driver.volume.lock().map(|v| *v).unwrap_or(1.0)
}
