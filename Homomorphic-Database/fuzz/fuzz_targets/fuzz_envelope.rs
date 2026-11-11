//! Fuzz target: ciphertext envelope deserialization.
//!
//! `Ciphertext::from_envelope_bytes` is the server's wire entry point for
//! every ciphertext it accepts. The invariant under test: **deserialization
//! is total** — arbitrary bytes yield either a valid `Ciphertext` or a typed
//! `FheError`, never a panic, never an absurd allocation (length-prefix
//! bombs must be rejected by the header validation).
//!
//! Run: `cargo fuzz run fuzz_envelope` (needs clang; see `fuzz/README.md`).

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = fhe_core::Ciphertext::from_envelope_bytes(data);
});
