//! TASK-212: no single control stops the music by accident, and held controls let go cleanly.

use amp_adapter_gui::audio_driver::AudioPlaybackDriver;
use amp_adapter_gui::dj_controls::{press, release, ConsoleState, DjAction};
use amp_adapter_gui::dj_session::DjSession;
use std::time::{Duration, Instant};

fn session() -> (AudioPlaybackDriver, DjSession) {
    let driver = AudioPlaybackDriver::new_headless().expect("driver");
    let dj = DjSession::new(&driver, None);
    let p = dj.params();
    p.set_crossfader(-1.0); // deck A on air, deck B silent on master
    p.deck(0).set_playing(true);
    p.deck(1).set_playing(true);
    (driver, dj)
}

#[test]
fn dj_pausing_the_deck_on_air_needs_a_second_press() {
    let (_driver, mut dj) = session();
    let mut st = ConsoleState::default();
    let t = Instant::now();
    press(&mut dj, &mut st, DjAction::PlayPause(0), false, t);
    assert!(
        dj.params().deck(0).playing(),
        "one press does not stop the music"
    );
    assert!(st.message.is_some(), "the user is told to press again");
    press(
        &mut dj,
        &mut st,
        DjAction::PlayPause(0),
        false,
        t + Duration::from_millis(400),
    );
    assert!(!dj.params().deck(0).playing(), "the second press stops it");
}

#[test]
fn dj_a_late_second_press_starts_over() {
    let (_driver, mut dj) = session();
    let mut st = ConsoleState::default();
    let t = Instant::now();
    press(&mut dj, &mut st, DjAction::PlayPause(0), false, t);
    press(
        &mut dj,
        &mut st,
        DjAction::PlayPause(0),
        false,
        t + Duration::from_secs(3),
    );
    assert!(dj.params().deck(0).playing());
}

#[test]
fn dj_off_air_deck_pauses_at_once_and_cue_never_stops_the_deck_on_air() {
    let (_driver, mut dj) = session();
    let mut st = ConsoleState::default();
    let t = Instant::now();
    press(&mut dj, &mut st, DjAction::PlayPause(1), false, t);
    assert!(
        !dj.params().deck(1).playing(),
        "deck B is not heard: pauses immediately"
    );
    press(&mut dj, &mut st, DjAction::Cue(0), false, t);
    assert!(
        dj.params().deck(0).playing(),
        "CUE is ignored on the deck on air"
    );
}

#[test]
fn dj_nudge_and_cue_preview_end_when_released() {
    let (_driver, mut dj) = session();
    let mut st = ConsoleState::default();
    let t = Instant::now();
    press(&mut dj, &mut st, DjAction::NudgeForward(1), false, t);
    assert!(dj.params().deck(1).nudge() > 0.0);
    release(&mut dj, &mut st, DjAction::NudgeForward(1));
    assert_eq!(dj.params().deck(1).nudge(), 0.0);

    dj.params().deck(1).set_playing(false);
    st.cue_point[1] = 0.0; // paused on the cue point: holding CUE previews
    press(&mut dj, &mut st, DjAction::Cue(1), false, t);
    assert!(dj.params().deck(1).playing());
    release(&mut dj, &mut st, DjAction::Cue(1));
    assert!(
        !dj.params().deck(1).playing(),
        "letting go returns to the cue point"
    );
}
