use sybil_core::{primitives::compute_nullifier, IdentityNullifier, SybilError, SybilResult};

use crate::store::NullifierStore;

/// Nullifier Manager: generates, registers, and verifies identity nullifiers.
pub struct NullifierManager {
    store: NullifierStore,
}

impl NullifierManager {
    pub fn new() -> Self {
        Self {
            store: NullifierStore::new(),
        }
    }

    /// Generate a nullifier from an identity seed.
    ///
    /// The same seed always produces the same nullifier.
    pub fn generate_nullifier(&self, seed: &[u8]) -> IdentityNullifier {
        IdentityNullifier(compute_nullifier(seed))
    }

    /// Check if a nullifier has already been used.
    pub fn nullifier_exists(&self, nullifier: &IdentityNullifier) -> bool {
        self.store.exists(nullifier)
    }

    /// Register a nullifier as used.
    pub fn register_nullifier(&mut self, nullifier: IdentityNullifier) -> SybilResult<()> {
        if self.store.exists(&nullifier) {
            return Err(SybilError::NullifierAlreadyExists(nullifier.0.clone()));
        }
        self.store.insert(nullifier);
        Ok(())
    }

    /// Verify that a nullifier is valid and unused, then mark it used.
    ///
    /// Returns true if the nullifier was valid and consumed.
    /// Returns false if the nullifier was already used.
    pub fn verify_and_consume(&mut self, nullifier: &IdentityNullifier) -> SybilResult<bool> {
        if self.store.exists(nullifier) {
            Ok(false)
        } else {
            self.store.insert(nullifier.clone());
            Ok(true)
        }
    }

    /// Get the total number of registered nullifiers.
    pub fn total_nullifiers(&self) -> usize {
        self.store.len()
    }
}

impl Default for NullifierManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_register() {
        let mut manager = NullifierManager::new();
        let seed = b"my-identity-secret";
        let nullifier = manager.generate_nullifier(seed);
        assert!(!manager.nullifier_exists(&nullifier));
        manager.register_nullifier(nullifier.clone()).unwrap();
        assert!(manager.nullifier_exists(&nullifier));
    }

    #[test]
    fn test_duplicate_registration_rejected() {
        let mut manager = NullifierManager::new();
        let seed = b"same-secret";
        let nullifier = manager.generate_nullifier(seed);
        manager.register_nullifier(nullifier.clone()).unwrap();
        assert!(manager.register_nullifier(nullifier).is_err());
    }

    #[test]
    fn test_verify_and_consume() {
        let mut manager = NullifierManager::new();
        let seed = b"fresh-secret";
        let nullifier = manager.generate_nullifier(seed);
        assert!(manager.verify_and_consume(&nullifier).unwrap());
        // Second call should return false (already consumed)
        assert!(!manager.verify_and_consume(&nullifier).unwrap());
    }

    #[test]
    fn test_deterministic_generation() {
        let manager = NullifierManager::new();
        let n1 = manager.generate_nullifier(b"seed");
        let n2 = manager.generate_nullifier(b"seed");
        assert_eq!(n1, n2);
    }

    #[test]
    fn test_different_seeds_different_nullifiers() {
        let manager = NullifierManager::new();
        let n1 = manager.generate_nullifier(b"seed-a");
        let n2 = manager.generate_nullifier(b"seed-b");
        assert_ne!(n1, n2);
    }

    #[test]
    fn test_total_nullifiers() {
        let mut manager = NullifierManager::new();
        assert_eq!(manager.total_nullifiers(), 0);
        manager
            .register_nullifier(manager.generate_nullifier(b"a"))
            .unwrap();
        assert_eq!(manager.total_nullifiers(), 1);
        manager
            .register_nullifier(manager.generate_nullifier(b"b"))
            .unwrap();
        assert_eq!(manager.total_nullifiers(), 2);
    }
}