//! Beat matching math (TASK-205): which deck leads, what tempo a synced deck must play at, how
//! far its beats are from the master's, and how hard to steer to close that gap.

#![forbid(unsafe_code)]

/// Seconds over which a synced deck closes a beat-phase error.
const CATCH_UP_SECS: f64 = 0.25;
/// Largest steering, as a fraction of the deck's speed (keeps corrections inaudible).
const MAX_STEER: f64 = 0.03;

/// Pitch slider ranges, in percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitchRange {
    /// ±8 % (classic turntable).
    Narrow,
    /// ±16 %.
    Wide,
    /// ±50 %.
    Extreme,
}

impl PitchRange {
    /// Range in percent.
    pub fn percent(self) -> f32 {
        match self {
            Self::Narrow => 8.0,
            Self::Wide => 16.0,
            Self::Extreme => 50.0,
        }
    }

    /// Tempo change in percent for a slider at `pos` in `[-1, 1]`.
    pub fn slider_to_percent(self, pos: f32) -> f32 {
        pos.clamp(-1.0, 1.0) * self.percent()
    }
}

/// What the sync logic needs to know about a deck.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeckBeat {
    /// Track tempo from analysis.
    pub bpm: f64,
    /// A downbeat, in track seconds.
    pub anchor: f64,
    /// Playhead, in track seconds.
    pub position: f64,
    /// Current speed (1 = original tempo).
    pub speed: f64,
}

impl DeckBeat {
    /// Tempo the deck is heard at.
    pub fn heard_bpm(&self) -> f64 {
        self.bpm * self.speed
    }

    /// Beats played since the anchor (fractional).
    pub fn beats(&self) -> f64 {
        (self.position - self.anchor) * self.bpm / 60.0
    }
}

/// How many master beats one beat of a track at `deck_bpm` should span so the two meet as
/// closely as possible: 1 normally, 2 or 0.5 for half or double tempo pairs (70 with 140).
pub fn beat_ratio(master_bpm: f64, deck_bpm: f64) -> f64 {
    [0.5, 1.0, 2.0]
        .into_iter()
        .min_by(|a, b| {
            let da = (master_bpm / a / deck_bpm).ln().abs();
            let db = (master_bpm / b / deck_bpm).ln().abs();
            da.total_cmp(&db)
        })
        .unwrap_or(1.0)
}

/// Speed at which `deck` matches the master's heard tempo.
pub fn sync_speed(master: &DeckBeat, deck_bpm: f64) -> f64 {
    let ratio = beat_ratio(master.heard_bpm(), deck_bpm);
    master.heard_bpm() / ratio / deck_bpm
}

/// How far `deck`'s beat is from the master's, in deck beats, wrapped to `[-0.5, 0.5)`.
/// Positive means the deck is ahead.
pub fn phase_error(master: &DeckBeat, deck: &DeckBeat) -> f64 {
    let ratio = beat_ratio(master.heard_bpm(), deck.bpm);
    let target = master.beats() / ratio;
    let e = (deck.beats() - target).rem_euclid(1.0);
    if e >= 0.5 {
        e - 1.0
    } else {
        e
    }
}

/// Speed for a synced deck: the matched tempo plus a gentle steer that removes `error_beats`
/// within about a quarter of a second.
pub fn steered_speed(base: f64, error_beats: f64, deck_bpm: f64) -> f64 {
    let error_secs = error_beats * 60.0 / deck_bpm;
    let steer = (-error_secs / CATCH_UP_SECS).clamp(-MAX_STEER * base, MAX_STEER * base);
    base + steer
}

/// The tempo master: the explicit choice if it is playing and has a grid, otherwise the playing
/// deck with a grid that is loudest on master (ties go to the earlier deck).
pub fn pick_master(explicit: Option<usize>, candidates: &[(bool, bool, f32)]) -> Option<usize> {
    if let Some(i) = explicit.filter(|i| candidates.get(*i).is_some_and(|c| c.0 && c.1)) {
        return Some(i);
    }
    candidates
        .iter()
        .enumerate()
        .filter(|(_, (playing, grid, _))| *playing && *grid)
        .fold(None, |best: Option<(usize, f32)>, (i, c)| match best {
            Some((_, g)) if g >= c.2 => best,
            _ => Some((i, c.2)),
        })
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck(bpm: f64, anchor: f64, position: f64, speed: f64) -> DeckBeat {
        DeckBeat {
            bpm,
            anchor,
            position,
            speed,
        }
    }

    #[test]
    fn dj_sync_matches_tempo_and_handles_double_time() {
        let master = deck(124.0, 0.0, 0.0, 1.0);
        assert!((sync_speed(&master, 120.0) - 124.0 / 120.0).abs() < 1e-12);
        assert!(
            (sync_speed(&master, 62.0) - 1.0).abs() < 1e-12,
            "62 locks at half time"
        );
        let pitched = deck(124.0, 0.0, 0.0, 1.05);
        assert!((sync_speed(&pitched, 124.0) - 1.05).abs() < 1e-12);
    }

    #[test]
    fn dj_sync_phase_error_wraps_to_the_nearest_beat() {
        let master = deck(120.0, 0.0, 10.0, 1.0); // exactly on a beat
        let ahead = phase_error(&master, &deck(120.0, 0.0, 20.1, 1.0));
        assert!((ahead - 0.2).abs() < 1e-9, "{ahead}");
        let behind = phase_error(&master, &deck(120.0, 0.0, 19.9, 1.0));
        assert!((behind + 0.2).abs() < 1e-9, "{behind}");
        let anchored = phase_error(&master, &deck(120.0, 0.3, 0.3, 1.0));
        assert!(anchored.abs() < 1e-9, "anchors line up");
    }

    #[test]
    fn dj_sync_steering_is_gentle_and_points_the_right_way() {
        assert!(steered_speed(1.0, 0.1, 120.0) < 1.0, "ahead → slow down");
        assert!(steered_speed(1.0, -0.1, 120.0) > 1.0, "behind → speed up");
        assert_eq!(steered_speed(1.0, 0.5, 120.0), 0.97, "never more than 3 %");
        assert_eq!(steered_speed(1.0, 0.0, 120.0), 1.0);
    }

    #[test]
    fn dj_sync_master_is_the_loudest_playing_deck_unless_chosen() {
        let decks = [
            (true, true, 0.2),
            (true, true, 0.9),
            (false, true, 1.0),
            (true, false, 1.0),
        ];
        assert_eq!(pick_master(None, &decks), Some(1));
        assert_eq!(pick_master(Some(0), &decks), Some(0));
        assert_eq!(
            pick_master(Some(2), &decks),
            Some(1),
            "a paused choice falls back"
        );
        assert_eq!(pick_master(None, &[(false, true, 1.0)]), None);
    }

    #[test]
    fn dj_pitch_ranges_scale_the_slider() {
        assert_eq!(PitchRange::Narrow.slider_to_percent(1.0), 8.0);
        assert_eq!(PitchRange::Wide.slider_to_percent(-0.5), -8.0);
        assert_eq!(PitchRange::Extreme.slider_to_percent(2.0), 50.0);
    }
}
