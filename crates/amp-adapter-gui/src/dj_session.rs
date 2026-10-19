//! The GUI's side of DJ mode (TASK-209): owns the engine's control handle, loads decks, applies
//! analysis results (beat grid, loudness) and hot cues when they arrive, and asks the audio
//! worker to enter or leave DJ mode. Free of egui so it can be driven by tests.

#![forbid(unsafe_code)]

use crate::audio_driver::AudioPlaybackDriver;
use crate::dj_link::DjRequest;
use crate::track_loader::load_track_decoder;
use amp_adapter_storage::RedbStorageAdapter;
use amp_core_engine::dj::analysis::spawn_analysis;
use amp_core_engine::dj::automix::{
    AutoMix, AutoMixConfig, AutoMixSource, AutoMixState, AutoMixTrack,
};
use amp_core_engine::dj::engine::{
    DjControl, DjEngine, DjParams, DjStatus, TrackBuffer, CROSSFADER_LEFT, HOT_CUES,
};
use amp_domain::{
    analysis_key, next_for_deck, DeckAssignment, HotCues, Track, TrackAnalysis, TrackId,
};
use amp_ports::{AnalysisCachePort, HotCueStorePort, PcmStreamPort, StreamResolverPort};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

/// Loudness every deck is levelled to.
const TARGET_LUFS: f64 = -14.0;

/// A track on a deck.
#[derive(Debug, Clone)]
pub struct DeckSlot {
    /// The track.
    pub track: Track,
    /// Its decoded audio.
    pub buffer: Arc<TrackBuffer>,
    /// Filled when analysis finishes (or comes from the cache).
    pub analysis: Arc<OnceLock<TrackAnalysis>>,
    key: String,
    applied: bool,
    cues_dirty: bool,
}

impl DeckSlot {
    /// The analysis, once available.
    pub fn analysis(&self) -> Option<&TrackAnalysis> {
        self.analysis.get()
    }
}

/// DJ mode as seen from the GUI.
pub struct DjSession {
    control: DjControl,
    decks: [Option<DeckSlot>; 2],
    active: bool,
    library: Option<Arc<RedbStorageAdapter>>,
    automix: Option<AutoMix>,
}

impl std::fmt::Debug for DjSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DjSession")
            .field("active", &self.active)
            .field("decks", &self.decks)
            .finish()
    }
}

impl DjSession {
    /// Creates the engine at the sound card's rate and hands it to the audio worker.
    /// `library` caches analysis and stores hot cues.
    pub fn new(driver: &AudioPlaybackDriver, library: Option<Arc<RedbStorageAdapter>>) -> Self {
        let (engine, control) = DjEngine::new(driver.sample_rate(), 2);
        let p = control.params();
        for d in 0..2 {
            p.deck(d).set_sync(true);
            p.deck(d).set_quantize(true);
        }
        p.set_crossfader(CROSSFADER_LEFT);
        driver.dj.install_engine(engine);
        Self {
            control,
            decks: [None, None],
            active: false,
            library,
            automix: None,
        }
    }

    /// Mixer and deck controls.
    pub fn params(&self) -> &DjParams {
        self.control.params()
    }

    /// What the decks are doing.
    pub fn status(&self) -> &DjStatus {
        self.control.status()
    }

    /// The engine's control handle (loops, cues, seeks).
    pub fn control(&mut self) -> &mut DjControl {
        &mut self.control
    }

    /// The track on `deck`.
    pub fn deck(&self, deck: usize) -> Option<&DeckSlot> {
        self.decks.get(deck)?.as_ref()
    }

    /// True while DJ mode is on.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Turns DJ mode on: the playing track continues as deck A.
    pub fn enter(&mut self, driver: &AudioPlaybackDriver) {
        if self.active {
            return;
        }
        let current = driver.current_track.lock().ok().and_then(|t| t.clone());
        let deck_a = current.map(|t| self.load(driver, 0, t));
        self.params().set_crossfader(CROSSFADER_LEFT);
        self.params().deck(0).set_fader(1.0);
        driver.dj.request(DjRequest::Enter { deck_a });
        self.active = true;
    }

    /// Turns DJ mode off: the deck on air carries on as normal playback.
    pub fn leave(&mut self, driver: &AudioPlaybackDriver) {
        if !self.active {
            return;
        }
        let deck = self.on_air_deck();
        let track = self.deck(deck).map(|s| Box::new(s.track.clone()));
        let playing = self.params().deck(deck).playing();
        driver.dj.request(DjRequest::Leave {
            track,
            deck,
            status: self.control.status_handle(),
            playing,
        });
        self.active = false;
    }

    /// The deck the audience hears: the tempo master if there is one, else the loudest playing
    /// deck, else deck A.
    pub fn on_air_deck(&self) -> usize {
        if let Some(d) = (0..2).find(|d| self.status().deck(*d).is_master()) {
            return d;
        }
        let p = self.params();
        let (l, r) = amp_core_engine::dj::engine::curve_gains(p.master().curve(), p.crossfader());
        let loud = |d: usize| {
            let side = if d == 0 { l } else { r };
            if p.deck(d).playing() {
                p.deck(d).fader() * side
            } else {
                0.0
            }
        };
        if loud(1) > loud(0) {
            1
        } else {
            0
        }
    }

    /// Loads `track` onto `deck` (paused at the start) and starts decoding and analysing it.
    pub fn load(
        &mut self,
        driver: &AudioPlaybackDriver,
        deck: usize,
        track: Track,
    ) -> Arc<TrackBuffer> {
        let resolvers = driver
            .resolvers
            .lock()
            .map(|r| r.clone())
            .unwrap_or_default();
        let prepared = prepare_track(&track, resolvers, self.library.clone());
        self.control.load(deck, Arc::clone(&prepared.buffer));
        let buffer = Arc::clone(&prepared.buffer);
        self.adopt(deck, track, prepared);
        buffer
    }

    /// Records `track` (already sent to the engine) as `deck`'s track and restores its hot cues.
    fn adopt(&mut self, deck: usize, track: Track, prepared: Prepared) {
        self.params().deck(deck).set_grid(0.0, 0.0);
        if let Some(lib) = &self.library {
            for (slot, cue) in lib.load_hot_cues(&prepared.key).into_iter().enumerate() {
                self.control.set_hot_cue(deck, slot, cue);
            }
        }
        self.decks[deck] = Some(DeckSlot {
            track,
            buffer: prepared.buffer,
            analysis: prepared.analysis,
            key: prepared.key,
            applied: false,
            cues_dirty: false,
        });
    }

    /// True while auto mix runs.
    pub fn automix_on(&self) -> bool {
        self.automix.is_some()
    }

    /// Starts auto mix from the deck on air (or from the first assigned track when nothing
    /// plays).
    pub fn start_automix(&mut self, source: &mut PlaylistDeckSource<'_>) {
        let mut auto = AutoMix::new(AutoMixConfig::default());
        let on_air = self.on_air_deck();
        let current = self
            .deck(on_air)
            .filter(|_| self.params().deck(on_air).playing())
            .map(|s| AutoMixTrack {
                title: s.track.metadata.title.clone(),
                buffer: Arc::clone(&s.buffer),
                analysis: s.analysis.get().cloned(),
                pending: Some(Arc::clone(&s.analysis)),
            });
        match current {
            Some(t) => auto.take_over(&mut self.control, source, on_air, t),
            None => {
                auto.start(&mut self.control, source);
            }
        }
        self.automix = Some(auto);
        self.adopt_loaded(source);
    }

    /// Stops auto mix; the decks keep playing as they are.
    pub fn stop_automix(&mut self) {
        self.automix = None;
    }

    /// Advances auto mix (call every UI frame while it runs).
    pub fn tick_automix(&mut self, source: &mut PlaylistDeckSource<'_>) {
        let Some(auto) = self.automix.as_mut() else {
            return;
        };
        if auto.tick(&mut self.control, source) == AutoMixState::Finished {
            self.automix = None;
        }
        self.adopt_loaded(source);
    }

    fn adopt_loaded(&mut self, source: &mut PlaylistDeckSource<'_>) {
        for (deck, track, prepared) in source.loaded.drain(..) {
            self.adopt(deck, track, prepared);
        }
    }

    /// Stores the playhead in hot cue `slot` (saved to the library on the next `poll`).
    pub fn mark_hot_cue(&mut self, deck: usize, slot: usize) {
        self.control.mark_hot_cue(deck, slot);
        if let Some(Some(s)) = self.decks.get_mut(deck) {
            s.cues_dirty = true;
        }
    }

    /// Clears hot cue `slot`.
    pub fn clear_hot_cue(&mut self, deck: usize, slot: usize) {
        self.control.set_hot_cue(deck, slot, None);
        if let Some(Some(s)) = self.decks.get_mut(deck) {
            s.cues_dirty = true;
        }
    }

    /// Call every UI frame: applies finished analyses and saves changed hot cues.
    pub fn poll(&mut self) {
        for deck in 0..2 {
            let Some(slot) = self.decks[deck].as_mut() else {
                continue;
            };
            if !slot.applied {
                if let Some(a) = slot.analysis.get() {
                    let p = self.control.params();
                    p.deck(deck)
                        .set_grid(a.bpm.unwrap_or(0.0), a.first_downbeat_secs);
                    if let Some(lufs) = a.loudness_lufs {
                        p.deck(deck)
                            .channel()
                            .set_auto_gain_db((TARGET_LUFS - lufs) as f32);
                    }
                    slot.applied = true;
                }
            }
            if slot.cues_dirty {
                let status = self.control.status().deck(deck);
                let cues: HotCues = std::array::from_fn(|i| status.hot_cue(i));
                if let Some(lib) = &self.library {
                    let _ = lib.store_hot_cues(&slot.key, &cues);
                }
                slot.cues_dirty = false;
            }
        }
        self.control.collect_garbage();
    }
}

/// Hot cue slots per deck.
pub const CUE_SLOTS: usize = HOT_CUES;

/// A track being decoded and analysed for a deck.
#[derive(Debug, Clone)]
pub struct Prepared {
    buffer: Arc<TrackBuffer>,
    analysis: Arc<OnceLock<TrackAnalysis>>,
    key: String,
}

/// Starts decoding `track` and analysing it (or reading the cached analysis).
pub fn prepare_track(
    track: &Track,
    resolvers: Vec<Arc<dyn StreamResolverPort>>,
    library: Option<Arc<RedbStorageAdapter>>,
) -> Prepared {
    let source = track.clone();
    let buffer = amp_core_engine::dj::engine::spawn_loader(move || {
        load_track_decoder(&source, &resolvers).map(|d| Box::new(d) as Box<dyn PcmStreamPort>)
    });
    let key = analysis_key(&track.source);
    let cache = library.map(|l| l as Arc<dyn AnalysisCachePort>);
    let analysis = spawn_analysis(key.clone(), Arc::clone(&buffer), cache);
    Prepared {
        buffer,
        analysis,
        key,
    }
}

/// Auto mix's view of the playlist: each deck takes its next assigned, unplayed track.
pub struct PlaylistDeckSource<'a> {
    /// The playlist in order.
    pub playlist: &'a [Track],
    /// Deck assignment per track.
    pub assign: &'a HashMap<TrackId, DeckAssignment>,
    /// Tracks already loaded onto a deck; picked tracks are added.
    pub played: &'a mut HashSet<TrackId>,
    /// Stream resolvers for online sources.
    pub resolvers: Vec<Arc<dyn StreamResolverPort>>,
    /// Analysis cache and hot cues.
    pub library: Option<Arc<RedbStorageAdapter>>,
    loaded: Vec<(usize, Track, Prepared)>,
}

impl<'a> PlaylistDeckSource<'a> {
    /// A source over the playlist.
    pub fn new(
        playlist: &'a [Track],
        assign: &'a HashMap<TrackId, DeckAssignment>,
        played: &'a mut HashSet<TrackId>,
        resolvers: Vec<Arc<dyn StreamResolverPort>>,
        library: Option<Arc<RedbStorageAdapter>>,
    ) -> Self {
        Self {
            playlist,
            assign,
            played,
            resolvers,
            library,
            loaded: Vec::new(),
        }
    }

    /// Index of the next track for `deck`, without taking it.
    pub fn peek(&self, deck: usize) -> Option<usize> {
        let order: Vec<TrackId> = self.playlist.iter().map(|t| t.id).collect();
        next_for_deck(&order, self.assign, self.played, deck)
    }
}

impl AutoMixSource for PlaylistDeckSource<'_> {
    fn next_for(&mut self, deck: usize) -> Option<AutoMixTrack> {
        let track = self.playlist.get(self.peek(deck)?)?.clone();
        self.played.insert(track.id);
        let prepared = prepare_track(&track, self.resolvers.clone(), self.library.clone());
        let t = AutoMixTrack {
            title: track.metadata.title.clone(),
            buffer: Arc::clone(&prepared.buffer),
            analysis: prepared.analysis.get().cloned(),
            pending: Some(Arc::clone(&prepared.analysis)),
        };
        self.loaded.push((deck, track, prepared));
        Some(t)
    }
}
