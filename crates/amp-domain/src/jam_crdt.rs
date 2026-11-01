//! Collaborative jam queue (a Replicated Growable Array) for mesh jam sessions: every replica
//! that has seen the same operations shows the same queue, whatever order they arrived in and
//! however often (TASK-187).
//!
//! - Inserts are ordered after their parent by Lamport timestamp (newest first among siblings);
//!   an insert whose parent has not arrived yet waits for it instead of landing at the end.
//! - Votes are a set of voters per item, so a repeated vote counts once.
//! - Votes and deletes for items that have not arrived yet are kept and applied when they do.

#![forbid(unsafe_code)]

use crate::Track;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use uuid::Uuid;

/// Monotonically increasing Lamport timestamp with peer fingerprint tie-breaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LamportTimestamp {
    /// Logical clock counter.
    pub counter: u64,
    /// Unique identifier of the peer originating this timestamp.
    pub peer_id: Uuid,
}

impl LamportTimestamp {
    /// Creates a new Lamport timestamp.
    #[must_use]
    pub fn new(counter: u64, peer_id: Uuid) -> Self {
        Self { counter, peer_id }
    }

    /// Advances the clock given a remote timestamp according to Lamport's algorithm.
    pub fn advance(&mut self, remote: &LamportTimestamp) {
        self.counter = self.counter.max(remote.counter) + 1;
    }
}

/// An entry in the collaborative jam queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JamItem {
    /// Unique identifier for this queue entry.
    pub id: Uuid,
    /// Associated track metadata.
    pub track: Track,
    /// Name or ID of the peer who submitted the track.
    pub submitted_by: String,
    /// Lamport timestamp of item creation.
    pub timestamp: LamportTimestamp,
    /// Peers who voted for it.
    pub voters: BTreeSet<Uuid>,
    /// Tombstone marker indicating soft deletion.
    pub is_deleted: bool,
}

impl JamItem {
    /// A new, unvoted entry.
    #[must_use]
    pub fn new(id: Uuid, track: Track, submitted_by: String, timestamp: LamportTimestamp) -> Self {
        Self {
            id,
            track,
            submitted_by,
            timestamp,
            voters: BTreeSet::new(),
            is_deleted: false,
        }
    }

    /// Number of distinct peers who voted for it.
    #[must_use]
    pub fn upvotes(&self) -> usize {
        self.voters.len()
    }
}

/// Atomic operations applied to the collaborative queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JamOperation {
    /// Inserts a new track after a target item (or at the head if `after_id` is None).
    Insert {
        item: Box<JamItem>,
        after_id: Option<Uuid>,
    },
    /// `peer_id` votes for an item.
    Upvote {
        item_id: Uuid,
        peer_id: Uuid,
        timestamp: LamportTimestamp,
    },
    /// Soft-deletes an item by setting its tombstone.
    Delete {
        item_id: Uuid,
        timestamp: LamportTimestamp,
    },
}

impl JamOperation {
    fn counter(&self) -> u64 {
        match self {
            Self::Insert { item, .. } => item.timestamp.counter,
            Self::Upvote { timestamp, .. } | Self::Delete { timestamp, .. } => timestamp.counter,
        }
    }
}

/// Decentralized collaborative queue container ensuring eventual consistency across peers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CollaborativeQueue {
    items: Vec<JamItem>,
    /// Inserts waiting for their parent, by parent id.
    waiting_children: HashMap<Uuid, Vec<JamItem>>,
    waiting_votes: HashMap<Uuid, BTreeSet<Uuid>>,
    waiting_deletes: HashSet<Uuid>,
    /// Highest Lamport counter seen, so local timestamps follow everything already applied.
    max_counter: u64,
}

impl CollaborativeQueue {
    /// Creates an empty collaborative queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all active (non-tombstoned) items in display order.
    #[must_use]
    pub fn active_items(&self) -> Vec<&JamItem> {
        self.items.iter().filter(|item| !item.is_deleted).collect()
    }

    /// Ids of every integrated item, deleted ones included (valid `after_id`s).
    #[must_use]
    pub fn all_ids(&self) -> Vec<Uuid> {
        self.items.iter().map(|i| i.id).collect()
    }

    /// Returns the total count of items, including tombstones.
    #[must_use]
    pub fn len_with_tombstones(&self) -> usize {
        self.items.len()
    }

    /// Finds an item by its unique ID.
    #[must_use]
    pub fn find_item(&self, id: &Uuid) -> Option<&JamItem> {
        self.items.iter().find(|i| &i.id == id)
    }

    /// A timestamp for a new local operation: after `clock` and after everything applied here.
    pub fn next_timestamp(&self, clock: &mut LamportTimestamp, peer_id: Uuid) -> LamportTimestamp {
        clock.counter = clock.counter.max(self.max_counter) + 1;
        clock.peer_id = peer_id;
        *clock
    }

    fn known(&self, id: &Uuid) -> bool {
        self.items.iter().any(|i| &i.id == id)
            || self
                .waiting_children
                .values()
                .flatten()
                .any(|i| &i.id == id)
    }

    /// Applies an operation idempotently to the CRDT queue.
    pub fn apply_operation(&mut self, op: JamOperation) {
        self.max_counter = self.max_counter.max(op.counter());
        match op {
            JamOperation::Insert { item, after_id } => {
                if self.known(&item.id) {
                    return;
                }
                match after_id {
                    Some(parent) if !self.items.iter().any(|i| i.id == parent) => {
                        self.waiting_children.entry(parent).or_default().push(*item);
                    }
                    _ => self.integrate(*item, after_id),
                }
            }
            JamOperation::Upvote {
                item_id, peer_id, ..
            } => match self.items.iter_mut().find(|i| i.id == item_id) {
                Some(item) => {
                    item.voters.insert(peer_id);
                }
                None => {
                    self.waiting_votes
                        .entry(item_id)
                        .or_default()
                        .insert(peer_id);
                }
            },
            JamOperation::Delete { item_id, .. } => {
                match self.items.iter_mut().find(|i| i.id == item_id) {
                    Some(item) => item.is_deleted = true,
                    None => {
                        self.waiting_deletes.insert(item_id);
                    }
                }
            }
        }
    }

    /// Places `item` after `after` (RGA: skip newer siblings and their descendants), then
    /// whatever was waiting for it.
    fn integrate(&mut self, mut item: JamItem, after: Option<Uuid>) {
        if let Some(votes) = self.waiting_votes.remove(&item.id) {
            item.voters.extend(votes);
        }
        if self.waiting_deletes.remove(&item.id) {
            item.is_deleted = true;
        }
        let start = after
            .and_then(|p| self.items.iter().position(|i| i.id == p))
            .map_or(0, |p| p + 1);
        let mut pos = start;
        while pos < self.items.len() && self.items[pos].timestamp > item.timestamp {
            pos += 1;
        }
        let id = item.id;
        self.items.insert(pos, item);
        if let Some(children) = self.waiting_children.remove(&id) {
            for child in children {
                self.integrate(child, Some(id));
            }
        }
    }
}
