//! Key lock: tempo changes without pitch changes, by WSOLA on the deck playhead.
//!
//! Every hop the deck plays one Hann-windowed grain read at the track's native speed, so its
//! pitch never moves. Grains start where the playhead is (which moves at the new tempo), nudged
//! by up to a few milliseconds to where the audio best continues the previous grain, and
//! overlap-add with 50 % overlap. Buffers are allocated once; producing audio never allocates.

#![forbid(unsafe_code)]

use super::player::read_frame;
use super::track_buffer::TrackBuffer;

/// Grain length in output frames (about 43 ms at 48 kHz).
const GRAIN: usize = 2_048;
const HOP: usize = GRAIN / 2;
/// Largest grain shift either way, in source frames.
const SEARCH: isize = 256;
/// Source frames compared when choosing the shift (every 4th one is used).
const CORR: usize = 512;
const CORR_STEP: usize = 4;

/// Key-lock state of one deck.
#[derive(Debug)]
pub(crate) struct KeyLock {
    window: Box<[f32]>,
    tail: Box<[f32]>,
    hop: Box<[f32]>,
    reference: Box<[f32]>,
    next: usize,
    prev_start: Option<f64>,
}

impl Default for KeyLock {
    fn default() -> Self {
        let window = (0..GRAIN)
            .map(|i| {
                let x = std::f32::consts::PI * i as f32 / GRAIN as f32;
                x.sin() * x.sin()
            })
            .collect();
        Self {
            window,
            tail: vec![0.0; HOP * 2].into_boxed_slice(),
            hop: vec![0.0; HOP * 2].into_boxed_slice(),
            reference: vec![0.0; CORR / CORR_STEP].into_boxed_slice(),
            next: HOP,
            prev_start: None,
        }
    }
}

impl KeyLock {
    /// Forgets the previous grain (after a seek, a load, or switching key lock on).
    pub(crate) fn reset(&mut self) {
        self.next = HOP;
        self.prev_start = None;
        self.tail.fill(0.0);
    }

    /// Source frames past the playhead that must be decoded before the next grain is built.
    pub(crate) fn lookahead(native: f64) -> usize {
        (GRAIN as f64 * native).ceil() as usize + SEARCH as usize + 4
    }

    /// The next output frame for a deck whose playhead is at `position` (source frames) and
    /// whose track plays `native` source frames per output frame at its original pitch.
    pub(crate) fn next_frame(
        &mut self,
        track: &TrackBuffer,
        position: f64,
        native: f64,
    ) -> (f32, f32) {
        if self.next >= HOP {
            self.build(track, position, native);
        }
        let i = self.next;
        self.next += 1;
        (self.hop[i * 2], self.hop[i * 2 + 1])
    }

    fn build(&mut self, track: &TrackBuffer, nominal: f64, native: f64) {
        let start = match self.prev_start {
            Some(prev) => self.best_start(track, prev + HOP as f64 * native, nominal, native),
            None => nominal,
        };
        for j in 0..GRAIN {
            let (l, r) = read_frame(track, start + j as f64 * native);
            let w = self.window[j];
            if j < HOP {
                self.hop[j * 2] = self.tail[j * 2] + l * w;
                self.hop[j * 2 + 1] = self.tail[j * 2 + 1] + r * w;
            } else {
                let k = j - HOP;
                self.tail[k * 2] = l * w;
                self.tail[k * 2 + 1] = r * w;
            }
        }
        self.prev_start = Some(start);
        self.next = 0;
    }

    /// The grain start within `SEARCH` frames of `nominal` whose audio best matches the natural
    /// continuation `cont` of the previous grain (normalised cross-correlation).
    fn best_start(&mut self, track: &TrackBuffer, cont: f64, nominal: f64, native: f64) -> f64 {
        let mono = |pos: f64| {
            if pos < 0.0 {
                return 0.0;
            }
            track
                .frame(pos.round() as usize)
                .map_or(0.0, |(l, r)| 0.5 * (l + r))
        };
        for (k, v) in self.reference.iter_mut().enumerate() {
            *v = mono(cont + (k * CORR_STEP) as f64 * native);
        }
        let (mut best, mut best_score) = (nominal, f32::MIN);
        let mut delta = -SEARCH;
        while delta <= SEARCH {
            let candidate = nominal + delta as f64;
            if candidate >= 0.0 {
                let (mut dot, mut energy) = (0.0f32, 1e-9f32);
                for (k, &rv) in self.reference.iter().enumerate() {
                    let v = mono(candidate + (k * CORR_STEP) as f64 * native);
                    dot += rv * v;
                    energy += v * v;
                }
                let score = dot / energy.sqrt();
                if score > best_score {
                    best_score = score;
                    best = candidate;
                }
            }
            delta += 2;
        }
        best
    }
}
