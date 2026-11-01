//! DJ controls (TASK-212): every console control as an action, the keyboard map, and the
//! rules that keep mixing safe and fun.
//!
//! - Buttons and keys trigger the same [`DjAction`]s, so every control is reachable both ways
//!   (a test checks that every action has a key).
//! - Nothing stops the music by accident: pausing the deck on air needs a second press within
//!   a second, and CUE (which jumps back and pauses) is ignored on the deck on air.
//! - CUE works like a CDJ: paused away from the cue point it sets the cue there; paused on it,
//!   holding CUE previews from there and letting go returns.

#![forbid(unsafe_code)]

use crate::dj_session::DjSession;
use eframe::egui::{Key, Modifiers};
use std::time::{Duration, Instant};

/// Beat-loop sizes the loop size buttons step through.
pub const LOOP_SIZES: [f64; 8] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0];
/// Second press needed within this time to pause the deck on air.
const CONFIRM_STOP: Duration = Duration::from_secs(1);
/// Pitch change per key press, in percent (Shift: ten times as much).
const PITCH_STEP: f32 = 0.1;
/// Tempo bend while a nudge key or button is held.
const NUDGE: f32 = 0.04;
/// Crossfader step per key press.
const XFADE_STEP: f32 = 0.1;

/// One DJ control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DjAction {
    /// Play or pause a deck (the deck on air needs a confirming second press).
    PlayPause(usize),
    /// CDJ-style cue: set / return / hold to preview.
    Cue(usize),
    /// Toggle tempo and phase sync.
    Sync(usize),
    /// Toggle prelisten (headphones).
    Pfl(usize),
    /// A-B loop: first press sets A, second sets B.
    LoopAb(usize),
    /// Loop the current beat-loop size from here.
    BeatLoop(usize),
    /// Halve the loop (or the beat-loop size when not looping).
    LoopHalve(usize),
    /// Double the loop (or the beat-loop size).
    LoopDouble(usize),
    /// Leave the loop.
    ExitLoop(usize),
    /// Jump to a hot cue, or set it when empty.
    HotCue(usize, usize),
    /// Clear a hot cue.
    ClearHotCue(usize, usize),
    /// Tempo down / up one step.
    PitchDown(usize),
    /// Tempo up one step.
    PitchUp(usize),
    /// Bend the tempo down while held.
    NudgeBack(usize),
    /// Bend the tempo up while held.
    NudgeForward(usize),
    /// Crossfader one step towards deck A.
    CrossfadeLeft,
    /// Crossfader one step towards deck B.
    CrossfadeRight,
    /// Crossfader to the centre.
    CrossfadeCentre,
    /// Headphones: more cue, less master.
    CueMixDown,
    /// Headphones: more master.
    CueMixUp,
    /// Headphone level down.
    CueVolumeDown,
    /// Headphone level up.
    CueVolumeUp,
    /// Auto mix on / off.
    AutoMix,
    /// Show or hide the keyboard cheat sheet.
    CheatSheet,
}

/// A key bound to an action.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Binding {
    /// The action.
    pub action: DjAction,
    /// The key.
    pub key: Key,
    /// Required modifiers.
    pub modifiers: Modifiers,
}

const fn b(action: DjAction, key: Key, modifiers: Modifiers) -> Binding {
    Binding {
        action,
        key,
        modifiers,
    }
}

const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::SHIFT;
const ALT: Modifiers = Modifiers::ALT;

/// Keys of one deck: play, cue, sync, PFL, A-B, beat loop, halve, double, exit loop,
/// pitch down/up, nudge back/forward.
const DECK_KEYS: [[Key; 13]; 2] = [
    [
        Key::A,
        Key::S,
        Key::D,
        Key::F,
        Key::E,
        Key::R,
        Key::Q,
        Key::W,
        Key::T,
        Key::Z,
        Key::X,
        Key::C,
        Key::V,
    ],
    [
        Key::J,
        Key::K,
        Key::L,
        Key::H,
        Key::U,
        Key::I,
        Key::Y,
        Key::O,
        Key::P,
        Key::N,
        Key::M,
        Key::Comma,
        Key::Period,
    ],
];
const DIGITS: [Key; 8] = [
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
];

/// The DJ keyboard map: deck A on the left hand, deck B on the right; hot cues on the number
/// row (Shift for deck B, Alt to clear).
pub fn keymap() -> Vec<Binding> {
    let mut map = Vec::new();
    for (deck, keys) in DECK_KEYS.iter().enumerate() {
        let per_deck = [
            DjAction::PlayPause(deck),
            DjAction::Cue(deck),
            DjAction::Sync(deck),
            DjAction::Pfl(deck),
            DjAction::LoopAb(deck),
            DjAction::BeatLoop(deck),
            DjAction::LoopHalve(deck),
            DjAction::LoopDouble(deck),
            DjAction::ExitLoop(deck),
            DjAction::PitchDown(deck),
            DjAction::PitchUp(deck),
            DjAction::NudgeBack(deck),
            DjAction::NudgeForward(deck),
        ];
        for (action, key) in per_deck.into_iter().zip(keys.iter()) {
            map.push(b(action, *key, NONE));
        }
        let shift = if deck == 0 { NONE } else { SHIFT };
        for (slot, key) in DIGITS.iter().enumerate() {
            map.push(b(DjAction::HotCue(deck, slot), *key, shift));
            map.push(b(DjAction::ClearHotCue(deck, slot), *key, ALT | shift));
        }
    }
    map.extend([
        b(DjAction::CrossfadeLeft, Key::ArrowLeft, NONE),
        b(DjAction::CrossfadeRight, Key::ArrowRight, NONE),
        b(DjAction::CrossfadeCentre, Key::ArrowDown, NONE),
        b(DjAction::CueMixDown, Key::Minus, NONE),
        b(DjAction::CueMixUp, Key::Equals, NONE),
        b(DjAction::CueVolumeDown, Key::Minus, SHIFT),
        b(DjAction::CueVolumeUp, Key::Equals, SHIFT),
        b(DjAction::AutoMix, Key::B, NONE),
        b(DjAction::CheatSheet, Key::Questionmark, NONE),
    ]);
    map
}

/// Every control on the console (for the reachability check and the cheat sheet).
pub fn all_actions() -> Vec<DjAction> {
    let mut v = Vec::new();
    for d in 0..2 {
        v.extend([
            DjAction::PlayPause(d),
            DjAction::Cue(d),
            DjAction::Sync(d),
            DjAction::Pfl(d),
            DjAction::LoopAb(d),
            DjAction::BeatLoop(d),
            DjAction::LoopHalve(d),
            DjAction::LoopDouble(d),
            DjAction::ExitLoop(d),
            DjAction::PitchDown(d),
            DjAction::PitchUp(d),
            DjAction::NudgeBack(d),
            DjAction::NudgeForward(d),
        ]);
        for s in 0..8 {
            v.extend([DjAction::HotCue(d, s), DjAction::ClearHotCue(d, s)]);
        }
    }
    v.extend([
        DjAction::CrossfadeLeft,
        DjAction::CrossfadeRight,
        DjAction::CrossfadeCentre,
        DjAction::CueMixDown,
        DjAction::CueMixUp,
        DjAction::CueVolumeDown,
        DjAction::CueVolumeUp,
        DjAction::AutoMix,
        DjAction::CheatSheet,
    ]);
    v
}

/// Short description for the cheat sheet.
pub fn describe(a: DjAction) -> String {
    let deck = |d: usize| crate::dj_strip::DECK_NAMES[d.min(3)];
    match a {
        DjAction::PlayPause(d) => format!("Play / pause {}", deck(d)),
        DjAction::Cue(d) => format!("Cue {}", deck(d)),
        DjAction::Sync(d) => format!("Sync {}", deck(d)),
        DjAction::Pfl(d) => format!("Prelisten {}", deck(d)),
        DjAction::LoopAb(d) => format!("Loop A-B {}", deck(d)),
        DjAction::BeatLoop(d) => format!("Beat loop {}", deck(d)),
        DjAction::LoopHalve(d) => format!("Loop ½ {}", deck(d)),
        DjAction::LoopDouble(d) => format!("Loop ×2 {}", deck(d)),
        DjAction::ExitLoop(d) => format!("Exit loop {}", deck(d)),
        DjAction::HotCue(d, s) => format!("Hot cue {} {}", s + 1, deck(d)),
        DjAction::ClearHotCue(d, s) => format!("Clear cue {} {}", s + 1, deck(d)),
        DjAction::PitchDown(d) => format!("Tempo − {}", deck(d)),
        DjAction::PitchUp(d) => format!("Tempo + {}", deck(d)),
        DjAction::NudgeBack(d) => format!("Nudge back {}", deck(d)),
        DjAction::NudgeForward(d) => format!("Nudge forward {}", deck(d)),
        DjAction::CrossfadeLeft => "Crossfader ◀".into(),
        DjAction::CrossfadeRight => "Crossfader ▶".into(),
        DjAction::CrossfadeCentre => "Crossfader centre".into(),
        DjAction::CueMixDown => "Headphones: more cue".into(),
        DjAction::CueMixUp => "Headphones: more master".into(),
        DjAction::CueVolumeDown => "Headphones quieter".into(),
        DjAction::CueVolumeUp => "Headphones louder".into(),
        DjAction::AutoMix => "Auto mix".into(),
        DjAction::CheatSheet => "This cheat sheet".into(),
    }
}

/// Console state that lives outside the engine.
#[derive(Debug, Clone)]
pub struct ConsoleState {
    /// Beat-loop size per deck, as an index into [`LOOP_SIZES`].
    pub loop_size: [usize; 4],
    /// CUE point per deck, in seconds.
    pub cue_point: [f64; 4],
    /// Whether CUE is currently previewing (held) per deck.
    pub previewing: [bool; 4],
    stop_armed: [Option<Instant>; 4],
    /// Cheat sheet shown.
    pub cheat_sheet: bool,
    /// Actions drawn as on-screen controls this frame (for the reachability check).
    pub rendered: Vec<DjAction>,
    /// Held on-screen controls (nudge, cue preview).
    pub held: Vec<DjAction>,
    /// Last feedback for the user ("press again to stop").
    pub message: Option<(String, Instant)>,
}

impl Default for ConsoleState {
    fn default() -> Self {
        Self {
            loop_size: [4; 4],
            cue_point: [0.0; 4],
            previewing: [false; 4],
            stop_armed: [None; 4],
            cheat_sheet: false,
            rendered: Vec::new(),
            held: Vec::new(),
            message: None,
        }
    }
}

impl ConsoleState {
    fn say(&mut self, text: &str, now: Instant) {
        self.message = Some((text.to_string(), now));
    }
}

/// Applies a pressed action. Returns true when auto mix should be toggled (the caller owns the
/// playlist source it needs).
pub fn press(
    dj: &mut DjSession,
    st: &mut ConsoleState,
    a: DjAction,
    shift: bool,
    now: Instant,
) -> bool {
    let on_air = |dj: &DjSession, d: usize| {
        dj.params().deck(d).playing() && dj.on_air_deck() == d && audible(dj, d)
    };
    let p = dj.control().params_handle();
    match a {
        DjAction::PlayPause(d) => {
            let deck = p.deck(d);
            if !deck.playing() {
                deck.set_playing(true);
            } else if !on_air(dj, d) || st.stop_armed[d].is_some_and(|t| now - t < CONFIRM_STOP) {
                deck.set_playing(false);
                st.stop_armed[d] = None;
            } else {
                st.stop_armed[d] = Some(now);
                st.say("Deck on air — press again to stop it", now);
            }
        }
        DjAction::Cue(d) => {
            if on_air(dj, d) {
                st.say("Cue is off on the deck on air", now);
            } else if p.deck(d).playing() {
                p.deck(d).set_playing(false);
                dj.control().seek(d, st.cue_point[d]);
            } else {
                let pos = dj.status().deck(d).position_secs();
                if (pos - st.cue_point[d]).abs() < 0.01 {
                    // Held on the cue point: preview from it (released in `release`).
                    st.previewing[d] = true;
                    p.deck(d).set_playing(true);
                } else {
                    st.cue_point[d] = quantized(dj, d, pos);
                    dj.control().seek(d, st.cue_point[d]);
                }
            }
        }
        DjAction::Sync(d) => p.deck(d).set_sync(!p.deck(d).sync()),
        DjAction::Pfl(d) => p.deck(d).set_pfl(!p.deck(d).pfl()),
        DjAction::LoopAb(d) => {
            dj.control().ab(d);
        }
        DjAction::BeatLoop(d) => {
            let beats = LOOP_SIZES[st.loop_size[d]];
            dj.control().beat_loop(d, beats, false);
        }
        DjAction::LoopHalve(d) | DjAction::LoopDouble(d) => {
            let up = matches!(a, DjAction::LoopDouble(_));
            if dj.status().deck(d).loop_active() {
                dj.control().resize_loop(d, if up { 2.0 } else { 0.5 });
            }
            st.loop_size[d] = if up {
                (st.loop_size[d] + 1).min(LOOP_SIZES.len() - 1)
            } else {
                st.loop_size[d].saturating_sub(1)
            };
        }
        DjAction::ExitLoop(d) => {
            dj.control().exit_loop(d);
        }
        DjAction::HotCue(d, s) => {
            if dj.status().deck(d).hot_cue(s).is_some() {
                dj.control().jump_hot_cue(d, s);
            } else {
                dj.mark_hot_cue(d, s);
            }
        }
        DjAction::ClearHotCue(d, s) => dj.clear_hot_cue(d, s),
        DjAction::PitchDown(d) | DjAction::PitchUp(d) => {
            let step = if shift { PITCH_STEP * 10.0 } else { PITCH_STEP };
            let sign = if matches!(a, DjAction::PitchUp(_)) {
                1.0
            } else {
                -1.0
            };
            let deck = p.deck(d);
            deck.set_pitch_percent(deck.pitch_percent() + sign * step);
        }
        DjAction::NudgeBack(d) => p.deck(d).set_nudge(-NUDGE),
        DjAction::NudgeForward(d) => p.deck(d).set_nudge(NUDGE),
        DjAction::CrossfadeLeft => p.set_crossfader(p.crossfader() - XFADE_STEP),
        DjAction::CrossfadeRight => p.set_crossfader(p.crossfader() + XFADE_STEP),
        DjAction::CrossfadeCentre => p.set_crossfader(0.0),
        DjAction::CueMixDown | DjAction::CueMixUp => {
            let m = p.cue_mix().mix
                + if matches!(a, DjAction::CueMixUp) {
                    0.1
                } else {
                    -0.1
                };
            p.set_cue_mix(m);
        }
        DjAction::CueVolumeDown | DjAction::CueVolumeUp => {
            let v = p.cue_mix().volume
                + if matches!(a, DjAction::CueVolumeUp) {
                    0.1
                } else {
                    -0.1
                };
            p.set_cue_volume(v);
        }
        DjAction::AutoMix => return true,
        DjAction::CheatSheet => st.cheat_sheet = !st.cheat_sheet,
    }
    false
}

/// Applies the release of a held control (nudge, cue preview).
pub fn release(dj: &mut DjSession, st: &mut ConsoleState, a: DjAction) {
    match a {
        DjAction::NudgeBack(d) | DjAction::NudgeForward(d) => dj.params().deck(d).set_nudge(0.0),
        DjAction::Cue(d) if st.previewing[d] => {
            st.previewing[d] = false;
            dj.params().deck(d).set_playing(false);
            dj.control().seek(d, st.cue_point[d]);
        }
        _ => {}
    }
}

/// True when deck `d` is heard on the master output.
fn audible(dj: &DjSession, d: usize) -> bool {
    let p = dj.params();
    let (l, r) = amp_core_engine::dj::engine::curve_gains(p.master().curve(), p.crossfader());
    let side = if d == 0 { l } else { r };
    p.deck(d).fader() * side > 0.05
}

/// `secs` on the beat grid when quantize is on.
fn quantized(dj: &DjSession, d: usize, secs: f64) -> f64 {
    let p = dj.params().deck(d);
    match (p.quantize(), dj.deck(d).and_then(|s| s.analysis())) {
        (true, Some(a)) => a.nearest_beat(secs).unwrap_or(secs),
        _ => secs,
    }
}

/// Short key label for the cheat sheet.
pub fn key_label(bind: &Binding) -> String {
    let mut s = String::new();
    if bind.modifiers.alt {
        s.push_str("Alt+");
    }
    if bind.modifiers.shift {
        s.push_str("Shift+");
    }
    s.push_str(bind.key.symbol_or_name());
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn dj_every_control_has_a_key() {
        let map = keymap();
        for a in all_actions() {
            assert!(map.iter().any(|b| b.action == a), "{a:?} has no key");
        }
    }

    #[test]
    fn dj_no_key_does_two_things() {
        let mut seen = HashSet::new();
        for b in keymap() {
            let k = (b.key, b.modifiers.alt, b.modifiers.shift);
            assert!(seen.insert(k), "{:?} is bound twice", b.key);
        }
    }

    #[test]
    fn dj_keys_leave_app_shortcuts_alone() {
        // Ctrl/Cmd shortcuts (J, D, P) and the playlist's [ ] \\ stay free.
        for b in keymap() {
            assert!(!b.modifiers.command && !b.modifiers.ctrl, "{:?}", b.key);
            assert!(
                ![Key::OpenBracket, Key::CloseBracket, Key::Backslash].contains(&b.key),
                "{:?} is the playlist's",
                b.key
            );
        }
    }
}
