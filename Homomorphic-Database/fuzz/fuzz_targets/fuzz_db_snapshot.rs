//! Fuzz target: encrypted-database snapshot deserialization.
//!
//! `EncryptedDatabase::from_bytes` runs when a `--data-dir` snapshot/WAL is
//! loaded on boot. The invariant under test: a corrupt or crafted snapshot
//! yields `DbError::CorruptedDatabase` (or decodes), never a panic and never
//! an absurd allocation.
//!
//! Run: `cargo fuzz run fuzz_db_snapshot` (needs clang; see `fuzz/README.md`).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = fhe_db::EncryptedDatabase::from_bytes(data);
});
