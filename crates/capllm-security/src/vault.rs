//! AES-256-GCM encrypted transient in-memory vault.
//!
//! Stores DLP-redacted original values encrypted with a per-process random key.
//! Entries expire after a configurable TTL and are never persisted to disk.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit};
use parking_lot::RwLock;

/// Default TTL for vault entries (5 minutes).
const DEFAULT_TTL: Duration = Duration::from_secs(300);

/// A single encrypted vault entry.
struct VaultEntry {
    /// AES-256-GCM encrypted ciphertext.
    ciphertext: Vec<u8>,
    /// Nonce used for encryption (96 bits).
    nonce: aes_gcm::aead::Nonce<Aes256Gcm>,
    /// When this entry was created.
    created_at: Instant,
}

/// Encrypted transient in-memory store for DLP-redacted original values.
///
/// - Per-process random AES-256 key (never written to disk)
/// - Each entry individually encrypted with a unique nonce
/// - TTL-based expiration with periodic cleanup
pub struct SensitiveVault {
    cipher: Aes256Gcm,
    entries: RwLock<HashMap<String, VaultEntry>>,
    ttl: Duration,
}

impl SensitiveVault {
    /// Create a new vault with a random encryption key.
    #[must_use]
    pub fn new() -> Self {
        let key = Aes256Gcm::generate_key(OsRng);
        Self::with_key(&key)
    }

    /// Create a vault with a specific key (for testing).
    fn with_key(key: &Key<Aes256Gcm>) -> Self {
        Self {
            cipher: Aes256Gcm::new(key),
            entries: RwLock::new(HashMap::new()),
            ttl: DEFAULT_TTL,
        }
    }

    /// Store a plaintext value, encrypting it with AES-256-GCM.
    ///
    /// The `id` should be the unique placeholder suffix (e.g. `a1b2c3d4`).
    pub fn store(&self, id: &str, plaintext: &str) {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let Ok(ciphertext) = self.cipher.encrypt(&nonce, plaintext.as_bytes()) else {
            tracing::error!(id, "vault encryption failed");
            return;
        };

        let entry = VaultEntry {
            ciphertext,
            nonce,
            created_at: Instant::now(),
        };

        self.entries.write().insert(id.to_owned(), entry);
    }

    /// Retrieve and decrypt a stored value. Returns `None` if expired or missing.
    #[allow(clippy::significant_drop_tightening)]
    pub fn retrieve(&self, id: &str) -> Option<String> {
        let (nonce, ciphertext) = {
            let entries = self.entries.read();
            let entry = entries.get(id)?;

            if entry.created_at.elapsed() > self.ttl {
                None
            } else {
                Some((entry.nonce, entry.ciphertext.clone()))
            }
        }?;

        self.cipher
            .decrypt(&nonce, ciphertext.as_ref())
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }

    /// Remove expired entries from the vault.
    pub fn cleanup(&self) {
        let mut entries = self.entries.write();
        entries.retain(|_, entry| entry.created_at.elapsed() <= self.ttl);
    }

    /// Number of active entries (for monitoring).
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Whether the vault is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }
}

impl Default for SensitiveVault {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SensitiveVault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SensitiveVault")
            .field("entries", &self.entries.read().len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_and_retrieve_roundtrip() {
        let vault = SensitiveVault::new();
        vault.store("test-1", "123-45-6789");

        let result = vault.retrieve("test-1");
        assert_eq!(result, Some("123-45-6789".to_owned()));
    }

    #[test]
    fn retrieve_missing_returns_none() {
        let vault = SensitiveVault::new();
        assert_eq!(vault.retrieve("nonexistent"), None);
    }

    #[test]
    fn expired_entry_returns_none() {
        let key = Aes256Gcm::generate_key(OsRng);
        let vault = SensitiveVault {
            cipher: Aes256Gcm::new(&key),
            entries: RwLock::new(HashMap::new()),
            ttl: Duration::from_millis(1), // 1ms TTL for testing
        };

        vault.store("expire-test", "secret");
        std::thread::sleep(Duration::from_millis(10));

        assert_eq!(vault.retrieve("expire-test"), None);
    }

    #[test]
    fn cleanup_removes_expired() {
        let key = Aes256Gcm::generate_key(OsRng);
        let vault = SensitiveVault {
            cipher: Aes256Gcm::new(&key),
            entries: RwLock::new(HashMap::new()),
            ttl: Duration::from_millis(1),
        };

        vault.store("old-1", "data1");
        vault.store("old-2", "data2");
        std::thread::sleep(Duration::from_millis(10));

        assert_eq!(vault.len(), 2);
        vault.cleanup();
        assert_eq!(vault.len(), 0);
    }

    #[test]
    fn multiple_entries_independent() {
        let vault = SensitiveVault::new();
        vault.store("a", "value-a");
        vault.store("b", "value-b");
        vault.store("c", "value-c");

        assert_eq!(vault.retrieve("a"), Some("value-a".to_owned()));
        assert_eq!(vault.retrieve("b"), Some("value-b".to_owned()));
        assert_eq!(vault.retrieve("c"), Some("value-c".to_owned()));
        assert_eq!(vault.len(), 3);
    }
}
