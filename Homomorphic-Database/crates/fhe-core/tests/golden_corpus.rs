//! Cross-version golden corpus for the ciphertext envelope format.
//!
//! The files in `tests/golden/` pin the **exact byte layout** of envelope
//! version 1 (a U32 ciphertext encrypting `42` under the committed client
//! key). If [`fhe_core::CURRENT_ENVELOPE_VERSION`] is ever bumped, or the
//! envelope frame changes, this test fails loudly — forcing a deliberate
//! migration decision instead of silently misreading persisted/wire data.
//!
//! Regenerating the corpus (only needed when intentionally changing the
//! format **and** bumping the version):
//!
//! ```text
//! cargo test --release -p fhe-core --test golden_corpus regenerate_golden_corpus -- --ignored
//! ```

use std::path::PathBuf;

use fhe_core::ciphertext::CURRENT_ENVELOPE_VERSION;
use fhe_core::{Ciphertext, CiphertextEnvelope, ClientKey, Encryptor, ScalarWidth};

/// Directory holding the committed golden files.
fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// The golden client key (`v1_client_key.bin`).
fn golden_key_path() -> PathBuf {
    golden_dir().join("v1_client_key.bin")
}

/// The golden envelope (`v1_envelope_u32_42.bin`).
fn golden_envelope_path() -> PathBuf {
    golden_dir().join("v1_envelope_u32_42.bin")
}

/// The committed v1 envelope must deserialize with the **current** version,
/// and must decrypt to `42` under the committed key.
#[test]
fn golden_v1_envelope_decrypts_and_matches_current_version() {
    let key_bytes = std::fs::read(golden_key_path()).expect("golden key file missing");
    let key = ClientKey::from_bytes(&key_bytes).expect("golden key failed to deserialize");

    let envelope_bytes =
        std::fs::read(golden_envelope_path()).expect("golden envelope file missing");
    let envelope: CiphertextEnvelope =
        bincode::deserialize(&envelope_bytes).expect("golden envelope frame failed to decode");
    assert_eq!(
        envelope.version, CURRENT_ENVELOPE_VERSION,
        "golden corpus is for a different envelope version than this build; \
         re-encrypt (or add a migration) before bumping CURRENT_ENVELOPE_VERSION"
    );

    let ct = Ciphertext::from_envelope_bytes(&envelope_bytes).expect("golden envelope rejected");
    assert_eq!(ct.width(), ScalarWidth::U32);
    assert_eq!(ct.decrypt(&key).as_u64(), 42);
}

/// A freshly produced envelope must deserialize back to its plaintext — the
/// live counterpart of the golden check.
#[test]
fn fresh_envelope_matches_golden_semantics() {
    let key_bytes = std::fs::read(golden_key_path()).expect("golden key file missing");
    let key = ClientKey::from_bytes(&key_bytes).expect("golden key failed to deserialize");
    let encryptor = Encryptor::new(&key);
    let ct = encryptor
        .encrypt_u64(42, ScalarWidth::U32)
        .expect("encrypt");
    let bytes = ct.to_envelope_bytes().expect("serialize");
    let back = Ciphertext::from_envelope_bytes(&bytes).expect("deserialize");
    assert_eq!(back.decrypt(&key).as_u64(), 42);
}

/// Regenerate the golden corpus files. Marked `#[ignore]` because it writes
/// into the source tree; run explicitly only when intentionally changing the
/// envelope format (with a version bump).
#[test]
#[ignore]
fn regenerate_golden_corpus() {
    let (client_key, _server_key) = ClientKey::new();
    let encryptor = Encryptor::new(&client_key);
    let ct = encryptor
        .encrypt_u64(42, ScalarWidth::U32)
        .expect("encrypt");

    std::fs::create_dir_all(golden_dir()).expect("create golden dir");
    std::fs::write(golden_key_path(), client_key.to_bytes().expect("key bytes"))
        .expect("write golden key");
    std::fs::write(
        golden_envelope_path(),
        ct.to_envelope_bytes().expect("envelope bytes"),
    )
    .expect("write golden envelope");
    eprintln!(
        "golden corpus written:\n  {}\n  {}",
        golden_key_path().display(),
        golden_envelope_path().display()
    );
}
