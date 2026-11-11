//! # zk-genome-storage-traits
//!
//! Platform-abstracted encrypted storage traits for mobile secure storage.
//!
//! Defines the `SecureStorage` trait and key management types that abstract
//! over iOS Keychain, Android Keystore, and fallback passphrase-derived keys.
//!
//! # Key Hierarchy
//! ```text
//! Secure Enclave
//!   └── Device Key (SE-backed, never exported)
//!         └── Database Encryption Key (wrapped by Device Key)
//!               └── Genome Data (AES-256-GCM in SQLCipher)
//! ```

#![no_std]
#![deny(missing_docs)]
#![deny(unsafe_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// A key identifier for stored secrets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyId(pub String);

impl KeyId {
    /// Create a new key identifier.
    pub const fn new(id: String) -> Self {
        KeyId(id)
    }
}

/// Types of keys managed by the secure storage system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// The device's root key (stored in Secure Enclave / TEE).
    DeviceKey,
    /// The database encryption key (wrapped by device key).
    DatabaseKey,
    /// A passphrase-derived key (PBKDF2, for backup/fallback).
    PassphraseKey,
    /// A DID private key for consent management.
    DidKey,
}

/// Error types for secure storage operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// The key was not found in secure storage.
    KeyNotFound(KeyId),
    /// Cryptographic operation failed.
    CryptoError,
    /// Platform-specific error (Keychain/Keystore).
    PlatformError(String),
    /// Authentication failed (biometric/passcode).
    AuthenticationFailed,
    /// The requested operation requires a higher security level.
    InsufficientSecurity,
}

/// Security level for storage operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    /// No authentication required.
    None,
    /// Device unlock required (passcode).
    DevicePasscode,
    /// Biometric authentication required (Face ID / fingerprint).
    Biometric,
    /// Biometric + device passcode (highest security).
    BiometricAndPasscode,
}

/// Abstraction over platform-specific secure storage.
///
/// On iOS: backed by Keychain with `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`
/// On Android: backed by EncryptedSharedPreferences + Android Keystore
pub trait SecureStorage {
    /// Store a secret in secure storage.
    fn store(&mut self, id: KeyId, data: &[u8], level: SecurityLevel) -> Result<(), StorageError>;

    /// Retrieve a secret from secure storage.
    fn retrieve(&self, id: &KeyId) -> Result<Vec<u8>, StorageError>;

    /// Delete a secret from secure storage.
    fn delete(&mut self, id: &KeyId) -> Result<(), StorageError>;

    /// Check if a key exists in secure storage.
    fn exists(&self, id: &KeyId) -> bool;

    /// Get the current security level for a key.
    fn security_level(&self, id: &KeyId) -> Result<SecurityLevel, StorageError>;
}

/// Database encryption configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct DatabaseConfig {
    /// Path to the SQLCipher database file.
    pub db_path: String,
    /// Whether to enable WAL mode.
    pub wal_mode: bool,
    /// Page size for the database.
    pub page_size: u32,
    /// KDF iterations for SQLCipher key derivation.
    pub kdf_iterations: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            db_path: String::new(),
            wal_mode: true,
            page_size: 4096,
            kdf_iterations: 256_000,
        }
    }
}

/// A simple in-memory implementation of SecureStorage for testing.
#[cfg(feature = "test-utils")]
pub mod test_storage {
    use super::*;
    use alloc::collections::BTreeMap;

    /// In-memory secure storage for testing.
    #[derive(Debug, Clone)]
    pub struct InMemoryStorage {
        store: BTreeMap<KeyId, Vec<u8>>,
        levels: BTreeMap<KeyId, SecurityLevel>,
    }

    impl InMemoryStorage {
        /// Create a new empty in-memory storage.
        pub fn new() -> Self {
            InMemoryStorage {
                store: BTreeMap::new(),
                levels: BTreeMap::new(),
            }
        }
    }

    impl SecureStorage for InMemoryStorage {
        fn store(&mut self, id: KeyId, data: &[u8], level: SecurityLevel) -> Result<(), StorageError> {
            self.store.insert(KeyId(id.0.clone()), data.to_vec());
            self.levels.insert(KeyId(id.0.clone()), level);
            Ok(())
        }

        fn retrieve(&self, id: &KeyId) -> Result<Vec<u8>, StorageError> {
            self.store.get(id).cloned().ok_or_else(|| StorageError::KeyNotFound(KeyId(id.0.clone())))
        }

        fn delete(&mut self, id: &KeyId) -> Result<(), StorageError> {
            self.store.remove(id);
            self.levels.remove(id);
            Ok(())
        }

        fn exists(&self, id: &KeyId) -> bool {
            self.store.contains_key(id)
        }

        fn security_level(&self, id: &KeyId) -> Result<SecurityLevel, StorageError> {
            self.levels.get(id).copied().ok_or_else(|| StorageError::KeyNotFound(KeyId(id.0.clone())))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_store_and_retrieve() {
            let mut storage = InMemoryStorage::new();
            let id = KeyId("test_key".into());
            storage.store(id.clone(), b"secret data", SecurityLevel::Biometric).unwrap();
            assert!(storage.exists(&id));
            let data = storage.retrieve(&id).unwrap();
            assert_eq!(data, b"secret data");
        }

        #[test]
        fn test_delete() {
            let mut storage = InMemoryStorage::new();
            let id = KeyId("temp".into());
            storage.store(id.clone(), b"data", SecurityLevel::None).unwrap();
            assert!(storage.exists(&id));
            storage.delete(&id).unwrap();
            assert!(!storage.exists(&id));
        }

        #[test]
        fn test_key_not_found() {
            let storage = InMemoryStorage::new();
            let result = storage.retrieve(&KeyId("nonexistent".into()));
            assert!(matches!(result, Err(StorageError::KeyNotFound(_))));
        }
    }
}