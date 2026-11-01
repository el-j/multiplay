//! What the OS sends maps to player requests, and the OS hears about playback only when it
//! changes (TASK-173). No OS media session is needed for these.

#![forbid(unsafe_code)]

use amp_adapter_media_controls::{map_event, PlaybackSync};
use amp_ports::{MediaKeyEvent, MediaPlayback};
use souvlaki::{MediaControlEvent as E, MediaPosition, SeekDirection};
use std::time::Duration;

#[test]
fn os_events_map_to_player_requests() {
    for (os, ours) in [
        (E::Play, Some(MediaKeyEvent::Play)),
        (E::Pause, Some(MediaKeyEvent::Pause)),
        (E::Toggle, Some(MediaKeyEvent::Toggle)),
        (E::Stop, Some(MediaKeyEvent::Stop)),
        (E::Next, Some(MediaKeyEvent::Next)),
        (E::Previous, Some(MediaKeyEvent::Previous)),
        (
            E::Seek(SeekDirection::Forward),
            Some(MediaKeyEvent::SeekStep { forward: true }),
        ),
        (
            E::SeekBy(SeekDirection::Backward, Duration::from_secs(15)),
            Some(MediaKeyEvent::SeekBy(-15)),
        ),
        (
            E::SetPosition(MediaPosition(Duration::from_secs(42))),
            Some(MediaKeyEvent::SetPosition(Duration::from_secs(42))),
        ),
        // Not ours to act on: volume belongs to the system, quitting to the user.
        (E::SetVolume(0.3), None),
        (E::Quit, None),
    ] {
        assert_eq!(map_event(os.clone()), ours, "{os:?}");
    }
}

#[test]
fn playback_is_sent_on_change_and_on_seeks_only() {
    let mut sync = PlaybackSync::default();
    let at = |s: u64| Duration::from_secs(s);
    let t0 = std::time::Instant::now();
    assert!(sync.should_send(MediaPlayback::Playing { position: at(0) }, t0));
    // Steady playback: the OS extrapolates the position itself.
    assert!(!sync.should_send(MediaPlayback::Playing { position: at(5) }, t0 + at(5)));
    // A seek (position jumped against the clock): tell the OS.
    assert!(sync.should_send(MediaPlayback::Playing { position: at(60) }, t0 + at(6)));
    // Pause is a change.
    assert!(sync.should_send(MediaPlayback::Paused { position: at(61) }, t0 + at(7)));
    assert!(!sync.should_send(MediaPlayback::Paused { position: at(61) }, t0 + at(30)));
    assert!(sync.should_send(MediaPlayback::Stopped, t0 + at(31)));
}
