//! Pairing tokens, persisted session secrets, and failed-attempt throttling.
//!
//! Pairing flow: the host shows a short-lived, single-use pairing token (QR code).
//! A client that presents it is granted a 128-bit session secret, which the host
//! stores only as a SHA-256 hash ([`crate::devices`]). The client reconnects with that
//! secret until the user revokes the device.

#![forbid(unsafe_code)]

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Failed attempts allowed per address before back-off starts.
pub const FREE_ATTEMPTS: u32 = 5;
/// Longest back-off applied to a misbehaving address.
pub const MAX_BACKOFF: Duration = Duration::from_secs(300);
const MAX_TRACKED_ADDRESSES: usize = 4096;
const MAX_PENDING_TOKENS: usize = 16;

/// Hex-encodes `n` bytes from the OS cryptographic RNG.
pub(crate) fn random_hex(n: usize) -> String {
    let mut buf = vec![0u8; n];
    getrandom::fill(&mut buf).expect("OS random number generator unavailable");
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn sha256_hex(input: &str) -> String {
    Sha256::digest(input.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub(crate) fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Short-lived, single-use pairing tokens (64 bits, shown as 16 hex characters).
#[derive(Debug, Default)]
pub struct PairingTokens {
    pending: HashMap<String, Instant>,
}

impl PairingTokens {
    /// Length of a pairing token in hex characters.
    pub const LEN: usize = 16;

    /// Issues a new token valid for `ttl`. Older tokens are dropped beyond a small cap.
    pub fn issue(&mut self, ttl: Duration) -> String {
        let now = Instant::now();
        self.pending.retain(|_, expiry| *expiry > now);
        if self.pending.len() >= MAX_PENDING_TOKENS {
            self.pending.clear();
        }
        let token = random_hex(Self::LEN / 2).to_uppercase();
        self.pending.insert(token.clone(), now + ttl);
        token
    }

    /// Consumes `token`; returns true only if it existed and had not expired.
    pub fn consume(&mut self, token: &str) -> bool {
        if !is_hex(token, Self::LEN) {
            return false;
        }
        match self.pending.remove(&token.to_uppercase()) {
            Some(expiry) => expiry > Instant::now(),
            None => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FailureRecord {
    failures: u32,
    blocked_until: Option<Instant>,
}

/// Per-address exponential back-off for failed authentication attempts.
#[derive(Debug, Default)]
pub struct FailureThrottle {
    records: HashMap<IpAddr, FailureRecord>,
}

impl FailureThrottle {
    /// Returns `Err(retry_after)` while `ip` is backed off.
    pub fn check(&self, ip: IpAddr, now: Instant) -> Result<(), Duration> {
        match self.records.get(&ip).and_then(|r| r.blocked_until) {
            Some(until) if until > now => Err(until - now),
            _ => Ok(()),
        }
    }

    /// Records a failed attempt; after [`FREE_ATTEMPTS`] each failure doubles the back-off.
    pub fn record_failure(&mut self, ip: IpAddr, now: Instant) {
        if self.records.len() >= MAX_TRACKED_ADDRESSES && !self.records.contains_key(&ip) {
            self.records
                .retain(|_, r| r.blocked_until.is_some_and(|until| until > now));
        }
        let rec = self.records.entry(ip).or_insert(FailureRecord {
            failures: 0,
            blocked_until: None,
        });
        rec.failures = rec.failures.saturating_add(1);
        if rec.failures >= FREE_ATTEMPTS {
            let exp = (rec.failures - FREE_ATTEMPTS).min(16);
            let backoff = Duration::from_secs(1u64 << exp).min(MAX_BACKOFF);
            rec.blocked_until = Some(now + backoff);
        }
    }

    /// Clears the failure history of `ip` after a successful authentication.
    pub fn record_success(&mut self, ip: IpAddr) {
        self.records.remove(&ip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairing_tokens_are_single_use_and_expire() {
        let mut tokens = PairingTokens::default();
        let t = tokens.issue(Duration::from_secs(60));
        assert_eq!(t.len(), PairingTokens::LEN);
        assert!(tokens.consume(&t.to_lowercase()));
        assert!(!tokens.consume(&t));

        let expired = tokens.issue(Duration::ZERO);
        assert!(!tokens.consume(&expired));
        assert!(!tokens.consume("not-hex"));
    }

    #[test]
    fn throttle_backs_off_exponentially_and_resets() {
        let ip: IpAddr = "10.0.0.7".parse().unwrap();
        let now = Instant::now();
        let mut t = FailureThrottle::default();
        for _ in 0..FREE_ATTEMPTS - 1 {
            t.record_failure(ip, now);
            assert!(t.check(ip, now).is_ok());
        }
        t.record_failure(ip, now);
        assert_eq!(t.check(ip, now), Err(Duration::from_secs(1)));
        t.record_failure(ip, now);
        assert_eq!(t.check(ip, now), Err(Duration::from_secs(2)));
        assert!(t.check(ip, now + Duration::from_secs(3)).is_ok());
        for _ in 0..40 {
            t.record_failure(ip, now);
        }
        assert_eq!(t.check(ip, now), Err(MAX_BACKOFF));
        t.record_success(ip);
        assert!(t.check(ip, now).is_ok());
    }
}
