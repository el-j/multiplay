//! The jam queue converges (TASK-187): whatever order operations arrive in, however often, every
//! replica ends up with the same queue and the same votes.

#![forbid(unsafe_code)]

use amp_domain::jam_crdt::{CollaborativeQueue, JamItem, JamOperation, LamportTimestamp};
use amp_domain::{AudioSource, Track, TrackMetadata};
use proptest::prelude::*;
use uuid::Uuid;

fn track(n: usize) -> Track {
    Track::new(
        AudioSource::LocalFile {
            path: format!("/m/{n}.flac"),
        },
        TrackMetadata::new(format!("Song {n}"), "Artist"),
    )
}

/// What a replica shows: ids in order, with their votes.
fn view(q: &CollaborativeQueue) -> Vec<(Uuid, usize)> {
    q.active_items()
        .iter()
        .map(|i| (i.id, i.upvotes()))
        .collect()
}

#[derive(Debug, Clone)]
enum Action {
    Insert { peer: usize, after: Option<usize> },
    Vote { peer: usize, item: usize },
    Delete { peer: usize, item: usize },
}

fn action() -> impl Strategy<Value = Action> {
    prop_oneof![
        3 => (0..3usize, proptest::option::of(0..64usize)).prop_map(|(peer, after)| Action::Insert { peer, after }),
        2 => (0..3usize, 0..64usize).prop_map(|(peer, item)| Action::Vote { peer, item }),
        1 => (0..3usize, 0..64usize).prop_map(|(peer, item)| Action::Delete { peer, item }),
    ]
}

/// Runs `actions` on three peers that each apply their own operations at once, and returns every
/// operation created (what the network would carry).
fn simulate(actions: &[Action]) -> Vec<JamOperation> {
    let peers: Vec<Uuid> = (0..3).map(|i| Uuid::from_u128(i + 1)).collect();
    let mut replicas: Vec<CollaborativeQueue> = (0..3).map(|_| CollaborativeQueue::new()).collect();
    let mut clocks = [LamportTimestamp::new(0, Uuid::nil()); 3];
    let mut ops = Vec::new();
    for (n, a) in actions.iter().enumerate() {
        let peer = match a {
            Action::Insert { peer, .. }
            | Action::Vote { peer, .. }
            | Action::Delete { peer, .. } => *peer,
        };
        // A peer has seen everything so far half of the time (gossip catches up), so some
        // operations are concurrent and some causal.
        if n % 2 == 0 {
            for op in &ops {
                replicas[peer].apply_operation(Clone::clone(op));
            }
        }
        let visible: Vec<Uuid> = replicas[peer].all_ids();
        let stamp = replicas[peer].next_timestamp(&mut clocks[peer], peers[peer]);
        let op = match a {
            Action::Insert { after, .. } => JamOperation::Insert {
                item: Box::new(JamItem::new(
                    Uuid::from_u128(1000 + n as u128),
                    track(n),
                    format!("peer{peer}"),
                    stamp,
                )),
                after_id: after.and_then(|i| visible.get(i % visible.len().max(1)).copied()),
            },
            Action::Vote { item, .. } => match visible.get(item % visible.len().max(1)) {
                Some(&item_id) => JamOperation::Upvote {
                    item_id,
                    peer_id: peers[peer],
                    timestamp: stamp,
                },
                None => continue,
            },
            Action::Delete { item, .. } => match visible.get(item % visible.len().max(1)) {
                Some(&item_id) => JamOperation::Delete {
                    item_id,
                    timestamp: stamp,
                },
                None => continue,
            },
        };
        replicas[peer].apply_operation(op.clone());
        ops.push(op);
    }
    ops
}

fn shuffled(ops: &[JamOperation], seed: u64, duplicate_every: usize) -> Vec<JamOperation> {
    let mut out: Vec<JamOperation> = ops.to_vec();
    for (i, op) in ops.iter().enumerate() {
        if duplicate_every > 0 && i % duplicate_every == 0 {
            out.push(op.clone());
        }
    }
    let mut s = seed | 1;
    for i in (1..out.len()).rev() {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        out.swap(i, (s % (i as u64 + 1)) as usize);
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(std::env::var("FUZZ_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(2_000)))]

    #[test]
    fn replicas_converge_in_any_order_with_duplicates(
        actions in proptest::collection::vec(action(), 1..40),
        seeds in proptest::collection::vec(any::<u64>(), 3),
    ) {
        let ops = simulate(&actions);
        let mut views = Vec::new();
        for (k, seed) in seeds.iter().enumerate() {
            let mut q = CollaborativeQueue::new();
            for op in shuffled(&ops, *seed, k + 1) {
                q.apply_operation(op);
            }
            views.push(view(&q));
        }
        // In order of creation, without shuffling, as the reference.
        let mut reference = CollaborativeQueue::new();
        for op in ops.iter().cloned() {
            reference.apply_operation(op);
        }
        for v in &views {
            prop_assert_eq!(v, &view(&reference));
        }
    }
}

#[test]
fn a_vote_counts_once_however_often_it_arrives() {
    let mut q = CollaborativeQueue::new();
    let peer = Uuid::new_v4();
    let item = JamItem::new(
        Uuid::new_v4(),
        track(1),
        "a".into(),
        LamportTimestamp::new(1, peer),
    );
    let id = item.id;
    let vote = JamOperation::Upvote {
        item_id: id,
        peer_id: peer,
        timestamp: LamportTimestamp::new(2, peer),
    };
    // The vote arrives before the item, then again after it.
    q.apply_operation(vote.clone());
    q.apply_operation(JamOperation::Insert {
        item: Box::new(item),
        after_id: None,
    });
    q.apply_operation(vote);
    assert_eq!(q.active_items()[0].upvotes(), 1);
}

#[test]
fn a_delete_that_arrives_first_still_deletes() {
    let mut q = CollaborativeQueue::new();
    let peer = Uuid::new_v4();
    let item = JamItem::new(
        Uuid::new_v4(),
        track(1),
        "a".into(),
        LamportTimestamp::new(1, peer),
    );
    q.apply_operation(JamOperation::Delete {
        item_id: item.id,
        timestamp: LamportTimestamp::new(2, peer),
    });
    q.apply_operation(JamOperation::Insert {
        item: Box::new(item),
        after_id: None,
    });
    assert!(q.active_items().is_empty(), "no resurrection");
}
