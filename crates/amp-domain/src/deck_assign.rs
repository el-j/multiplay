//! DJ deck assignment of playlist tracks (TASK-210): one playlist, each row assigned to deck
//! A, deck B, both, or neither. Each deck plays its tracks in playlist order; unassigned rows
//! are skipped in DJ mode.

#![forbid(unsafe_code)]

use crate::TrackId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Which deck(s) a playlist track belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum DeckAssignment {
    /// Not part of the DJ set.
    #[default]
    None,
    /// Deck A only.
    A,
    /// Deck B only.
    B,
    /// Either deck.
    Both,
}

impl DeckAssignment {
    /// Next value when the badge is clicked: none → A → B → both → none.
    pub fn cycle(self) -> Self {
        match self {
            Self::None => Self::A,
            Self::A => Self::B,
            Self::B => Self::Both,
            Self::Both => Self::None,
        }
    }

    /// True when `deck` (0 = A, 1 = B) may play the track.
    pub fn allows(self, deck: usize) -> bool {
        matches!(
            (self, deck),
            (Self::A, 0) | (Self::B, 1) | (Self::Both, 0) | (Self::Both, 1)
        )
    }

    /// Badge text.
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "·",
            Self::A => "A",
            Self::B => "B",
            Self::Both => "AB",
        }
    }
}

/// Index in `order` of the next track `deck` should play: the first track in playlist order that
/// the deck may play and that has not been played (or loaded) yet.
pub fn next_for_deck(
    order: &[TrackId],
    assign: &HashMap<TrackId, DeckAssignment>,
    played: &HashSet<TrackId>,
    deck: usize,
) -> Option<usize> {
    order.iter().position(|id| {
        !played.contains(id) && assign.get(id).copied().unwrap_or_default().allows(deck)
    })
}

/// The rows to mark "next on A" and "next on B". A track both decks could take is shown on A,
/// and B is shown its next track after that one.
pub fn next_markers(
    order: &[TrackId],
    assign: &HashMap<TrackId, DeckAssignment>,
    played: &HashSet<TrackId>,
) -> [Option<usize>; 2] {
    let a = next_for_deck(order, assign, played, 0);
    let mut taken = played.clone();
    if let Some(i) = a {
        taken.insert(order[i]);
    }
    [a, next_for_deck(order, assign, &taken, 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(assigns: &[DeckAssignment]) -> (Vec<TrackId>, HashMap<TrackId, DeckAssignment>) {
        let order: Vec<TrackId> = assigns.iter().map(|_| TrackId::new()).collect();
        let map = order.iter().copied().zip(assigns.iter().copied()).collect();
        (order, map)
    }

    use DeckAssignment::{Both, None as Off, A, B};

    #[test]
    fn each_deck_takes_its_tracks_in_playlist_order() {
        let (order, map) = set(&[B, A, Off, A, B]);
        let mut played = HashSet::new();
        assert_eq!(next_for_deck(&order, &map, &played, 0), Some(1));
        assert_eq!(next_for_deck(&order, &map, &played, 1), Some(0));
        played.insert(order[1]);
        assert_eq!(next_for_deck(&order, &map, &played, 0), Some(3));
        played.insert(order[3]);
        assert_eq!(
            next_for_deck(&order, &map, &played, 0),
            None,
            "unassigned row skipped"
        );
    }

    #[test]
    fn both_rows_go_to_whichever_deck_asks_first_and_only_once() {
        let (order, map) = set(&[Both, A, B]);
        let mut played = HashSet::new();
        assert_eq!(next_for_deck(&order, &map, &played, 1), Some(0));
        played.insert(order[0]);
        assert_eq!(next_for_deck(&order, &map, &played, 0), Some(1));
        assert_eq!(next_for_deck(&order, &map, &played, 1), Some(2));
    }

    #[test]
    fn markers_never_point_both_decks_at_the_same_row() {
        let (order, map) = set(&[Both, Both, A]);
        let played = HashSet::new();
        assert_eq!(next_markers(&order, &map, &played), [Some(0), Some(1)]);
    }

    #[test]
    fn removing_a_row_keeps_the_rest_in_order() {
        let (mut order, map) = set(&[A, A, A]);
        let second = order[1];
        order.remove(0);
        let next = next_for_deck(&order, &map, &HashSet::new(), 0).map(|i| order[i]);
        assert_eq!(next, Some(second));
    }

    #[test]
    fn badge_cycles_through_every_choice() {
        assert_eq!(Off.cycle().cycle().cycle().cycle(), Off);
        assert!(Both.allows(0) && Both.allows(1));
        assert!(!A.allows(1) && !B.allows(0) && !Off.allows(0));
    }
}
