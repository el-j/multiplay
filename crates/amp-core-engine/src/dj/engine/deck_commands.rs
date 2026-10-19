//! Loop and cue actions, applied on the render thread where the exact playhead is known.
//! With quantize on and a beat grid, loop points and cues land on the nearest beat and cue
//! jumps keep the beat phase.

#![forbid(unsafe_code)]

use super::loops::HOT_CUES;
use super::params::DeckParams;
use super::player::DeckPlayer;

/// A loop or cue action for one deck.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeckCommand {
    /// Loop between two track times in seconds (not quantized).
    LoopBetween(f64, f64),
    /// Loop `beats` beats from the (quantized) playhead; with `roll`, the track keeps running
    /// underneath and playback resumes there when the loop ends.
    BeatLoop {
        /// Loop length in beats (1/4 … 32).
        beats: f64,
        /// Loop roll instead of a normal loop.
        roll: bool,
    },
    /// First press sets loop-in (A), second press sets loop-out (B) and starts the loop.
    Ab,
    /// Leaves the loop (keeping it for reloop).
    ExitLoop,
    /// Turns the last loop back on and jumps to its start.
    Reloop,
    /// Multiplies the loop length (0.5 halves, 2 doubles).
    ResizeLoop(f64),
    /// Sets or clears a hot cue (seconds).
    SetHotCue(usize, Option<f64>),
    /// Stores the (quantized) playhead in a hot cue slot.
    MarkHotCue(usize),
    /// Jumps to a hot cue.
    JumpHotCue(usize),
}

/// Applies `cmd` to `deck`.
pub(crate) fn apply(deck: &mut DeckPlayer, p: &DeckParams, cmd: DeckCommand) {
    let rate = f64::from(deck.sample_rate());
    if rate <= 0.0 {
        if let DeckCommand::SetHotCue(slot, v) = cmd {
            set_cue(deck, slot, v);
        }
        return;
    }
    let grid = p.grid();
    let snap = |secs: f64| match (p.quantize(), grid) {
        (true, Some((bpm, anchor))) => {
            let period = 60.0 / bpm;
            anchor + ((secs - anchor) / period).round() * period
        }
        _ => secs,
    };
    let now = deck.position / rate;
    let l = &mut deck.loops;
    match cmd {
        DeckCommand::LoopBetween(a, b) => {
            l.shadow = None;
            l.set(a * rate, b * rate);
        }
        DeckCommand::BeatLoop { beats, roll } => {
            if let Some((bpm, _)) = grid {
                let start = snap(now);
                let len = beats.clamp(1.0 / 32.0, 64.0) * 60.0 / bpm;
                l.set(start * rate, (start + len) * rate);
                l.shadow = roll.then_some(deck.position);
            }
        }
        DeckCommand::Ab => match l.pending_a.take() {
            None => l.pending_a = Some(snap(now) * rate),
            Some(a) => {
                let b = snap(now) * rate;
                if b > a {
                    l.shadow = None;
                    l.set(a, b);
                } else {
                    l.pending_a = Some(a);
                }
            }
        },
        DeckCommand::ExitLoop => {
            l.active = false;
            if let Some(shadow) = l.shadow.take() {
                deck.jump_to_frame(shadow);
            }
        }
        DeckCommand::Reloop => {
            if let Some((s, _)) = l.range {
                l.active = true;
                deck.jump_to_frame(s);
            }
        }
        DeckCommand::ResizeLoop(factor) => {
            if let (Some((s, e)), true) = (l.range, factor.is_finite() && factor > 0.0) {
                let end = s + (e - s) * factor;
                if end - s >= 16.0 {
                    l.range = Some((s, end));
                }
            }
        }
        DeckCommand::SetHotCue(slot, v) => set_cue(deck, slot, v),
        DeckCommand::MarkHotCue(slot) => set_cue(deck, slot, Some(snap(now))),
        DeckCommand::JumpHotCue(slot) => {
            if let Some(cue) = deck.loops.cues.get(slot).copied().flatten() {
                let target = match (p.quantize(), grid) {
                    (true, Some(_)) => cue + (now - snap(now)),
                    _ => cue,
                };
                let frame = target.max(0.0) * rate;
                if let Some((s, e)) = deck.loops.active_range() {
                    if frame < s || frame >= e {
                        deck.loops.active = false;
                    }
                }
                deck.loops.shadow = None;
                deck.jump_to_frame(frame);
            }
        }
    }
}

fn set_cue(deck: &mut DeckPlayer, slot: usize, v: Option<f64>) {
    if slot < HOT_CUES {
        deck.loops.cues[slot] = v.filter(|s| s.is_finite() && *s >= 0.0);
    }
}
