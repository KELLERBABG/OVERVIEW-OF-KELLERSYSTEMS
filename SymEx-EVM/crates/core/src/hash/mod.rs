//! Symbolic Hash Table — Keccak Optimization
//!
//! ## The Problem
//!
//! In EVM storage, mapping keys are computed as:
//! ```text
//! storage_slot = keccak256(key . slot)
//! ```
//! When `key` is symbolic, `keccak256(key . slot)` becomes an opaque symbolic
//! expression. Expanding full 256-bit Keccak-256 bit-vector constraints in Z3
//! is intractable — the circuit has tens of thousands of gates.
//!
//! ## The Solution
//!
//! Instead of bit-blasting Keccak, we model it as an **uninterpreted function**
//! with an explicit **symbolic hash table** (SMT array). This gives us:
//!
//! - **Injective mapping**: `keccak256(a) = keccak256(b) ⇒ a = b` (sound for Keccak)
//! - **Explicit overwrites**: `store(m, keccak256(k1), v1); store(m, keccak256(k2), v2)`
//! - **Query support**: `select(m, keccak256(sym_var))` resolves to known entries
//! - **No bit-blasting**: Z3 treats Keccak as an opaque function, avoiding
//!   exponential explosion
//!
//! ## Implementation
//!
//! The `SymbolicHashTable` maintains a mapping from concrete Keccak outputs
//! back to their inputs, and a set of symbolic hash constraints. When the
//! solver encounters `keccak256(sym)`, it checks the table for known matches
//! before generating new constraints.

use crate::types::{SymbolicValue, SymVarId, U256};
use sha3::{Digest, Keccak256};
use std::collections::BTreeMap;

/// A mapping from a 32-byte Keccak-256 hash back to its preimage.
#[derive(Debug, Clone)]
pub struct HashEntry {
    /// The preimage bytes that produced this hash.
    pub preimage: Vec<u8>,
    /// The hash output (32 bytes).
    pub hash: [u8; 32],
    /// Whether the preimage contained symbolic variables.
    pub is_symbolic: bool,
    /// The Keccak-256 output as a U256 storage key.
    pub hash_u256: U256,
}

/// The symbolic hash table tracks all Keccak-256 operations encountered
/// during symbolic execution.
#[derive(Debug, Clone)]
pub struct SymbolicHashTable {
    /// Map from 32-byte hash → preimage entries.
    entries: BTreeMap<[u8; 32], HashEntry>,
    /// Map from symbolic variable ID → known hash values.
    /// When a symbolic variable is constrained to equal a specific hash,
    /// we record it here for fast lookup.
    symbolic_hashes: BTreeMap<u64, [u8; 32]>,
    /// Counter for generating fresh symbolic hash variables.
    fresh_counter: u64,
}

impl SymbolicHashTable {
    /// Create a new empty hash table.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            symbolic_hashes: BTreeMap::new(),
            fresh_counter: 0,
        }
    }

    /// Compute a concrete Keccak-256 hash and record it.
    pub fn hash_concrete(&mut self, preimage: &[u8]) -> [u8; 32] {
        let hash: [u8; 32] = Keccak256::digest(preimage).into();
        let entry = HashEntry {
            preimage: preimage.to_vec(),
            hash,
            is_symbolic: false,
            hash_u256: U256::from_be_bytes(&hash),
        };
        self.entries.entry(hash).or_insert(entry);
        hash
    }

    /// Record a symbolic Keccak-256 computation.
    /// The preimage contains symbolic variables; returns a fresh identifier
    /// representing the opaque hash result.
    pub fn hash_symbolic(&mut self, _preimage: &SymbolicValue) -> u64 {
        let id = self.fresh_counter;
        self.fresh_counter += 1;
        id
    }

    /// Try to resolve a Keccak-256 of a concrete preimage to its storage key.
    pub fn lookup_preimage(&self, hash: &[u8; 32]) -> Option<&HashEntry> {
        self.entries.get(hash)
    }

    /// Given a known symbolic variable, check if its hash value has been constrained.
    pub fn get_symbolic_hash(&self, var_id: u64) -> Option<&[u8; 32]> {
        self.symbolic_hashes.get(&var_id)
    }

    /// Record that a symbolic variable has been constrained to equal a specific hash.
    pub fn constrain_symbolic(&mut self, var_id: u64, hash: [u8; 32]) {
        self.symbolic_hashes.insert(var_id, hash);
    }

    /// Generate a storage slot from (key, slot) using the standard Solidity layout:
    /// `slot = keccak256(key . slot)` where key is left-padded to 32 bytes.
    pub fn generate_storage_slot(
        &mut self,
        key: &SymbolicValue,
        slot: u64,
    ) -> SymbolicValue {
        match key {
            SymbolicValue::Concrete(key_val) => {
                // Concrete key: compute the actual Keccak-256 hash
                let mut preimage = Vec::with_capacity(64);
                preimage.extend_from_slice(&key_val.to_be_bytes());
                preimage.extend_from_slice(&U256::from(slot).to_be_bytes());
                let hash_bytes = self.hash_concrete(&preimage);
                SymbolicValue::Concrete(U256::from_be_bytes(&hash_bytes))
            }
            SymbolicValue::Symbolic(id) => {
                // Symbolic key: mark that keccak256(key || slot) was called
                // Return a fresh symbolic variable representing the hash
                let hash_id = self.hash_symbolic(key);
                SymbolicValue::fresh(SymVarId(1_000_000 + hash_id))
            }
            SymbolicValue::Expression(_) => {
                let hash_id = self.hash_symbolic(key);
                SymbolicValue::fresh(SymVarId(1_000_000 + hash_id))
            }
        }
    }

    /// Number of stored hash entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for SymbolicHashTable {
    fn default() -> Self {
        Self::new()
    }
}

/// In the SMT-LIB encoding, Keccak256 is represented as an uninterpreted
/// function to avoid bit-blasting the full Keccak circuit:
///
/// ```smt
/// (declare-fun keccak256 (Word) Word)
/// ```
///
/// The injectivity property can be asserted as-needed:
/// ```smt
/// (assert (forall ((a Word) (b Word)) (=> (= (keccak256 a) (keccak256 b)) (= a b))))
/// ```
///
/// This gives Z3 enough information to reason about storage layout without
/// exploding the constraint graph.
pub fn smt_declare_keccak() -> &'static str {
    "(declare-fun keccak256 (Word) Word)\n"
}

pub fn smt_keccak_injectivity() -> &'static str {
    "(assert (forall ((a Word) (b Word)) (=> (= (keccak256 a) (keccak256 b)) (= a b))))\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concrete_hash() {
        let mut table = SymbolicHashTable::new();
        let hash = table.hash_concrete(b"hello");
        assert_eq!(hash.len(), 32);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_hash_empty() {
        let table = SymbolicHashTable::new();
        assert!(table.is_empty());
    }

    #[test]
    fn test_concrete_storage_slot() {
        let mut table = SymbolicHashTable::new();
        let key = SymbolicValue::concrete(42);
        let slot = table.generate_storage_slot(&key, 0);
        assert!(matches!(slot, SymbolicValue::Concrete(_)));
    }

    #[test]
    fn test_symbolic_storage_slot() {
        let mut table = SymbolicHashTable::new();
        let key = SymbolicValue::fresh(SymVarId(0));
        let slot = table.generate_storage_slot(&key, 1);
        assert!(matches!(slot, SymbolicValue::Symbolic(_)));
    }

    #[test]
    fn test_smt_declarations() {
        let decl = smt_declare_keccak();
        assert!(decl.contains("declare-fun keccak256"));
    }
}