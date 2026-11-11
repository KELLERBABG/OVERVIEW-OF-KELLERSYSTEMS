use std::collections::HashSet;
use sybil_core::IdentityNullifier;

/// Thread-safe, in-memory store for registered identity nullifiers.
///
/// In production, this would be backed by a persistent database.
#[derive(Default)]
pub struct NullifierStore {
    nullifiers: HashSet<IdentityNullifier>,
}

impl NullifierStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a nullifier exists in the store.
    pub fn exists(&self, nullifier: &IdentityNullifier) -> bool {
        self.nullifiers.contains(nullifier)
    }

    /// Insert a nullifier into the store.
    pub fn insert(&mut self, nullifier: IdentityNullifier) {
        self.nullifiers.insert(nullifier);
    }

    /// Remove a nullifier from the store.
    pub fn remove(&mut self, nullifier: &IdentityNullifier) {
        self.nullifiers.remove(nullifier);
    }

    /// Get the number of stored nullifiers.
    pub fn len(&self) -> usize {
        self.nullifiers.len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.nullifiers.is_empty()
    }
}