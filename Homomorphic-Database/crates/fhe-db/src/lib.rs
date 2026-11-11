//! # fhe-db — Encrypted Database Engine
//!
//! A columnar database engine that stores and aggregates **fully encrypted
//! data**. The server that uses this crate never sees value plaintext: inserts
//! are [`fhe_core::Ciphertext`]s and aggregations (SUM / AVG / COUNT) are
//! evaluated homomorphically with [`fhe_core::CiphertextAlgebra`]. Schema
//! metadata is plaintext by design.
//!
//! ## Layout
//!
//! - [`schema`] — [`ColumnType`], [`ColumnDef`], [`TableSchema`],
//!   [`SchemaError`]: plaintext metadata describing table shapes.
//! - [`database`] — [`Table`], [`EncryptedDatabase`]: in-memory columnar
//!   storage with bincode persistence to disk.
//! - [`query`] — [`QueryEngine`], [`AggregateResult`]: homomorphic SUM /
//!   AVG / COUNT queries.
//! - [`wal`] — [`Wal`], [`WalBackedDatabase`]: crash-safe write-ahead
//!   journaling with atomic checkpoints.
//! - [`error`] — [`DbError`], [`DbResult`]: first-class typed errors.
//!
//! ## Security Model
//!
//! Only the **client** holds the decryption key. A server running this
//! engine stores ciphertexts and evaluates aggregations *blindly*: it learns
//! row counts (public metadata) and ciphertexts, never plaintext values.
//!
//! ## Dependency Rule
//!
//! This crate depends on [`fhe_core`] **only** — it never touches TFHE-rs
//! directly.

pub mod database;
pub mod error;
pub mod query;
pub mod schema;
pub mod wal;

pub use database::{EncryptedDatabase, Table};
pub use error::{DbError, DbResult};
pub use query::{AggregateResult, QueryEngine};
pub use schema::{ColumnDef, ColumnType, SchemaError, TableSchema};
pub use wal::{
    CheckpointReport, RecoveryReport, Wal, WalBackedDatabase, WalRecord, CURRENT_LOG_MAGIC,
    CURRENT_LOG_VERSION, LOG_HEADER_LEN,
};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use fhe_core::{ClientKey, Encryptor, ScalarWidth};

    fn setup() -> (ClientKey, EncryptedDatabase) {
        let (client_key, server_key) = ClientKey::new();
        // Server key installed for the query path in these integration tests.
        fhe_core::keys::ServerKeyGuard::new(&server_key);
        let mut db = EncryptedDatabase::new();
        let schema = TableSchema::new("payments")
            .with_column(ColumnDef {
                name: "amount".to_string(),
                column_type: ColumnType::U32,
            })
            .with_column(ColumnDef {
                name: "note".to_string(),
                column_type: ColumnType::U16,
            });
        db.create_table(schema).unwrap();
        (client_key, db)
    }

    fn encrypt(ck: &ClientKey, value: u64, width: ScalarWidth) -> fhe_core::Ciphertext {
        Encryptor::new(ck).encrypt_u64(value, width).unwrap()
    }

    #[test]
    fn end_to_end_encrypted_sum_avg_count() {
        let (client_key, mut db) = setup();
        let engine = QueryEngine::new();

        // Insert three encrypted rows into `amount` and two into `note`.
        let amounts = vec![
            encrypt(&client_key, 10, ScalarWidth::U32),
            encrypt(&client_key, 20, ScalarWidth::U32),
            encrypt(&client_key, 30, ScalarWidth::U32),
        ];
        let inserted = db
            .insert_ciphertexts("payments", "amount", amounts)
            .unwrap();
        assert_eq!(inserted, 3);

        let notes = vec![
            encrypt(&client_key, 7, ScalarWidth::U16),
            encrypt(&client_key, 9, ScalarWidth::U16),
        ];
        let inserted = db.insert_ciphertexts("payments", "note", notes).unwrap();
        assert_eq!(inserted, 2);

        // SUM of `amount`.
        let col = db.column("payments", "amount").unwrap();
        let sum = engine.sum(col, ColumnType::U32).unwrap();
        assert_eq!(sum.count, 3);
        assert_eq!(sum.value.decrypt(&client_key).as_u64(), 60);

        // AVG of `amount` with client-side division.
        let avg = engine.avg(col, ColumnType::U32).unwrap();
        assert_eq!(avg.count, 3);
        let total = avg.value.decrypt(&client_key).as_u64();
        assert_eq!(total / avg.count, 20);

        // COUNT.
        assert_eq!(engine.count(col), 3);

        // Columnar independence: `note` has a different width and count.
        let note_col = db.column("payments", "note").unwrap();
        assert_eq!(engine.count(note_col), 2);
        let note_sum = engine.sum(note_col, ColumnType::U16).unwrap();
        assert_eq!(note_sum.value.decrypt(&client_key).as_u64(), 16);
    }

    #[test]
    fn width_mismatch_rejected_at_insert() {
        let (client_key, mut db) = setup();
        // U64 ciphertext into a U32 column must be rejected without a partial
        // insert.
        let bad = encrypt(&client_key, 1, ScalarWidth::U64);
        let err = db
            .insert_ciphertexts("payments", "amount", vec![bad])
            .unwrap_err();
        assert!(matches!(err, DbError::ColumnWidthMismatch { .. }));
        assert_eq!(db.column("payments", "amount").unwrap().len(), 0);
    }

    #[test]
    fn persistence_roundtrip() {
        let (client_key, mut db) = setup();
        let amounts = vec![
            encrypt(&client_key, 100, ScalarWidth::U32),
            encrypt(&client_key, 200, ScalarWidth::U32),
        ];
        db.insert_ciphertexts("payments", "amount", amounts)
            .unwrap();

        let bytes = db.to_bytes().unwrap();
        let loaded = EncryptedDatabase::from_bytes(&bytes).unwrap();

        let col = loaded.column("payments", "amount").unwrap();
        let engine = QueryEngine::new();
        let sum = engine.sum(col, ColumnType::U32).unwrap();
        assert_eq!(sum.value.decrypt(&client_key).as_u64(), 300);
    }

    #[test]
    fn unknown_table_and_column_errors() {
        let db = EncryptedDatabase::new();
        assert!(matches!(
            db.table_schema("missing"),
            Err(DbError::UnknownTable(_))
        ));
        assert!(matches!(
            db.column("missing", "x"),
            Err(DbError::UnknownTable(_))
        ));
    }
}
