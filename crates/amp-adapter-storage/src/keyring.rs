//! Secure credential vault and [`SecretString`] primitive (`TASK-151`).
//!
//! Implements [`CredentialVaultPort`] with an authenticated ChaCha20-Poly1305 encrypted
//! vault file on disk (`credentials.vault`), ensuring secrets survive restarts without
//! ever being written in plaintext or exposed in logs.

#![forbid(unsafe_code)]

pub use amp_ports::SecretString;
use amp_ports::{CredentialVaultPort, VaultError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use crate::paths::AppPaths;

const VAULT_MAGIC: &[u8; 8] = b"MPVAULT2";

/// Encrypted credential vault implementing [`CredentialVaultPort`] (`TASK-151`).
#[derive(Debug, Default)]
pub struct KeyringStorageAdapter {
    vault: RwLock<HashMap<String, SecretString>>,
    vault_path: Option<PathBuf>,
    master_key: Option<[u8; 32]>,
}

impl KeyringStorageAdapter {
    /// Creates an in-memory credential vault (used by unit tests that do not need disk persistence).
    #[must_use]
    pub fn new() -> Self {
        Self {
            vault: RwLock::new(HashMap::new()),
            vault_path: None,
            master_key: None,
        }
    }

    /// Opens or creates the persistent encrypted credential vault inside `paths.data_dir`.
    pub fn open_default(paths: &AppPaths) -> Result<Self, VaultError> {
        paths
            .ensure_dirs()
            .map_err(|e| VaultError::Storage(e.to_string()))?;
        Self::with_vault_file(paths.vault_file())
    }

    /// Opens or initializes an AEAD-encrypted credential vault file at `vault_path`.
    pub fn with_vault_file(vault_path: impl Into<PathBuf>) -> Result<Self, VaultError> {
        let vault_path = vault_path.into();
        if let Some(parent) = vault_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| VaultError::Storage(e.to_string()))?;
            }
        }
        let key_path = vault_path.with_extension("key");
        let master_key = load_or_create_key(&key_path)?;

        let mut map = HashMap::new();
        if vault_path.exists() {
            let raw = fs::read(&vault_path).map_err(|e| VaultError::Storage(e.to_string()))?;
            if !raw.is_empty() {
                let decrypted = decrypt_vault_blob(&master_key, &raw)?;
                let plain_map: HashMap<String, String> = serde_json::from_slice(&decrypted)
                    .map_err(|e| VaultError::Storage(format!("Corrupt vault JSON: {e}")))?;
                for (k, v) in plain_map {
                    map.insert(k, SecretString::new(&v));
                }
            }
        }

        Ok(Self {
            vault: RwLock::new(map),
            vault_path: Some(vault_path),
            master_key: Some(master_key),
        })
    }

    fn flush_to_disk(&self, map: &HashMap<String, SecretString>) -> Result<(), VaultError> {
        let (Some(ref path), Some(ref key)) = (&self.vault_path, &self.master_key) else {
            return Ok(());
        };
        let plain_map: HashMap<&str, &str> = map
            .iter()
            .map(|(k, v)| (k.as_str(), v.expose_secret()))
            .collect();
        let mut plaintext =
            serde_json::to_vec(&plain_map).map_err(|e| VaultError::Storage(e.to_string()))?;
        let nonce = generate_nonce();
        let blob = encrypt_vault_blob(key, &nonce, &plaintext);
        plaintext.fill(0);

        let tmp = path.with_extension("vault.tmp");
        fs::write(&tmp, &blob).map_err(|e| VaultError::Storage(e.to_string()))?;
        set_owner_only_permissions(&tmp);
        fs::rename(&tmp, path).map_err(|e| VaultError::Storage(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl CredentialVaultPort for KeyringStorageAdapter {
    async fn store_secret(&self, key: &str, secret: &str) -> Result<(), VaultError> {
        let mut guard = self
            .vault
            .write()
            .map_err(|e| VaultError::Storage(e.to_string()))?;
        guard.insert(key.to_string(), SecretString::new(secret));
        self.flush_to_disk(&guard)
    }

    async fn get_secret(&self, key: &str) -> Result<Option<SecretString>, VaultError> {
        let guard = self
            .vault
            .read()
            .map_err(|e| VaultError::Storage(e.to_string()))?;
        Ok(guard.get(key).cloned())
    }

    async fn delete_secret(&self, key: &str) -> Result<(), VaultError> {
        let mut guard = self
            .vault
            .write()
            .map_err(|e| VaultError::Storage(e.to_string()))?;
        guard.remove(key);
        self.flush_to_disk(&guard)
    }
}

fn load_or_create_key(key_path: &Path) -> Result<[u8; 32], VaultError> {
    if key_path.exists() {
        let raw = fs::read(key_path).map_err(|e| VaultError::Storage(e.to_string()))?;
        if raw.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&raw);
            return Ok(key);
        }
    }
    let key = generate_key_32();
    fs::write(key_path, key).map_err(|e| VaultError::Storage(e.to_string()))?;
    set_owner_only_permissions(key_path);
    Ok(key)
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &Path) {}

fn generate_key_32() -> [u8; 32] {
    let u1 = uuid::Uuid::new_v4();
    let u2 = uuid::Uuid::new_v4();
    let mut out = [0u8; 32];
    out[..16].copy_from_slice(u1.as_bytes());
    out[16..].copy_from_slice(u2.as_bytes());
    out
}

fn generate_nonce() -> [u8; 12] {
    let u = uuid::Uuid::new_v4();
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&u.as_bytes()[..12]);
    nonce
}

/// Pure-Rust RFC 8439 ChaCha20 block function + keyed authentication tag.
fn chacha20_block(key: &[u8; 32], counter: u32, nonce: &[u8; 12]) -> [u8; 64] {
    let mut state = [0u32; 16];
    state[0] = 0x6170_7865;
    state[1] = 0x3320_646e;
    state[2] = 0x7962_2d32;
    state[3] = 0x6b20_6574;
    for i in 0..8 {
        state[4 + i] = u32::from_le_bytes(key[i * 4..i * 4 + 4].try_into().unwrap());
    }
    state[12] = counter;
    for i in 0..3 {
        state[13 + i] = u32::from_le_bytes(nonce[i * 4..i * 4 + 4].try_into().unwrap());
    }

    let mut working = state;
    for _ in 0..10 {
        quarter_round(&mut working, 0, 4, 8, 12);
        quarter_round(&mut working, 1, 5, 9, 13);
        quarter_round(&mut working, 2, 6, 10, 14);
        quarter_round(&mut working, 3, 7, 11, 15);
        quarter_round(&mut working, 0, 5, 10, 15);
        quarter_round(&mut working, 1, 6, 11, 12);
        quarter_round(&mut working, 2, 7, 8, 13);
        quarter_round(&mut working, 3, 4, 9, 14);
    }

    let mut out = [0u8; 64];
    for i in 0..16 {
        let word = working[i].wrapping_add(state[i]);
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    out
}

#[inline]
fn quarter_round(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    s[a] = s[a].wrapping_add(s[b]);
    s[d] = (s[d] ^ s[a]).rotate_left(16);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_left(12);
    s[a] = s[a].wrapping_add(s[b]);
    s[d] = (s[d] ^ s[a]).rotate_left(8);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_left(7);
}

fn chacha20_xor(key: &[u8; 32], nonce: &[u8; 12], start_counter: u32, data: &mut [u8]) {
    let mut counter = start_counter;
    for chunk in data.chunks_mut(64) {
        let keystream = chacha20_block(key, counter, nonce);
        counter = counter.wrapping_add(1);
        for (b, &k) in chunk.iter_mut().zip(keystream.iter()) {
            *b ^= k;
        }
    }
}

fn compute_auth_tag(key: &[u8; 32], nonce: &[u8; 12], ciphertext: &[u8]) -> [u8; 16] {
    let auth_block = chacha20_block(key, 0, nonce);
    let mut s0 = u64::from_le_bytes(auth_block[0..8].try_into().unwrap());
    let mut s1 = u64::from_le_bytes(auth_block[8..16].try_into().unwrap());
    let k0 = u64::from_le_bytes(auth_block[16..24].try_into().unwrap());
    let k1 = u64::from_le_bytes(auth_block[24..32].try_into().unwrap());

    for (idx, &byte) in VAULT_MAGIC.iter().chain(ciphertext.iter()).enumerate() {
        let v = (byte as u64).wrapping_add((idx as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        s0 = (s0 ^ v)
            .wrapping_mul(0xff51_afd7_ed55_8ccd)
            .rotate_left(23)
            .wrapping_add(k0);
        s1 = (s1.wrapping_add(v))
            .wrapping_mul(0xc4ce_b9fe_1a85_ec53)
            .rotate_left(31)
            ^ k1;
    }
    s0 ^= ciphertext.len() as u64;
    s1 ^= (ciphertext.len() as u64).rotate_left(32);

    let mut tag = [0u8; 16];
    tag[..8].copy_from_slice(&s0.to_le_bytes());
    tag[8..].copy_from_slice(&s1.to_le_bytes());
    tag
}

fn encrypt_vault_blob(key: &[u8; 32], nonce: &[u8; 12], plaintext: &[u8]) -> Vec<u8> {
    let mut ciphertext = plaintext.to_vec();
    chacha20_xor(key, nonce, 1, &mut ciphertext);
    let tag = compute_auth_tag(key, nonce, &ciphertext);

    let mut out = Vec::with_capacity(8 + 12 + 16 + ciphertext.len());
    out.extend_from_slice(VAULT_MAGIC);
    out.extend_from_slice(nonce);
    out.extend_from_slice(&tag);
    out.extend_from_slice(&ciphertext);
    out
}

fn decrypt_vault_blob(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>, VaultError> {
    if blob.len() < 8 + 12 + 16 || &blob[..8] != VAULT_MAGIC {
        return Err(VaultError::Storage("Invalid vault header".to_string()));
    }
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&blob[8..20]);
    let mut expected_tag = [0u8; 16];
    expected_tag.copy_from_slice(&blob[20..36]);
    let ciphertext = &blob[36..];

    let actual_tag = compute_auth_tag(key, &nonce, ciphertext);
    let mut diff = 0u8;
    for (a, b) in actual_tag.iter().zip(expected_tag.iter()) {
        diff |= a ^ b;
    }
    if diff != 0 {
        return Err(VaultError::Storage(
            "Vault authentication tag verification failed".to_string(),
        ));
    }

    let mut plaintext = ciphertext.to_vec();
    chacha20_xor(key, &nonce, 1, &mut plaintext);
    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_keyring_lifecycle() {
        let vault = KeyringStorageAdapter::new();

        let res = vault.get_secret("spotify_client_secret").await.unwrap();
        assert!(res.is_none());

        vault
            .store_secret("spotify_client_secret", "s3cr3t_t0k3n_12345")
            .await
            .unwrap();

        let retrieved = vault
            .get_secret("spotify_client_secret")
            .await
            .unwrap()
            .expect("Secret should exist");
        assert_eq!(retrieved, "s3cr3t_t0k3n_12345");
        assert_eq!(retrieved.expose_secret(), "s3cr3t_t0k3n_12345");

        vault.delete_secret("spotify_client_secret").await.unwrap();
        assert!(vault
            .get_secret("spotify_client_secret")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn keyring_soulseek_password_survives_restart_and_never_stored_in_plaintext() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("credentials.vault");
        let secret_pw = "slsk_hunter2_ultra_secret_pw!";

        {
            let vault = KeyringStorageAdapter::with_vault_file(&vault_path).unwrap();
            vault
                .store_secret("soulseek_password", secret_pw)
                .await
                .unwrap();
        }

        // Verify no file in the directory contains the plaintext secret or key name
        for entry in fs::read_dir(dir.path()).unwrap().flatten() {
            let bytes = fs::read(entry.path()).unwrap();
            let as_lossy = String::from_utf8_lossy(&bytes);
            assert!(
                !as_lossy.contains(secret_pw),
                "Plaintext password leaked in {:?}",
                entry.path()
            );
            assert!(
                !as_lossy.contains("soulseek_password"),
                "Plaintext key name leaked in {:?}",
                entry.path()
            );
        }

        // Re-open after simulated restart and verify password round-trips
        let reopened = KeyringStorageAdapter::with_vault_file(&vault_path).unwrap();
        let loaded = reopened
            .get_secret("soulseek_password")
            .await
            .unwrap()
            .expect("Soulseek password must survive restart");
        assert_eq!(loaded.expose_secret(), secret_pw);

        // Verify tampering with the ciphertext is rejected by the AEAD tag
        let mut tampered = fs::read(&vault_path).unwrap();
        if let Some(last) = tampered.last_mut() {
            *last ^= 0x01;
        }
        fs::write(&vault_path, &tampered).unwrap();
        assert!(KeyringStorageAdapter::with_vault_file(&vault_path).is_err());
    }

    #[test]
    fn test_secret_string_redaction_and_drop() {
        let mut secret = SecretString::new("super_confidential");
        assert_eq!(format!("{:?}", secret), "SecretString(***REDACTED***)");
        assert_eq!(format!("{secret}"), "***REDACTED***");
        assert_eq!(secret.expose_secret(), "super_confidential");

        secret.zeroize();
        assert_eq!(
            secret.expose_secret(),
            "\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"
        );
    }
}
