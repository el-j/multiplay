//! One deck's playhead: reads its track at a variable rate (source rate → engine rate, times the
//! tempo) with 4-point Hermite interpolation. Real-time safe: no locks, no allocation.

#![forbid(unsafe_code)]

use super::params::DeckParams;
use super::track_buffer::TrackBuffer;
use std::sync::Arc;

/// Render-thread state of a deck.
#[derive(Debug, Default)]
pub(crate) struct DeckPlayer {
    pub(crate) track: Option<Arc<TrackBuffer>>,
    /// Playhead in source frames.
    pub(crate) position: f64,
    pub(crate) ended: bool,
    /// Master and cue gains applied at the end of the previous block (ramped from, so gain
    /// changes never click).
    pub(crate) gain: f32,
    pub(crate) cue_gain: f32,
}

/// What happened while rendering a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Played {
    /// Nothing to play (no track, paused, ended).
    Silent,
    /// Audio for the whole block.
    Full,
    /// Decoding lagged behind the playhead; the rest of the block is silence.
    Starved,
}

impl DeckPlayer {
    /// Replaces the track and rewinds; returns the previous track so the caller can free it
    /// away from the render thread.
    pub(crate) fn load(&mut self, track: Option<Arc<TrackBuffer>>) -> Option<Arc<TrackBuffer>> {
        self.position = 0.0;
        self.ended = false;
        std::mem::replace(&mut self.track, track)
    }

    /// Moves the playhead to `secs` into the track.
    pub(crate) fn seek(&mut self, secs: f64) {
        let rate = self.track.as_ref().map_or(0, |t| t.sample_rate());
        if rate > 0 {
            self.position = (secs.max(0.0) * f64::from(rate)).floor();
            self.ended = false;
        }
    }

    /// Seconds into the track and seconds loaded.
    pub(crate) fn times(&self) -> (f64, f64) {
        match &self.track {
            Some(t) if t.sample_rate() > 0 => {
                let r = f64::from(t.sample_rate());
                (self.position / r, t.loaded_frames() as f64 / r)
            }
            _ => (0.0, 0.0),
        }
    }

    /// Writes the deck's signal for `out.len() / 2` stereo frames at `engine_rate`.
    pub(crate) fn render(&mut self, out: &mut [f32], engine_rate: u32, p: &DeckParams) -> Played {
        out.fill(0.0);
        let Some(track) = self.track.as_deref() else {
            return Played::Silent;
        };
        let rate = track.sample_rate();
        if rate == 0 || !p.playing() || self.ended || engine_rate == 0 {
            return Played::Silent;
        }
        let step =
            f64::from(rate) / f64::from(engine_rate) * f64::from(1.0 + p.pitch_percent() / 100.0);
        let loaded = track.loaded_frames();
        let complete = track.is_complete();
        for frame in out.as_chunks_mut::<2>().0 {
            let i = self.position.floor() as usize;
            if complete && i >= loaded {
                self.position = loaded as f64;
                self.ended = true;
                return Played::Full;
            }
            if !complete && i + 2 >= loaded {
                return Played::Starved;
            }
            let t = (self.position - i as f64) as f32;
            let at = |k: isize| {
                let idx = i as isize + k;
                if idx < 0 {
                    (0.0, 0.0)
                } else {
                    track.frame(idx as usize).unwrap_or((0.0, 0.0))
                }
            };
            let (a, b, c, d) = (at(-1), at(0), at(1), at(2));
            frame[0] = hermite(a.0, b.0, c.0, d.0, t);
            frame[1] = hermite(a.1, b.1, c.1, d.1, t);
            self.position += step;
        }
        Played::Full
    }
}

/// 4-point, 3rd-order Hermite interpolation between `y1` and `y2` at `t` in `[0, 1)`.
#[inline]
fn hermite(y0: f32, y1: f32, y2: f32, y3: f32, t: f32) -> f32 {
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * t + c2) * t + c1) * t + y1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dj_hermite_passes_through_samples_and_lines() {
        assert_eq!(hermite(0.0, 1.0, 2.0, 3.0, 0.0), 1.0);
        assert!((hermite(0.0, 1.0, 2.0, 3.0, 0.5) - 1.5).abs() < 1e-6);
        assert!((hermite(5.0, 5.0, 5.0, 5.0, 0.3) - 5.0).abs() < 1e-6);
    }

    #[test]
    fn dj_seek_lands_on_the_requested_second() {
        let mut deck = DeckPlayer::default();
        deck.load(Some(TrackBuffer::from_samples(
            48_000,
            &vec![0.0; 96_000 * 2],
        )));
        deck.seek(1.25);
        assert_eq!(deck.times().0, 1.25);
    }

    #[test]
    fn dj_starved_deck_holds_its_position() {
        let buf = TrackBuffer::empty();
        let mut w = super::super::track_buffer::TrackWriter::new(Arc::clone(&buf), 48_000);
        w.push(&vec![0.1; super::super::track_buffer::CHUNK_FRAMES * 2]);
        let mut deck = DeckPlayer::default();
        deck.load(Some(buf));
        let p = super::super::params::DjParams::default();
        p.deck(0).set_playing(true);
        let mut out = vec![0.0; 40_000];
        assert_eq!(deck.render(&mut out, 48_000, p.deck(0)), Played::Starved);
        let pos = deck.position;
        assert_eq!(deck.render(&mut out, 48_000, p.deck(0)), Played::Starved);
        assert_eq!(deck.position, pos, "no skipping ahead while starved");
    }
}
