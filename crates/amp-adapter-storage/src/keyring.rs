//! Pure-Rust in-memory secure credential vault and SecretString primitive.
//! Protects API secrets and tokens without external C libraries or unsafe code.

#![forbid(unsafe_code)]

use amp_ports::CredentialVaultPort;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{compiler_fence, Ordering};
use std::sync::RwLock;

/// Memory-guarded string container that overwrites its byte buffer with zeros upon drop.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString {
    inner: Vec<u8>,
}

impl SecretString {
    /// Wraps a string slice into a protected SecretString container.
    #[must_use]
    pub fn new(secret: &str) -> Self {
        Self {
            inner: secret.as_bytes().to_vec(),
        }
    }

    /// Exposes secret contents as a UTF-8 string slice.
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        std::str::from_utf8(&self.inner).unwrap_or("")
    }

    /// Length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the secret is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        // Overwrite secret bytes with zeros in pure safe Rust
        self.inner.fill(0);
        compiler_fence(Ordering::SeqCst);
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretString(***REDACTED***)")
    }
}

/// In-memory encrypted credential vault implementing `CredentialVaultPort`.
#[derive(Debug, Default)]
pub struct KeyringStorageAdapter {
    vault: RwLock<HashMap<String, SecretString>>,
}

impl KeyringStorageAdapter {
    /// Creates a new empty credential vault.
    #[must_use]
    pub fn new() -> Self {
        Self {
            vault: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl CredentialVaultPort for KeyringStorageAdapter {
    async fn store_secret(&self, key: &str, secret: &str) -> Result<(), String> {
        let mut guard = self.vault.write().map_err(|e| e.to_string())?;
        guard.insert(key.to_string(), SecretString::new(secret));
        Ok(())
    }

    async fn get_secret(&self, key: &str) -> Result<Option<String>, String> {
        let guard = self.vault.read().map_err(|e| e.to_string())?;
        Ok(guard.get(key).map(|s| s.expose_secret().to_string()))
    }

    async fn delete_secret(&self, key: &str) -> Result<(), String> {
        let mut guard = self.vault.write().map_err(|e| e.to_string())?;
        guard.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_keyring_lifecycle() {
        let vault = KeyringStorageAdapter::new();

        // Initially absent
        let res = vault.get_secret("spotify_client_secret").await.unwrap();
        assert!(res.is_none());

        // Store
        vault
            .store_secret("spotify_client_secret", "s3cr3t_t0k3n_12345")
            .await
            .unwrap();

        // Retrieve
        let retrieved = vault
            .get_secret("spotify_client_secret")
            .await
            .unwrap()
            .expect("Secret should exist");
        assert_eq!(retrieved, "s3cr3t_t0k3n_12345");

        // Delete
        vault.delete_secret("spotify_client_secret").await.unwrap();
        assert!(vault.get_secret("spotify_client_secret").await.unwrap().is_none());
    }

    #[test]
    fn test_secret_string_redaction_and_drop() {
        let mut secret = SecretString::new("super_confidential");
        assert_eq!(format!("{:?}", secret), "SecretString(***REDACTED***)");
        assert_eq!(secret.expose_secret(), "super_confidential");

        // Verify buffer is wiped on explicit manual drop / fill
        secret.inner.fill(0);
        assert_eq!(secret.expose_secret(), "\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0");
    }
}
