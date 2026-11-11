//! Errors produced by the encrypted database engine.

use fhe_core::FheError;

/// Errors that can occur while operating on the encrypted database.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// The referenced table does not exist.
    #[error("unknown table `{0}`")]
    UnknownTable(String),

    /// A table with the same name already exists.
    #[error("table `{0}` already exists")]
    TableExists(String),

    /// The referenced column does not exist in the table schema.
    #[error("unknown column `{column}` in table `{table}`")]
    UnknownColumn { table: String, column: String },

    /// A column with the same name already exists in the table schema.
    #[error("column `{0}` already exists")]
    ColumnExists(String),

    /// Ciphertext width does not match the column's declared type.
    #[error("column `{column}` expects {expected:?}, got {found:?}")]
    ColumnWidthMismatch {
        column: String,
        expected: fhe_core::ScalarWidth,
        found: fhe_core::ScalarWidth,
    },

    /// An operation was attempted on an empty column.
    #[error("column `{0}` is empty")]
    EmptyColumn(String),

    /// The on-disk database file does not match the expected format.
    #[error("corrupted database file: {0}")]
    CorruptedDatabase(String),

    /// I/O failure while reading or writing the database file.
    #[error("I/O error: {0}")]
    Io(String),

    /// An FHE-layer error propagated from `fhe-core`.
    #[error("FHE error: {0}")]
    Fhe(#[from] FheError),
}

impl From<bincode::Error> for DbError {
    fn from(e: bincode::Error) -> Self {
        DbError::CorruptedDatabase(e.to_string())
    }
}

impl From<std::io::Error> for DbError {
    fn from(e: std::io::Error) -> Self {
        DbError::Io(e.to_string())
    }
}

/// Convenience result alias for the encrypted database engine.
pub type DbResult<T> = Result<T, DbError>;
