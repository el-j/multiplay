//! Four decks on the console (TASK-220): the setting shows C and D, hiding them stops them, and
//! the deck on air follows each deck's crossfader assignment.

use amp_adapter_gui::create_headless_engine;
use amp_adapter_gui::dj_session::DjSession;
use amp_core_engine::dj::engine::{CrossfaderSide, CROSSFADER_RIGHT};

#[test]
fn dj_four_deck_setting_shows_and_hides_decks_c_and_d() {
    let driver = create_headless_engine().expect("driver");
    let mut dj = DjSession::new(&driver, None);
    assert_eq!(dj.deck_count(), 2, "two decks unless chosen");
    dj.set_deck_count(4);
    assert_eq!(dj.deck_count(), 4);
    dj.params().deck(3).set_playing(true);
    dj.set_deck_count(2);
    assert!(!dj.params().deck(3).playing(), "a hidden deck stops");
    dj.set_deck_count(3);
    assert_eq!(dj.deck_count(), 2, "only two or four");
}

#[test]
fn dj_four_deck_on_air_follows_the_crossfader_assignment() {
    let driver = create_headless_engine().expect("driver");
    let mut dj = DjSession::new(&driver, None);
    dj.set_deck_count(4);
    let p = dj.params();
    p.set_crossfader(CROSSFADER_RIGHT);
    for d in 0..4 {
        p.deck(d).set_fader(if d == 2 { 1.0 } else { 0.5 });
    }
    p.deck(0).set_playing(true);
    p.deck(2).set_playing(true);
    p.deck(2).set_side(CrossfaderSide::Left);
    assert_eq!(
        dj.on_air_deck(),
        0,
        "C is on the silent side; A wins the tie at zero"
    );
    p.deck(2).set_side(CrossfaderSide::Thru);
    assert_eq!(dj.on_air_deck(), 2, "C through the mixer is loudest");
}
