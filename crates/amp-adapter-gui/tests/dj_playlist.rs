//! TASK-210: one playlist with deck assignments feeds each deck its tracks in order, and the
//! assignments are kept with the session.

use amp_adapter_gui::dj_session::PlaylistDeckSource;
use amp_adapter_gui::MultiplayGuiApp;
use amp_core_engine::dj::automix::AutoMixSource;
use amp_domain::{AudioSource, DeckAssignment, Track, TrackMetadata};
use std::collections::{HashMap, HashSet};

fn track(title: &str) -> Track {
    Track::new(
        AudioSource::LocalFile {
            path: format!("/nowhere/{title}.flac"),
        },
        TrackMetadata::new(title, "Test"),
    )
}

#[test]
fn dj_playlist_feeds_each_deck_its_assigned_tracks_in_order() {
    use DeckAssignment::{Both, None as Off, A, B};
    let playlist: Vec<Track> = ["one", "two", "three", "four", "five", "six"]
        .into_iter()
        .map(track)
        .collect();
    let assign: HashMap<_, _> = playlist
        .iter()
        .map(|t| t.id)
        .zip([A, B, Off, Both, A, B])
        .collect();
    let mut played = HashSet::new();
    let mut source = PlaylistDeckSource::new(&playlist, &assign, &mut played, Vec::new(), None);
    let mut take = |deck| source.next_for(deck).map(|t| t.title);
    assert_eq!(take(0).as_deref(), Some("one"));
    assert_eq!(take(1).as_deref(), Some("two"));
    assert_eq!(
        take(1).as_deref(),
        Some("four"),
        "an A+B row goes to whichever deck asks"
    );
    assert_eq!(
        take(0).as_deref(),
        Some("five"),
        "the unassigned row is skipped"
    );
    assert_eq!(take(1).as_deref(), Some("six"));
    assert_eq!(take(0), None);
    assert_eq!(played.len(), 5);
}

#[test]
fn dj_playlist_assignments_survive_a_restart() {
    let root = std::env::temp_dir().join(format!("multiplay-assign-{}", uuid::Uuid::new_v4()));
    let mut app = MultiplayGuiApp::headless(&root).expect("app");
    app.playlist = vec![track("a"), track("b")];
    app.deck_assign
        .insert(app.playlist[1].id, DeckAssignment::Both);
    app.save_session();
    let back = MultiplayGuiApp::headless(&root).expect("reload");
    assert_eq!(
        back.deck_assign.get(&back.playlist[1].id),
        Some(&DeckAssignment::Both)
    );
    let _ = std::fs::remove_dir_all(root);
}
