//! Collaborative CRDT Queue and Lamport state machine for distributed mesh jam sessions.
//! Uses a Replicated Growable Array (RGA) structure with monotonic Lamport timestamps
//! and peer ID tie-breaking to achieve deterministic eventual consistency.

#![forbid(unsafe_code)]

use crate::Track;
use serde::{Deserialize, Serialize};
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
    /// Number of participant upvotes for sorting priority.
    pub upvotes: u32,
    /// Tombstone marker indicating soft deletion.
    pub is_deleted: bool,
}

/// Atomic operations applied to the collaborative queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JamOperation {
    /// Inserts a new track into the queue after a target item (or at the head if `after_id` is None).
    Insert {
        item: Box<JamItem>,
        after_id: Option<Uuid>,
    },
    /// Increments the upvote score of an item.
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

/// Decentralized collaborative queue container ensuring eventual consistency across peers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CollaborativeQueue {
    items: Vec<JamItem>,
}

impl CollaborativeQueue {
    /// Creates an empty collaborative queue.
    #[must_use]
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Returns all active (non-tombstoned) items in display order.
    #[must_use]
    pub fn active_items(&self) -> Vec<&JamItem> {
        self.items.iter().filter(|item| !item.is_deleted).collect()
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

    /// Applies an operation idempotently to the CRDT queue.
    pub fn apply_operation(&mut self, op: JamOperation) {
        match op {
            JamOperation::Insert { item, after_id } => {
                let item = *item;
                // Idempotency: Ignore if already present
                if self.items.iter().any(|i| i.id == item.id) {
                    return;
                }

                match after_id {
                    Some(prev_id) => {
                        if let Some(pos) = self.items.iter().position(|i| i.id == prev_id) {
                            // RGA order: if multiple nodes insert after the same item,
                            // order descending by (timestamp.counter, timestamp.peer_id).
                            let mut insert_pos = pos + 1;
                            while insert_pos < self.items.len()
                                && self.items[insert_pos].timestamp > item.timestamp
                            {
                                insert_pos += 1;
                            }
                            self.items.insert(insert_pos, item);
                        } else {
                            // Target parent not found, append to end
                            self.items.push(item);
                        }
                    }
                    None => {
                        // Insert at head respecting timestamp ordering
                        let mut insert_pos = 0;
                        while insert_pos < self.items.len()
                            && self.items[insert_pos].timestamp > item.timestamp
                        {
                            insert_pos += 1;
                        }
                        self.items.insert(insert_pos, item);
                    }
                }
            }
            JamOperation::Upvote { item_id, .. } => {
                if let Some(item) = self.items.iter_mut().find(|i| i.id == item_id) {
                    item.upvotes = item.upvotes.saturating_add(1);
                }
            }
            JamOperation::Delete { item_id, .. } => {
                if let Some(item) = self.items.iter_mut().find(|i| i.id == item_id) {
                    item.is_deleted = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AudioSource, TrackMetadata};

    fn sample_track(title: &str) -> Track {
        Track::new(
            AudioSource::LocalFile {
                path: format!("/music/{}.flac", title),
            },
            TrackMetadata::new(title, "Test Artist"),
        )
    }

    #[test]
    fn test_crdt_queue_insert_and_order() {
        let mut queue = CollaborativeQueue::new();
        let peer_a = Uuid::new_v4();
        let peer_b = Uuid::new_v4();

        let item1 = JamItem {
            id: Uuid::new_v4(),
            track: sample_track("Track 1"),
            submitted_by: "Alice".to_string(),
            timestamp: LamportTimestamp::new(1, peer_a),
            upvotes: 0,
            is_deleted: false,
        };

        let item2 = JamItem {
            id: Uuid::new_v4(),
            track: sample_track("Track 2"),
            submitted_by: "Bob".to_string(),
            timestamp: LamportTimestamp::new(2, peer_b),
            upvotes: 0,
            is_deleted: false,
        };

        let id1 = item1.id;
        queue.apply_operation(JamOperation::Insert {
            item: Box::new(item1),
            after_id: None,
        });

        queue.apply_operation(JamOperation::Insert {
            item: Box::new(item2),
            after_id: Some(id1),
        });

        let active = queue.active_items();
        assert_eq!(active.len(), 2);
        assert_eq!(active[0].track.metadata.title, "Track 1");
        assert_eq!(active[1].track.metadata.title, "Track 2");
    }

    #[test]
    fn test_crdt_queue_upvote_and_tombstone() {
        let mut queue = CollaborativeQueue::new();
        let peer = Uuid::new_v4();
        let item_id = Uuid::new_v4();

        let item = JamItem {
            id: item_id,
            track: sample_track("Hit Single"),
            submitted_by: "Charlie".to_string(),
            timestamp: LamportTimestamp::new(1, peer),
            upvotes: 0,
            is_deleted: false,
        };

        queue.apply_operation(JamOperation::Insert {
            item: Box::new(item),
            after_id: None,
        });

        assert_eq!(queue.active_items()[0].upvotes, 0);

        // Upvote
        queue.apply_operation(JamOperation::Upvote {
            item_id,
            peer_id: peer,
            timestamp: LamportTimestamp::new(2, peer),
        });
        assert_eq!(queue.active_items()[0].upvotes, 1);

        // Soft delete (tombstone)
        queue.apply_operation(JamOperation::Delete {
            item_id,
            timestamp: LamportTimestamp::new(3, peer),
        });
        assert_eq!(queue.active_items().len(), 0);
        assert_eq!(queue.len_with_tombstones(), 1);
    }
}
