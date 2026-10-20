//! Track requests waiting for the host (TASK-215): ids, lifecycle, and per-device limits.
//!
//! A phone may have at most [`MAX_PENDING_REQUESTS`] unanswered requests and may send them no
//! faster than a small token bucket allows; searches have their own bucket. Requests are only
//! ever *offered* to the host, which accepts or rejects them.

#![forbid(unsafe_code)]

use crate::devices::DeviceId;
use crate::reference::ValidReference;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Unanswered requests one device may have at a time.
pub const MAX_PENDING_REQUESTS: usize = 5;
/// Requests a device may send in a burst before it has to wait.
const REQUEST_BURST: f64 = 8.0;
/// One more request is allowed this often after the burst.
const REQUEST_REFILL: Duration = Duration::from_secs(5);
/// Searches a device may send in a burst; the host searches providers on its behalf.
const SEARCH_BURST: f64 = 4.0;
const SEARCH_REFILL: Duration = Duration::from_secs(2);
/// Answered requests remembered so a later "playing" update still finds its phone.
const MAX_REMEMBERED: usize = 1024;

/// Host-wide request number, unique while the daemon runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RequestId(pub u64);

/// Where a request stands, as shown on the phone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestStatus {
    Pending,
    Accepted,
    Rejected,
    Playing,
}

/// A validated track request handed to the host for moderation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackRequest {
    pub id: RequestId,
    pub device: DeviceId,
    pub device_name: String,
    /// The host chose to accept this device's requests without asking.
    pub trusted: bool,
    pub reference: ValidReference,
}

/// Something a phone asks of the host that the host must answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostRequest {
    /// Play something: approve or reject it.
    Track(TrackRequest),
    /// Search the library and providers; answer with `RemoteDaemon::send_search_results`.
    Search { device: DeviceId, query: String },
}

#[derive(Debug, Clone, Copy)]
struct Bucket {
    tokens: f64,
    at: Instant,
}

impl Bucket {
    fn take(slot: &mut Option<Self>, burst: f64, refill: Duration, now: Instant) -> bool {
        let b = slot.get_or_insert(Self {
            tokens: burst,
            at: now,
        });
        let earned = now.saturating_duration_since(b.at).as_secs_f64() / refill.as_secs_f64();
        b.tokens = (b.tokens + earned).min(burst);
        b.at = now;
        if b.tokens >= 1.0 {
            b.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Default)]
struct DeviceLimits {
    requests: Option<Bucket>,
    searches: Option<Bucket>,
}

/// Open and recently answered requests plus per-device rate limits.
#[derive(Debug, Default)]
pub struct RequestBook {
    next_id: u64,
    owners: HashMap<RequestId, (DeviceId, RequestStatus)>,
    order: VecDeque<RequestId>,
    limits: HashMap<DeviceId, DeviceLimits>,
}

impl RequestBook {
    /// Opens a request for `device`, or says why it may not send one now.
    pub fn open(&mut self, device: &DeviceId, now: Instant) -> Result<RequestId, &'static str> {
        let pending = self
            .owners
            .values()
            .filter(|(d, s)| d == device && *s == RequestStatus::Pending)
            .count();
        if pending >= MAX_PENDING_REQUESTS {
            return Err("You already have the most requests waiting. Wait for the host to answer.");
        }
        let limits = self.limits.entry(device.clone()).or_default();
        if !Bucket::take(&mut limits.requests, REQUEST_BURST, REQUEST_REFILL, now) {
            return Err("Too many requests. Try again in a few seconds.");
        }
        self.next_id += 1;
        let id = RequestId(self.next_id);
        self.owners
            .insert(id, (device.clone(), RequestStatus::Pending));
        self.order.push_back(id);
        while self.order.len() > MAX_REMEMBERED {
            if let Some(old) = self.order.pop_front() {
                self.owners.remove(&old);
            }
        }
        Ok(id)
    }

    /// Allows one search for `device`, or says why not.
    pub fn allow_search(&mut self, device: &DeviceId, now: Instant) -> Result<(), &'static str> {
        let limits = self.limits.entry(device.clone()).or_default();
        if Bucket::take(&mut limits.searches, SEARCH_BURST, SEARCH_REFILL, now) {
            Ok(())
        } else {
            Err("Too many searches. Try again in a few seconds.")
        }
    }

    /// Records the host's answer; returns the phone to tell, if the request is known.
    pub fn update(&mut self, id: RequestId, status: RequestStatus) -> Option<DeviceId> {
        let entry = self.owners.get_mut(&id)?;
        entry.1 = status;
        Some(entry.0.clone())
    }

    /// Drops everything about a revoked device.
    pub fn forget_device(&mut self, device: &DeviceId) {
        self.owners.retain(|_, (d, _)| d != device);
        self.order.retain(|id| self.owners.contains_key(id));
        self.limits.remove(device);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(s: &str) -> DeviceId {
        DeviceId(s.into())
    }

    #[test]
    fn pending_cap_is_per_device_and_freed_by_answers() {
        let mut book = RequestBook::default();
        let now = Instant::now();
        let ids: Vec<_> = (0..MAX_PENDING_REQUESTS)
            .map(|_| book.open(&dev("a"), now).unwrap())
            .collect();
        assert!(book.open(&dev("a"), now).is_err());
        assert!(book.open(&dev("b"), now).is_ok());
        assert_eq!(book.update(ids[0], RequestStatus::Accepted), Some(dev("a")));
        assert!(book.open(&dev("a"), now).is_ok());
    }

    #[test]
    fn bucket_refills_over_time() {
        let mut book = RequestBook::default();
        let now = Instant::now();
        for i in 0..REQUEST_BURST as usize {
            let id = book.open(&dev("a"), now).unwrap();
            if i % 2 == 0 {
                book.update(id, RequestStatus::Rejected);
            } else {
                book.update(id, RequestStatus::Accepted);
            }
        }
        assert!(book.open(&dev("a"), now).is_err());
        assert!(book.open(&dev("a"), now + REQUEST_REFILL).is_ok());
    }

    #[test]
    fn searches_are_rate_limited() {
        let mut book = RequestBook::default();
        let now = Instant::now();
        for _ in 0..SEARCH_BURST as usize {
            assert!(book.allow_search(&dev("a"), now).is_ok());
        }
        assert!(book.allow_search(&dev("a"), now).is_err());
        assert!(book.allow_search(&dev("a"), now + SEARCH_REFILL).is_ok());
    }
}
