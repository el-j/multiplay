//! The DJ engine's render side and the control handle that drives it.
//!
//! [`DjEngine::render`] mixes every deck into a master bus (faders and crossfader) and a cue bus
//! (decks with PFL on, before their fader) and is real-time safe: parameters arrive through
//! atomics, discrete actions (load, seek) through a lock-free queue, and replaced tracks travel
//! back to the control side to be freed there.

#![forbid(unsafe_code)]

use super::params::{CrossfaderSide, DeckParams, DjParams, DjStatus, MAX_DECKS};
use super::player::{DeckPlayer, Played};
use super::sync::{phase_error, pick_master, steered_speed, sync_speed, DeckBeat};
use super::track_buffer::TrackBuffer;
use std::sync::Arc;

/// Frames mixed per inner block; bounds the deck scratch buffer.
const BLOCK: usize = 256;
const QUEUE: usize = 64;

/// Discrete actions for the render thread.
#[derive(Debug)]
pub enum DjCommand {
    /// Put a track on a deck (from the start, paused state unchanged).
    Load(usize, Arc<TrackBuffer>),
    /// Move a deck's playhead to a time in seconds.
    Seek(usize, f64),
    /// Empty a deck.
    Eject(usize),
}

/// Render side; owned by the thread that produces audio.
#[derive(Debug)]
pub struct DjEngine {
    sample_rate: u32,
    deck_count: usize,
    decks: [DeckPlayer; MAX_DECKS],
    scratch: Box<[f32]>,
    params: Arc<DjParams>,
    status: Arc<DjStatus>,
    commands: rtrb::Consumer<DjCommand>,
    retired: rtrb::Producer<Arc<TrackBuffer>>,
}

/// Control side; owned by the UI or automation.
#[derive(Debug)]
pub struct DjControl {
    params: Arc<DjParams>,
    status: Arc<DjStatus>,
    commands: rtrb::Producer<DjCommand>,
    retired: rtrb::Consumer<Arc<TrackBuffer>>,
}

impl DjEngine {
    /// An engine rendering at `sample_rate` with `decks` active decks (at most 4).
    pub fn new(sample_rate: u32, decks: usize) -> (Self, DjControl) {
        let (cmd_tx, cmd_rx) = rtrb::RingBuffer::new(QUEUE);
        let (old_tx, old_rx) = rtrb::RingBuffer::new(QUEUE);
        let params = Arc::new(DjParams::default());
        let status = Arc::new(DjStatus::default());
        let engine = Self {
            sample_rate,
            deck_count: decks.clamp(1, MAX_DECKS),
            decks: Default::default(),
            scratch: vec![0.0; BLOCK * 2].into_boxed_slice(),
            params: Arc::clone(&params),
            status: Arc::clone(&status),
            commands: cmd_rx,
            retired: old_tx,
        };
        let control = DjControl {
            params,
            status,
            commands: cmd_tx,
            retired: old_rx,
        };
        (engine, control)
    }

    /// Output sample rate in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// The controls this engine reads.
    pub fn params(&self) -> Arc<DjParams> {
        Arc::clone(&self.params)
    }

    /// Mixes the next `master.len() / 2` stereo frames into `master` and the cue bus into
    /// `cue` (same length). Never allocates, locks or frees.
    pub fn render(&mut self, master: &mut [f32], cue: &mut [f32]) {
        self.apply_commands();
        let len = master.len().min(cue.len());
        master.fill(0.0);
        cue.fill(0.0);
        let mut at = 0;
        while at < len {
            let n = (len - at).min(BLOCK * 2) / 2 * 2;
            if n == 0 {
                break;
            }
            self.mix_block(&mut master[at..at + n], &mut cue[at..at + n]);
            at += n;
        }
        for s in master.iter_mut().chain(cue.iter_mut()) {
            // Safety net until the master limiter lands (TASK-207).
            *s = s.clamp(-1.0, 1.0);
        }
    }

    fn mix_block(&mut self, master: &mut [f32], cue: &mut [f32]) {
        let (left, right) = crossfader_gains(self.params.crossfader());
        let speeds = self.plan_speeds(left, right);
        let scratch = &mut self.scratch[..master.len()];
        for (d, &speed) in speeds.iter().enumerate().take(self.deck_count) {
            let p = self.params.deck(d);
            let deck = &mut self.decks[d];
            let playing = p.playing();
            let played = deck.render(scratch, self.sample_rate, playing, speed, p.key_lock());
            deck.was_playing = playing;
            let status = self.status.deck(d);
            if played == Played::Starved {
                status.count_underrun();
            }
            let gain = p.trim() * p.fader() * side_gain(p, left, right);
            let cue_gain = if p.pfl() { p.trim() } else { 0.0 };
            add_ramped(master, scratch, deck.gain, gain);
            add_ramped(cue, scratch, deck.cue_gain, cue_gain);
            deck.gain = gain;
            deck.cue_gain = cue_gain;
            let (pos, loaded) = deck.times();
            status.publish(pos, loaded, deck.ended);
        }
    }

    /// Speed of every deck for the next block: its own pitch and nudge, or, when synced, the
    /// master's tempo with a gentle steer onto the master's beat. A synced deck that starts
    /// playing jumps straight into phase (it starts from silence, so the jump is inaudible).
    fn plan_speeds(&mut self, left: f32, right: f32) -> [f64; MAX_DECKS] {
        let mut beats: [Option<DeckBeat>; MAX_DECKS] = [None; MAX_DECKS];
        let mut candidates = [(false, false, 0.0f32); MAX_DECKS];
        let mut manual = [1.0f64; MAX_DECKS];
        for d in 0..self.deck_count {
            let p = self.params.deck(d);
            let deck = &self.decks[d];
            manual[d] = f64::from(1.0 + p.pitch_percent() / 100.0 + p.nudge()).max(0.0);
            beats[d] = p.grid().map(|(bpm, anchor)| DeckBeat {
                bpm,
                anchor,
                position: deck.times().0,
                speed: manual[d],
            });
            let playing = p.playing() && deck.track.is_some() && !deck.ended;
            let loud = p.trim() * p.fader() * side_gain(p, left, right);
            candidates[d] = (playing, beats[d].is_some(), loud);
        }
        let lead = pick_master(self.params.master_deck(), &candidates[..self.deck_count]);
        let mut speeds = manual;
        for d in 0..self.deck_count {
            let p = self.params.deck(d);
            if let (Some(m), Some(me), true) = (lead, beats[d], p.sync()) {
                if let (Some(mb), true) = (beats[m], m != d) {
                    let base = sync_speed(&mb, me.bpm);
                    let err = phase_error(&mb, &me);
                    let starting = p.playing() && !self.decks[d].was_playing;
                    speeds[d] = if starting || self.decks[d].resnap {
                        // Snap back onto the beat, or forward when that would pass the start.
                        let period = 60.0 / me.bpm;
                        let mut delta = -err * period;
                        if me.position + delta < 0.0 {
                            delta += period;
                        }
                        self.decks[d].shift(delta);
                        base
                    } else {
                        steered_speed(base, err, me.bpm)
                    };
                }
            }
            self.decks[d].resnap = false;
            let heard = beats[d].map_or(0.0, |b| b.bpm * speeds[d]);
            self.status.deck(d).publish_tempo(heard, lead == Some(d));
        }
        speeds
    }

    fn apply_commands(&mut self) {
        while let Ok(cmd) = self.commands.pop() {
            let (deck, old) = match cmd {
                DjCommand::Load(d, t) if d < MAX_DECKS => (d, self.decks[d].load(Some(t))),
                DjCommand::Eject(d) if d < MAX_DECKS => (d, self.decks[d].load(None)),
                DjCommand::Seek(d, secs) if d < MAX_DECKS => {
                    self.decks[d].seek(secs);
                    continue;
                }
                _ => continue,
            };
            let _ = deck;
            if let Some(old) = old {
                old.cancel();
                // A full queue would mean the control side stopped collecting; then the old
                // track is freed here as a last resort.
                let _ = self.retired.push(old);
            }
        }
    }
}

impl DjControl {
    /// Mixer and deck controls.
    pub fn params(&self) -> &DjParams {
        &self.params
    }

    /// Shared handle to the controls, for other threads (UI, automation).
    pub fn params_handle(&self) -> Arc<DjParams> {
        Arc::clone(&self.params)
    }

    /// What the decks are doing.
    pub fn status(&self) -> &DjStatus {
        &self.status
    }

    /// Shared handle to the status.
    pub fn status_handle(&self) -> Arc<DjStatus> {
        Arc::clone(&self.status)
    }

    /// Puts `track` on `deck`. Returns false when the command queue is full.
    pub fn load(&mut self, deck: usize, track: Arc<TrackBuffer>) -> bool {
        self.collect_garbage();
        self.commands.push(DjCommand::Load(deck, track)).is_ok()
    }

    /// Moves `deck`'s playhead to `secs`.
    pub fn seek(&mut self, deck: usize, secs: f64) -> bool {
        self.commands.push(DjCommand::Seek(deck, secs)).is_ok()
    }

    /// Empties `deck`.
    pub fn eject(&mut self, deck: usize) -> bool {
        self.collect_garbage();
        self.commands.push(DjCommand::Eject(deck)).is_ok()
    }

    /// Frees tracks the render thread has let go of.
    pub fn collect_garbage(&mut self) {
        while self.retired.pop().is_ok() {}
    }
}

/// The crossfader gain for deck `p` given the left and right side gains.
fn side_gain(p: &DeckParams, left: f32, right: f32) -> f32 {
    match p.side() {
        CrossfaderSide::Left => left,
        CrossfaderSide::Right => right,
        CrossfaderSide::Thru => 1.0,
    }
}

/// Equal-power crossfader gains for the left and right sides at `pos` in `[-1, 1]`.
pub fn crossfader_gains(pos: f32) -> (f32, f32) {
    let t = (pos.clamp(-1.0, 1.0) + 1.0) * 0.25 * std::f32::consts::PI;
    (t.cos(), t.sin())
}

/// Adds `src * gain` to `dst`, ramping the gain linearly from `from` to `to` across the block.
fn add_ramped(dst: &mut [f32], src: &[f32], from: f32, to: f32) {
    if from == 0.0 && to == 0.0 {
        return;
    }
    let frames = (dst.len() / 2).max(1) as f32;
    let step = (to - from) / frames;
    for (i, (d, s)) in dst
        .as_chunks_mut::<2>()
        .0
        .iter_mut()
        .zip(src.as_chunks::<2>().0)
        .enumerate()
    {
        let g = from + step * (i + 1) as f32;
        d[0] += s[0] * g;
        d[1] += s[1] * g;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dj_crossfader_is_equal_power() {
        let (l, r) = crossfader_gains(0.0);
        assert!((l * l + r * r - 1.0).abs() < 1e-6);
        assert!((l - r).abs() < 1e-6);
        assert_eq!(crossfader_gains(-1.0).0, 1.0);
        assert!(crossfader_gains(-1.0).1.abs() < 1e-6);
        assert!(crossfader_gains(1.0).0.abs() < 1e-6);
    }

    #[test]
    fn dj_gain_ramps_without_jumps() {
        let src = vec![1.0; 8];
        let mut dst = vec![0.0; 8];
        add_ramped(&mut dst, &src, 0.0, 1.0);
        assert_eq!(dst, vec![0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1.0, 1.0]);
    }
}
