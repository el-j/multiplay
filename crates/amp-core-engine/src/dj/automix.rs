//! Auto mix (TASK-208): mixes from deck to deck before each track ends, like a DJ would.
//!
//! It runs on the control side (UI or a timer), never on the audio thread. On each `tick` it
//! reads the engine status and steers parameters:
//! - **Transition start.** It starts at the configured point before the outgoing track ends:
//!   beats or seconds before the end, or the detected outro. On a beat grid it starts on a
//!   downbeat, so bars line up.
//! - **Compatible tempos** (within 8 %, half or double time allowed): the incoming deck syncs to
//!   the outgoing one and starts on its first downbeat, so beats meet.
//! - **Otherwise** it is a plain equal-power crossfade.
//! - **After a transition** the freed deck is loaded from the source: the next track assigned to
//!   that deck, else the next unplayed track. With nothing left, the last track plays out.

#![forbid(unsafe_code)]

use super::engine::sync::beat_ratio;
use super::engine::{DjControl, TrackBuffer, CROSSFADER_LEFT, CROSSFADER_RIGHT};
use amp_domain::TrackAnalysis;
use std::sync::Arc;

/// Where a transition starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionStart {
    /// This many beats before the end (falls back to 8 s without a beat grid).
    BeatsBeforeEnd(f64),
    /// This many seconds before the end.
    SecondsBeforeEnd(f64),
    /// Where the track's energy drops for good near its end (falls back to 16 beats).
    Outro,
}

/// Auto-mix settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutoMixConfig {
    /// Where transitions start.
    pub start: TransitionStart,
    /// Transition length in beats of the outgoing track.
    pub length_beats: f64,
    /// Transition length without a beat grid, in seconds.
    pub fallback_secs: f64,
    /// Loudness every track is levelled to, in LUFS.
    pub target_lufs: f64,
    /// Largest tempo difference that is still synced (0.08 = 8 %).
    pub max_tempo_gap: f64,
}

impl Default for AutoMixConfig {
    fn default() -> Self {
        Self {
            start: TransitionStart::BeatsBeforeEnd(16.0),
            length_beats: 16.0,
            fallback_secs: 8.0,
            target_lufs: -14.0,
            max_tempo_gap: 0.08,
        }
    }
}

/// A track ready for a deck.
#[derive(Debug, Clone)]
pub struct AutoMixTrack {
    /// Shown in the history.
    pub title: String,
    /// The decoded (or decoding) audio.
    pub buffer: Arc<TrackBuffer>,
    /// Tempo, grid, loudness and length, when analysed.
    pub analysis: Option<TrackAnalysis>,
}

/// Where the next track for a deck comes from.
pub trait AutoMixSource {
    /// The next track for `deck`, or `None` when the playlist is used up.
    fn next_for(&mut self, deck: usize) -> Option<AutoMixTrack>;
}

/// Which deck a playlist entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assignment {
    /// Only this deck.
    Deck(usize),
    /// Either deck.
    Any,
}

/// A playlist with deck assignments, consumed in order.
#[derive(Debug, Clone)]
pub struct PlaylistSource {
    entries: Vec<(AutoMixTrack, Assignment, bool)>,
}

impl PlaylistSource {
    /// A playlist of tracks and their deck assignments.
    pub fn new(entries: Vec<(AutoMixTrack, Assignment)>) -> Self {
        Self {
            entries: entries.into_iter().map(|(t, a)| (t, a, false)).collect(),
        }
    }
}

impl AutoMixSource for PlaylistSource {
    fn next_for(&mut self, deck: usize) -> Option<AutoMixTrack> {
        let unplayed = |e: &&mut (AutoMixTrack, Assignment, bool)| !e.2;
        let pick = self
            .entries
            .iter()
            .position(|e| !e.2 && matches!(e.1, Assignment::Deck(d) if d == deck))
            .or_else(|| {
                self.entries
                    .iter()
                    .position(|e| !e.2 && e.1 == Assignment::Any)
            })
            .or_else(|| self.entries.iter_mut().position(|e| unplayed(&e)))?;
        self.entries[pick].2 = true;
        Some(self.entries[pick].0.clone())
    }
}

/// What auto mix is doing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutoMixState {
    /// One deck on air, waiting for the transition point.
    Playing {
        /// The deck on air.
        on_air: usize,
    },
    /// Mixing from one deck to the other.
    Mixing {
        /// The outgoing deck.
        from: usize,
        /// Whether the incoming deck is beat-synced.
        synced: bool,
    },
    /// Nothing left to play.
    Finished,
}

#[derive(Debug, Clone)]
struct Loaded {
    title: String,
    analysis: Option<TrackAnalysis>,
    duration: f64,
}

/// The auto-mix state machine for decks 0 and 1.
#[derive(Debug)]
pub struct AutoMix {
    config: AutoMixConfig,
    state: AutoMixState,
    decks: [Option<Loaded>; 2],
    /// Transition window in the outgoing track's seconds.
    window: (f64, f64),
    history: Vec<String>,
}

impl AutoMix {
    /// A stopped auto mix.
    pub fn new(config: AutoMixConfig) -> Self {
        Self {
            config,
            state: AutoMixState::Finished,
            decks: [None, None],
            window: (f64::MAX, f64::MAX),
            history: Vec::new(),
        }
    }

    /// Titles in the order they went on air.
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// The current state.
    pub fn state(&self) -> AutoMixState {
        self.state
    }

    /// Loads the first track on deck 0 and starts it, and cues the next one on deck 1.
    /// Returns false when the source has nothing.
    pub fn start(&mut self, control: &mut DjControl, source: &mut dyn AutoMixSource) -> bool {
        let Some(first) = source.next_for(0) else {
            return false;
        };
        self.load(control, 0, first);
        let p = control.params();
        for d in 0..2 {
            p.deck(d).set_fader(1.0);
            p.deck(d).set_sync(false);
        }
        p.set_crossfader(CROSSFADER_LEFT);
        p.set_master_deck(Some(0));
        p.deck(0).set_playing(true);
        self.go_on_air(0);
        if let Some(next) = source.next_for(1) {
            self.load(control, 1, next);
        }
        true
    }

    /// Advances the mix; call regularly (every audio block or every few tens of ms).
    pub fn tick(
        &mut self,
        control: &mut DjControl,
        source: &mut dyn AutoMixSource,
    ) -> AutoMixState {
        match self.state {
            AutoMixState::Playing { on_air } => {
                let pos = control.status().deck(on_air).position_secs();
                let other = 1 - on_air;
                if self.decks[other].is_some() && pos >= self.window.0 {
                    self.begin_mix(control, on_air);
                } else if control.status().deck(on_air).ended() {
                    control.params().deck(on_air).set_playing(false);
                    self.state = AutoMixState::Finished;
                }
            }
            AutoMixState::Mixing { from, .. } => {
                let pos = control.status().deck(from).position_secs();
                let (start, end) = self.window;
                let t = ((pos - start) / (end - start).max(1e-6)).clamp(0.0, 1.0);
                let (a, b) = side_positions(from);
                control.params().set_crossfader(a + (b - a) * t as f32);
                if t >= 1.0 || control.status().deck(from).ended() {
                    self.finish_mix(control, source, from);
                }
            }
            AutoMixState::Finished => {}
        }
        self.state
    }

    fn go_on_air(&mut self, deck: usize) {
        if let Some(l) = &self.decks[deck] {
            self.history.push(l.title.clone());
            self.window = self.plan(l);
        }
        self.state = AutoMixState::Playing { on_air: deck };
    }

    /// Transition window `(start, end)` in the track's seconds.
    fn plan(&self, l: &Loaded) -> (f64, f64) {
        let grid = l.analysis.as_ref().and_then(|a| {
            a.beat_period()
                .map(|p| (p, a.first_downbeat_secs, a.beats_per_bar))
        });
        let end = l.duration;
        let length = grid.map_or(self.config.fallback_secs, |(p, ..)| {
            self.config.length_beats * p
        });
        let wanted = match self.config.start {
            TransitionStart::SecondsBeforeEnd(s) => end - s,
            TransitionStart::BeatsBeforeEnd(b) => {
                grid.map_or(end - self.config.fallback_secs, |(p, ..)| end - b * p)
            }
            TransitionStart::Outro => {
                l.analysis
                    .as_ref()
                    .and_then(outro_start)
                    .unwrap_or_else(|| {
                        grid.map_or(end - self.config.fallback_secs, |(p, ..)| end - 16.0 * p)
                    })
            }
        };
        // Start on a downbeat at or before the wanted point, so the bars line up.
        let start = match grid {
            Some((p, anchor, bar))
                if !matches!(self.config.start, TransitionStart::SecondsBeforeEnd(_)) =>
            {
                let bar_len = p * f64::from(bar.max(1));
                anchor + ((wanted - anchor) / bar_len).floor() * bar_len
            }
            _ => wanted,
        }
        .max(0.0);
        (start, (start + length).min(end))
    }

    fn begin_mix(&mut self, control: &mut DjControl, from: usize) {
        let to = 1 - from;
        let p = control.params();
        let tempo = |d: usize| {
            self.decks[d]
                .as_ref()
                .and_then(|l| l.analysis.as_ref()?.bpm)
        };
        let synced = match (tempo(from), tempo(to)) {
            (Some(out_bpm), Some(in_bpm)) => {
                let heard = control.status().deck(from).bpm().max(out_bpm * 0.01);
                let gap = heard / beat_ratio(heard, in_bpm) / in_bpm - 1.0;
                gap.abs() <= self.config.max_tempo_gap
            }
            _ => false,
        };
        p.set_master_deck(Some(from));
        p.deck(to).set_sync(synced);
        p.deck(to).set_fader(1.0);
        p.deck(to).set_playing(true);
        self.state = AutoMixState::Mixing { from, synced };
    }

    fn finish_mix(&mut self, control: &mut DjControl, source: &mut dyn AutoMixSource, from: usize) {
        let to = 1 - from;
        let p = control.params();
        p.set_crossfader(side_positions(from).1);
        p.deck(from).set_playing(false);
        // The new track keeps the tempo it was mixed in at.
        let heard_speed =
            control.status().deck(to).bpm() / p.deck(to).grid().map_or(1.0, |(bpm, _)| bpm);
        if p.deck(to).sync() && heard_speed.is_finite() && heard_speed > 0.0 {
            p.deck(to)
                .set_pitch_percent(((heard_speed - 1.0) * 100.0) as f32);
        }
        p.deck(to).set_sync(false);
        p.set_master_deck(Some(to));
        self.decks[from] = None;
        self.go_on_air(to);
        if let Some(next) = source.next_for(from) {
            self.load(control, from, next);
        }
    }

    fn load(&mut self, control: &mut DjControl, deck: usize, t: AutoMixTrack) {
        let p = control.params_handle();
        let d = p.deck(deck);
        d.set_playing(false);
        d.set_pitch_percent(0.0);
        let analysis = t.analysis.clone();
        let (grid, lufs) = analysis.as_ref().map_or((None, None), |a| {
            (a.bpm.map(|b| (b, a.first_downbeat_secs)), a.loudness_lufs)
        });
        d.set_grid(grid.map_or(0.0, |g| g.0), grid.map_or(0.0, |g| g.1));
        d.channel()
            .set_auto_gain_db(lufs.map_or(0.0, |l| (self.config.target_lufs - l) as f32));
        let duration = analysis.as_ref().map_or_else(
            || t.buffer.loaded_frames() as f64 / f64::from(t.buffer.sample_rate().max(1)),
            |a| a.duration_secs,
        );
        control.load(deck, t.buffer);
        // Cue at the first downbeat, so a synced start lines up bars as well as beats.
        if let Some((bpm, anchor)) = grid {
            let bar = 4.0 * 60.0 / bpm;
            control.seek(deck, anchor.rem_euclid(bar));
        }
        self.decks[deck] = Some(Loaded {
            title: t.title,
            analysis,
            duration,
        });
    }
}

/// Crossfader positions for "only `deck`" and "only the other deck".
fn side_positions(deck: usize) -> (f32, f32) {
    if deck == 0 {
        (CROSSFADER_LEFT, CROSSFADER_RIGHT)
    } else {
        (CROSSFADER_RIGHT, CROSSFADER_LEFT)
    }
}

/// Where the outro starts: the last point after which the detail waveform stays below 60 % of
/// the track's median level, if that leaves at least 8 s and no more than a third of the track.
pub fn outro_start(a: &TrackAnalysis) -> Option<f64> {
    if a.detail.len() < 100 || a.detail_rate <= 0.0 {
        return None;
    }
    let level: Vec<u32> = a
        .detail
        .iter()
        .map(|c| c.iter().map(|&v| u32::from(v)).sum())
        .collect();
    let mut sorted = level.clone();
    sorted.sort_unstable();
    let median = sorted[sorted.len() / 2];
    let quiet = median * 6 / 10;
    let last_loud = level.iter().rposition(|&v| v > quiet)?;
    let at = (last_loud + 1) as f64 / f64::from(a.detail_rate);
    let left = a.duration_secs - at;
    (left >= 8.0 && left <= a.duration_secs / 3.0).then_some(at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use amp_domain::ANALYSIS_VERSION;

    #[test]
    fn dj_automix_finds_a_fading_outro() {
        let mut detail = vec![[100u8, 100, 100]; 6_000];
        for (i, c) in detail.iter_mut().enumerate().skip(4_800) {
            *c = [(30 - (i - 4_800) / 60) as u8; 3];
        }
        let a = TrackAnalysis {
            version: ANALYSIS_VERSION,
            duration_secs: 60.0,
            bpm: Some(120.0),
            first_downbeat_secs: 0.0,
            beats_per_bar: 4,
            overview: Vec::new(),
            detail,
            detail_rate: 100.0,
            loudness_lufs: None,
        };
        assert_eq!(outro_start(&a), Some(48.0));
    }
}
