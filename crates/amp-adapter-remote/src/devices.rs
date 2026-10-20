//! Paired devices: one session secret per phone, stored only as a SHA-256 hash, with a name,
//! last-seen time and the host's "trusted: auto-accept" choice (TASK-214, TASK-216).

#![forbid(unsafe_code)]

use crate::auth::{is_hex, random_hex, sha256_hex};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Short random identifier of a paired device. Safe to show; it is not a credential.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeviceId(pub String);

/// What the host sees about a paired device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSummary {
    pub id: DeviceId,
    pub name: String,
    /// Seconds since the Unix epoch.
    pub paired_at: u64,
    /// Seconds since the Unix epoch of the last connection or message.
    pub last_seen: u64,
    /// Requests from this device are accepted without asking.
    pub trusted: bool,
    /// At least one socket of this device is open right now.
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceRecord {
    id: DeviceId,
    name: String,
    hash: String,
    paired_at: u64,
    last_seen: u64,
    #[serde(default)]
    trusted: bool,
}

#[derive(Serialize, Deserialize)]
struct StoreFile {
    version: u32,
    devices: Vec<DeviceRecord>,
}

/// Seconds since the Unix epoch.
#[must_use]
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Session secrets of paired devices, persisted as hashes.
#[derive(Debug, Default)]
pub struct SessionStore {
    devices: Vec<DeviceRecord>,
    path: Option<PathBuf>,
}

impl SessionStore {
    /// Length of a session secret in hex characters (128 bits).
    pub const LEN: usize = 32;

    /// Loads devices from `path` or starts empty. `None` keeps sessions in memory. Files from
    /// before named devices (a bare JSON array of hashes) load as unnamed devices.
    #[must_use]
    pub fn load(path: Option<PathBuf>) -> Self {
        let text = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok());
        let devices = text.as_deref().map(parse_store).unwrap_or_default();
        Self { devices, path }
    }

    /// Pairs a new device named after `name` and returns its id and the plaintext secret.
    pub fn issue(&mut self, name: &str) -> (DeviceId, String) {
        let secret = random_hex(Self::LEN / 2);
        let id = DeviceId(random_hex(4));
        let now = unix_now();
        let name = self.unique_name(name);
        self.devices.push(DeviceRecord {
            id: id.clone(),
            name,
            hash: sha256_hex(&secret),
            paired_at: now,
            last_seen: now,
            trusted: false,
        });
        self.persist();
        (id, secret)
    }

    /// The device holding `secret`, if it was issued and not revoked.
    #[must_use]
    pub fn verify(&self, secret: &str) -> Option<DeviceId> {
        if !is_hex(secret, Self::LEN) {
            return None;
        }
        let hash = sha256_hex(&secret.to_lowercase());
        self.devices
            .iter()
            .find(|d| d.hash == hash)
            .map(|d| d.id.clone())
    }

    /// Records that `id` was just seen. Persisted so the host can show it after a restart.
    pub fn touch(&mut self, id: &DeviceId) {
        if let Some(d) = self.devices.iter_mut().find(|d| &d.id == id) {
            d.last_seen = unix_now();
            self.persist();
        }
    }

    /// Name and trust of `id`.
    #[must_use]
    pub fn describe(&self, id: &DeviceId) -> Option<(String, bool)> {
        self.devices
            .iter()
            .find(|d| &d.id == id)
            .map(|d| (d.name.clone(), d.trusted))
    }

    /// Sets whether requests from `id` are accepted without asking. False if unknown.
    pub fn set_trusted(&mut self, id: &DeviceId, trusted: bool) -> bool {
        let Some(d) = self.devices.iter_mut().find(|d| &d.id == id) else {
            return false;
        };
        d.trusted = trusted;
        self.persist();
        true
    }

    /// Forgets one device; it must scan a new QR code. False if unknown.
    pub fn revoke(&mut self, id: &DeviceId) -> bool {
        let before = self.devices.len();
        self.devices.retain(|d| &d.id != id);
        let removed = self.devices.len() != before;
        if removed {
            self.persist();
        }
        removed
    }

    /// Revokes every session; paired devices must scan a new QR code.
    pub fn revoke_all(&mut self) {
        self.devices.clear();
        self.persist();
    }

    /// Every paired device, oldest first. `connected` is left false for the caller to fill in.
    #[must_use]
    pub fn list(&self) -> Vec<DeviceSummary> {
        self.devices
            .iter()
            .map(|d| DeviceSummary {
                id: d.id.clone(),
                name: d.name.clone(),
                paired_at: d.paired_at,
                last_seen: d.last_seen,
                trusted: d.trusted,
                connected: false,
            })
            .collect()
    }

    /// Number of active sessions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.devices.len()
    }

    /// Returns true if no session is active.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }

    /// `name`, or `name 2`, `name 3`... when another device already uses it.
    fn unique_name(&self, name: &str) -> String {
        let taken = |n: &str| self.devices.iter().any(|d| d.name == n);
        if !taken(name) {
            return name.to_string();
        }
        (2..)
            .map(|i| format!("{name} {i}"))
            .find(|n| !taken(n))
            .unwrap_or_else(|| name.to_string())
    }

    fn persist(&self) {
        let Some(path) = &self.path else { return };
        let file = StoreFile {
            version: 2,
            devices: self.devices.clone(),
        };
        if let Ok(json) = serde_json::to_string(&file) {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let tmp = path.with_extension("tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, path);
            }
        }
    }
}

fn parse_store(text: &str) -> Vec<DeviceRecord> {
    if let Ok(file) = serde_json::from_str::<StoreFile>(text) {
        return file
            .devices
            .into_iter()
            .filter(|d| is_hex(&d.hash, 64) && is_hex(&d.id.0, 8))
            .collect();
    }
    let legacy: Vec<String> = serde_json::from_str(text).unwrap_or_default();
    legacy
        .into_iter()
        .filter(|h| is_hex(h, 64))
        .enumerate()
        .map(|(i, hash)| DeviceRecord {
            id: DeviceId(random_hex(4)),
            name: format!("Paired device {}", i + 1),
            hash,
            paired_at: 0,
            last_seen: 0,
            trusted: false,
        })
        .collect()
}

/// A readable device name from a browser's `User-Agent`, for the host's device list.
#[must_use]
pub fn device_label(user_agent: Option<&str>) -> String {
    let ua = user_agent.unwrap_or("");
    let label = if ua.contains("iPhone") {
        "iPhone"
    } else if ua.contains("iPad") {
        "iPad"
    } else if ua.contains("Android") {
        if ua.contains("Mobile") {
            "Android phone"
        } else {
            "Android tablet"
        }
    } else if ua.contains("Macintosh") {
        "Mac"
    } else if ua.contains("Windows") {
        "Windows PC"
    } else if ua.contains("Linux") || ua.contains("CrOS") {
        "Computer"
    } else {
        "Device"
    };
    label.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path() -> PathBuf {
        std::env::temp_dir()
            .join(format!("multiplay-devices-{}", random_hex(8)))
            .join("sessions.json")
    }

    #[test]
    fn sessions_are_hashed_persisted_and_revocable_per_device() {
        let path = temp_path();
        let mut store = SessionStore::load(Some(path.clone()));
        let (a, secret_a) = store.issue("iPhone");
        let (_, secret_b) = store.issue("iPhone");
        assert_eq!(store.verify(&secret_a), Some(a.clone()));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains(&secret_a));
        assert_eq!(store.list()[1].name, "iPhone 2");

        let mut reloaded = SessionStore::load(Some(path.clone()));
        assert!(reloaded.verify(&secret_b).is_some());
        assert!(reloaded.set_trusted(&a, true));
        assert!(reloaded.revoke(&a));
        let after = SessionStore::load(Some(path.clone()));
        assert!(after.verify(&secret_a).is_none());
        assert!(after.verify(&secret_b).is_some());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn legacy_hash_lists_still_load() {
        let path = temp_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let secret = random_hex(16);
        std::fs::write(&path, format!("[\"{}\"]", sha256_hex(&secret))).unwrap();
        let store = SessionStore::load(Some(path.clone()));
        assert!(store.verify(&secret).is_some());
        assert_eq!(store.list()[0].name, "Paired device 1");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn labels_come_from_the_user_agent() {
        assert_eq!(
            device_label(Some("Mozilla/5.0 (iPhone; CPU iPhone OS 18_0)")),
            "iPhone"
        );
        assert_eq!(
            device_label(Some("Mozilla/5.0 (Linux; Android 15) Mobile Safari")),
            "Android phone"
        );
        assert_eq!(device_label(None), "Device");
    }
}
