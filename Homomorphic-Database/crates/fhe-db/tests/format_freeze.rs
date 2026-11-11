//! Format-freeze gate for the on-disk write-ahead log.
//!
//! The external review is scoped to a fixed persistence format. While the
//! review is open, the log header must not drift silently: a changed magic,
//! version, or header size would make the auditor's reviewed layout wrong (and
//! would make an old log unreadable without a migration).
//!
//! Deliberate change procedure: see `docs/audit/FORMAT_FREEZE.md`.

use fhe_db::{CURRENT_LOG_MAGIC, CURRENT_LOG_VERSION, LOG_HEADER_LEN};

/// The log format version the external review is scoped to.
const FROZEN_LOG_VERSION: u64 = 1;

#[test]
fn wal_format_constants_are_frozen() {
    assert_eq!(CURRENT_LOG_MAGIC, b"FHELOG01");
    assert_eq!(
        CURRENT_LOG_VERSION, FROZEN_LOG_VERSION,
        "the WAL format version changed while the external audit is scoped to \
         version {FROZEN_LOG_VERSION}; follow docs/audit/FORMAT_FREEZE.md"
    );
    assert_eq!(LOG_HEADER_LEN, 24);
}

#[test]
fn fresh_log_header_matches_the_documented_layout() {
    let path = std::env::temp_dir().join(format!(
        "fhe_wal_freeze_{}_{}.log",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_file(&path);
    let wal = fhe_db::Wal::create(&path).expect("create WAL");
    drop(wal);

    let bytes = std::fs::read(&path).expect("read WAL");
    assert_eq!(
        bytes.len() as u64,
        LOG_HEADER_LEN,
        "a fresh log is exactly the fixed header"
    );
    assert_eq!(&bytes[0..8], CURRENT_LOG_MAGIC, "magic field");
    assert_eq!(
        u64::from_le_bytes(bytes[8..16].try_into().expect("8-byte version")),
        CURRENT_LOG_VERSION,
        "version field"
    );
    assert_eq!(
        u64::from_le_bytes(bytes[16..24].try_into().expect("8-byte snapshot length")),
        0,
        "a fresh log has an empty snapshot section"
    );

    let _ = std::fs::remove_file(&path);
}
