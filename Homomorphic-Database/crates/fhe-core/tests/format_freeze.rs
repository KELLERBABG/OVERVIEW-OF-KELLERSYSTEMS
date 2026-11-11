//! Format-freeze gate for the external audit.
//!
//! The external review is scoped to a **fixed** cryptographic surface: the
//! envelope frame, the committed golden corpus, and the TFHE parameter set.
//! While the review is open, those must not drift silently. This test fails if
//! any of them changes, forcing a deliberate, documented decision instead of
//! an accidental format break.
//!
//! Deliberate change procedure (see `docs/audit/FORMAT_FREEZE.md`):
//!
//! 1. Bump [`CURRENT_ENVELOPE_VERSION`].
//! 2. Regenerate the golden corpus
//!    (`cargo test --release -p fhe-core --test golden_corpus
//!    regenerate_golden_corpus -- --ignored`).
//! 3. Update the hashes and lengths below.
//! 4. Update `docs/audit/FORMAT_FREEZE.md` with the version, date, and reason.
//! 5. Notify the auditor; do not change the format while a review is running.
//!
//! Why the hashes matter: the golden corpus is the auditor's independent
//! known-answer artifact. If the bytes change without the version changing,
//! the corpus no longer pins what the auditor reviewed.

use std::path::PathBuf;

use fhe_core::ciphertext::CURRENT_ENVELOPE_VERSION;
use fhe_core::SecurityParameters;
use sha2::{Digest, Sha256};

/// The envelope format version the external review is scoped to.
const FROZEN_ENVELOPE_VERSION: u64 = 1;

/// Frozen `v1_client_key.bin`: SHA-256 and byte length.
const GOLDEN_KEY_SHA256: &str = "5cf6ff9d2ff2ad9ad513dcd3fb25ac85a712ff0737e87333d9d9c59480a62b20";
const GOLDEN_KEY_LEN: u64 = 31_267;

/// Frozen `v1_envelope_u32_42.bin`: SHA-256 and byte length.
const GOLDEN_ENVELOPE_SHA256: &str =
    "15054892d2ced814f7aede3381cc97c5f00fe99ac5b4052f229a087a9c4133d8";
const GOLDEN_ENVELOPE_LEN: u64 = 263_465;

/// The golden fixture directory.
fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// SHA-256 (hex) and byte length of a file.
fn digest(path: &std::path::Path) -> (String, u64) {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        panic!("cannot read frozen artifact {}: {e}", path.display());
    });
    let hash = Sha256::digest(&bytes);
    let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    (hex, bytes.len() as u64)
}

#[test]
fn envelope_version_is_frozen_at_v1() {
    assert_eq!(
        u64::from(CURRENT_ENVELOPE_VERSION),
        FROZEN_ENVELOPE_VERSION,
        "CURRENT_ENVELOPE_VERSION changed while the external audit is scoped \
         to version {FROZEN_ENVELOPE_VERSION}; follow docs/audit/FORMAT_FREEZE.md"
    );
}

#[test]
fn golden_corpus_bytes_are_frozen() {
    let (key_hash, key_len) = digest(&golden_dir().join("v1_client_key.bin"));
    assert_eq!(key_len, GOLDEN_KEY_LEN, "golden client key length changed");
    assert_eq!(
        key_hash, GOLDEN_KEY_SHA256,
        "golden client key bytes changed"
    );

    let (envelope_hash, envelope_len) = digest(&golden_dir().join("v1_envelope_u32_42.bin"));
    assert_eq!(
        envelope_len, GOLDEN_ENVELOPE_LEN,
        "golden envelope length changed"
    );
    assert_eq!(
        envelope_hash, GOLDEN_ENVELOPE_SHA256,
        "golden envelope bytes changed"
    );
}

#[test]
fn documented_security_parameters_are_frozen() {
    // The same values as `default_parameters_match_documented_pinning`, kept
    // here as the auditor-facing pin with the full set spelled out.
    let p = SecurityParameters::pinned_default();
    assert_eq!(p.lwe_dimension, 918);
    assert_eq!(p.glwe_dimension, 1);
    assert_eq!(p.polynomial_size, 2048);
    assert_eq!(p.pbs_base_log, 23);
    assert_eq!(p.pbs_level, 1);
    assert_eq!(p.ks_base_log, 4);
    assert_eq!(p.ks_level, 4);
    assert_eq!(p.message_modulus, 4);
    assert_eq!(p.carry_modulus, 4);
    assert_eq!(p.max_noise_level, 5);
    assert_eq!(p.log2_p_fail, -129.581);
}
