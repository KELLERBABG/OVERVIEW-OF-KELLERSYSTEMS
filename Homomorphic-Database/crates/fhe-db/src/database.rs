//! The encrypted database engine: columnar storage over encrypted columns.
//!
//! FHE-DB is **columnar**: homomorphic aggregation (SUM/AVG/COUNT) operates
//! per column, so each column is an independent [`EncryptedColumn`] of
//! width-homogeneous ciphertexts. A [`Table`] pairs a plaintext
//! [`TableSchema`] (names + types — metadata only) with its encrypted column
//! storage. The database is in-memory and can be persisted to disk with
//! bincode.
//!
//! ## Concurrency contract
//!
//! [`EncryptedDatabase`] is **not** internally synchronized: a single
//! instance must not be mutated by concurrent writers. In the server, one
//! `tokio::sync::RwLock` guards the whole database (see `fhe-server`), so all
//! mutations are serialized and reads run concurrently with each other but
//! never with a write. The unit tests below pin the two data-safety
//! invariants that hold under that contract:
//!
//! - **Batch atomicity** — [`Table::insert_ciphertexts`] validates the whole
//!   batch before writing; a single bad ciphertext leaves the column
//!   untouched (no partial insert).
//! - **Format versioning** — a persisted snapshot from an incompatible build
//!   is rejected rather than silently misread.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use bincode::Options as _;

use fhe_core::{Ciphertext, EncryptedColumn, ScalarWidth};

use crate::error::{DbError, DbResult};
use crate::schema::{ColumnDef, ColumnType, TableSchema};

/// Version of the on-disk database format. Used to reject incompatible files
/// produced by older/newer builds instead of silently misreading them.
const DATABASE_FORMAT_VERSION: u32 = 1;

/// An encrypted table: its plaintext metadata schema plus the encrypted
/// columnar storage.
///
/// Deliberately does **not** derive `Debug`: column values are ciphertexts,
/// which never expose plaintext via debug output.
#[derive(Clone, Serialize, Deserialize)]
pub struct Table {
    /// Column names and types (plaintext metadata; never contains values).
    pub schema: TableSchema,
    /// Column name → encrypted column storage.
    ///
    /// Invariant: exactly one entry per [`TableSchema::columns`] entry, and
    /// each column's `EncryptedColumn` width matches its `ColumnType`.
    pub columns: HashMap<String, EncryptedColumn>,
}

impl Table {
    /// Create a new empty table from the given schema.
    pub fn new(schema: TableSchema) -> Self {
        let columns = schema
            .columns
            .iter()
            .map(|c| {
                (
                    c.name.clone(),
                    EncryptedColumn::new(c.column_type.to_scalar_width()),
                )
            })
            .collect();
        Self { schema, columns }
    }

    /// Add a column definition plus its (empty) encrypted storage.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::ColumnExists`] if the column already exists.
    pub fn add_column(&mut self, column: ColumnDef) -> DbResult<()> {
        if self.schema.column(&column.name).is_some() {
            return Err(DbError::ColumnExists(column.name));
        }
        self.columns.insert(
            column.name.clone(),
            EncryptedColumn::new(column.column_type.to_scalar_width()),
        );
        self.schema.columns.push(column);
        Ok(())
    }

    /// Append ciphertexts to a column.
    ///
    /// The batch is validated **before** any write: if any ciphertext width
    /// mismatches the column's declared type, the column is left untouched
    /// (no partial insert).
    ///
    /// # Errors
    ///
    /// - [`DbError::UnknownColumn`] if the column does not exist in the
    ///   schema.
    /// - [`DbError::ColumnWidthMismatch`] if any ciphertext width differs from
    ///   the column's `ColumnType`.
    pub fn insert_ciphertexts(
        &mut self,
        column: &str,
        ciphertexts: Vec<Ciphertext>,
    ) -> DbResult<usize> {
        let def = self
            .schema
            .column(column)
            .ok_or_else(|| DbError::UnknownColumn {
                table: self.schema.name.clone(),
                column: column.to_string(),
            })?;
        let expected = def.column_type.to_scalar_width();

        // Validate the whole batch up front so a single bad ciphertext cannot
        // leave the column half-written.
        for ct in &ciphertexts {
            if ct.width() != expected {
                return Err(DbError::ColumnWidthMismatch {
                    column: column.to_string(),
                    expected,
                    found: ct.width(),
                });
            }
        }

        let col = self.columns.get_mut(column).ok_or_else(|| {
            // The schema was just used to find the column, so this can only
            // happen if the table was constructed inconsistently.
            DbError::CorruptedDatabase(format!(
                "table `{}` schema references column `{}` but storage is missing it",
                self.schema.name, column
            ))
        })?;

        let mut inserted = 0;
        for ct in ciphertexts {
            // Widths are pre-validated against the column's declared type, so
            // the column-level width enforcement cannot reject these.
            col.push(ct)?;
            inserted += 1;
        }
        Ok(inserted)
    }
}

/// An in-memory encrypted database, optionally persisted as bincode bytes.
///
/// Deliberately does **not** derive `Debug`: the stored ciphertexts never
/// expose plaintext via debug output.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct EncryptedDatabase {
    /// On-disk format version ([`DATABASE_FORMAT_VERSION`]).
    format_version: u32,
    /// Table name → table.
    tables: HashMap<String, Table>,
}

impl EncryptedDatabase {
    /// Create an empty database.
    pub fn new() -> Self {
        Self {
            format_version: DATABASE_FORMAT_VERSION,
            tables: HashMap::new(),
        }
    }

    /// Create a new table from its schema.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::TableExists`] if a table with the same name already
    /// exists.
    pub fn create_table(&mut self, schema: TableSchema) -> DbResult<()> {
        if self.tables.contains_key(&schema.name) {
            return Err(DbError::TableExists(schema.name));
        }
        self.tables.insert(schema.name.clone(), Table::new(schema));
        Ok(())
    }

    /// Add a column to an existing table.
    ///
    /// # Errors
    ///
    /// - [`DbError::UnknownTable`] if the table does not exist.
    /// - [`DbError::ColumnExists`] if the column already exists.
    pub fn add_column(&mut self, table: &str, column: ColumnDef) -> DbResult<()> {
        let table = self
            .tables
            .get_mut(table)
            .ok_or_else(|| DbError::UnknownTable(table.to_string()))?;
        table.add_column(column)
    }

    /// The schema of a single table.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::UnknownTable`] if the table does not exist.
    pub fn table_schema(&self, table: &str) -> DbResult<TableSchema> {
        Ok(self
            .tables
            .get(table)
            .ok_or_else(|| DbError::UnknownTable(table.to_string()))?
            .schema
            .clone())
    }

    /// The `ColumnType` of a column in a table.
    ///
    /// # Errors
    ///
    /// - [`DbError::UnknownTable`] if the table does not exist.
    /// - [`DbError::UnknownColumn`] if the column does not exist.
    pub fn column_type(&self, table: &str, column: &str) -> DbResult<ColumnType> {
        let schema = self.table_schema(table)?;
        let def = schema
            .column(column)
            .ok_or_else(|| DbError::UnknownColumn {
                table: table.to_string(),
                column: column.to_string(),
            })?;
        Ok(def.column_type)
    }

    /// All table schemas, sorted by name for deterministic responses.
    pub fn tables(&self) -> Vec<TableSchema> {
        let mut list: Vec<_> = self.tables.values().map(|t| t.schema.clone()).collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    /// Append ciphertexts to the named column of the named table.
    ///
    /// # Errors
    ///
    /// - [`DbError::UnknownTable`] if the table does not exist.
    /// - [`DbError::UnknownColumn`] if the column does not exist.
    /// - [`DbError::ColumnWidthMismatch`] if any ciphertext width differs from
    ///   the column's `ColumnType`.
    pub fn insert_ciphertexts(
        &mut self,
        table: &str,
        column: &str,
        ciphertexts: Vec<Ciphertext>,
    ) -> DbResult<usize> {
        let table = self
            .tables
            .get_mut(table)
            .ok_or_else(|| DbError::UnknownTable(table.to_string()))?;
        table.insert_ciphertexts(column, ciphertexts)
    }

    /// Borrow an encrypted column for read-only use (e.g. by the query
    /// engine).
    ///
    /// # Errors
    ///
    /// - [`DbError::UnknownTable`] if the table does not exist.
    /// - [`DbError::UnknownColumn`] if the column does not exist.
    pub fn column(&self, table: &str, column: &str) -> DbResult<&EncryptedColumn> {
        let table_ref = self
            .tables
            .get(table)
            .ok_or_else(|| DbError::UnknownTable(table.to_string()))?;
        table_ref
            .columns
            .get(column)
            .ok_or_else(|| DbError::UnknownColumn {
                table: table.to_string(),
                column: column.to_string(),
            })
    }

    /// The declared width of a table's column, if any rows exist yet.
    ///
    /// # Errors
    ///
    /// - [`DbError::UnknownTable`] if the table does not exist.
    /// - [`DbError::UnknownColumn`] if the column does not exist.
    pub fn column_width(&self, table: &str, column: &str) -> DbResult<Option<ScalarWidth>> {
        Ok(self.column(table, column)?.width())
    }

    /// Serialize the whole database to bincode bytes for storage.
    pub fn to_bytes(&self) -> DbResult<Vec<u8>> {
        bincode::serialize(self).map_err(Into::into)
    }

    /// Deserialize a database from bincode bytes.
    ///
    /// The decode is bounded by the input length so a corrupt or crafted
    /// snapshot cannot force bincode/serde to preallocate beyond the
    /// supplied bytes.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::CorruptedDatabase`] if the bytes do not decode to a
    /// database or carry an unsupported format version.
    pub fn from_bytes(bytes: &[u8]) -> DbResult<Self> {
        let db: Self = bincode::options()
            .with_fixint_encoding()
            .with_limit(bytes.len() as u64)
            .reject_trailing_bytes()
            .deserialize(bytes)?;
        if db.format_version != DATABASE_FORMAT_VERSION {
            return Err(DbError::CorruptedDatabase(format!(
                "unsupported database format version {} (expected {})",
                db.format_version, DATABASE_FORMAT_VERSION
            )));
        }
        Ok(db)
    }

    /// Persist the database to a file, creating parent directories if needed.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Io`] on filesystem errors and
    /// [`DbError::CorruptedDatabase`] if serialization fails.
    pub fn save(&self, path: impl AsRef<Path>) -> DbResult<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let bytes = self.to_bytes()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Load a database from a file.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Io`] if the file cannot be read and
    /// [`DbError::CorruptedDatabase`] if the contents are invalid or carry an
    /// unsupported format version.
    pub fn load(path: impl AsRef<Path>) -> DbResult<Self> {
        let bytes = std::fs::read(path.as_ref())?;
        Self::from_bytes(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsupported_format_version() {
        let mut db = EncryptedDatabase::new();
        db.format_version = 999;
        let bytes = bincode::serialize(&db).unwrap();
        assert!(matches!(
            EncryptedDatabase::from_bytes(&bytes),
            Err(DbError::CorruptedDatabase(_))
        ));
    }

    #[test]
    fn new_database_has_current_version() {
        let mut db = EncryptedDatabase::new();
        assert!(db.format_version == DATABASE_FORMAT_VERSION);
        // Ensure the field survives a save/load roundtrip.
        db.format_version = DATABASE_FORMAT_VERSION;
        let bytes = db.to_bytes().unwrap();
        let loaded = EncryptedDatabase::from_bytes(&bytes).unwrap();
        assert!(loaded.format_version == DATABASE_FORMAT_VERSION);
    }

    #[test]
    fn table_columns_are_precreated() {
        let schema = TableSchema::new("t")
            .with_column(ColumnDef {
                name: "a".into(),
                column_type: ColumnType::U8,
            })
            .with_column(ColumnDef {
                name: "b".into(),
                column_type: ColumnType::U64,
            });
        let table = Table::new(schema);
        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.columns["a"].width(), Some(ScalarWidth::U8));
        assert_eq!(table.columns["b"].width(), Some(ScalarWidth::U64));
    }

    /// Unique temp path per test process so parallel tests never collide.
    fn temp_path(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("fhe_db_persistence_{}_{}", tag, std::process::id()))
    }

    #[test]
    fn save_and_load_file_roundtrip() {
        let path = temp_path("roundtrip");
        // Clean any stale file from a previous run.
        let _ = std::fs::remove_file(&path);

        let mut db = EncryptedDatabase::new();
        let schema = TableSchema::new("t").with_column(ColumnDef {
            name: "c".into(),
            column_type: ColumnType::U32,
        });
        db.create_table(schema).unwrap();
        db.save(&path).expect("save to file");

        let loaded = EncryptedDatabase::load(&path).expect("load from file");
        assert_eq!(loaded.tables().len(), 1);
        assert_eq!(loaded.tables()[0].name, "t");
        assert_eq!(loaded.tables()[0].columns.len(), 1);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_rejects_corrupt_file() {
        let path = temp_path("corrupt");
        std::fs::write(&path, b"this is not a database snapshot").unwrap();

        // `EncryptedDatabase` deliberately has no `Debug`, so match instead
        // of `unwrap_err`.
        let err = match EncryptedDatabase::load(&path) {
            Ok(_) => panic!("corrupt file must fail to load"),
            Err(e) => e,
        };
        assert!(matches!(err, DbError::CorruptedDatabase(_)));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn batch_insert_is_all_or_nothing() {
        // A batch where one ciphertext has the wrong width must reject the
        // entire batch: no partial insert.
        let (client_key, _server_key) = fhe_core::ClientKey::new();
        let encryptor = fhe_core::Encryptor::new(&client_key);
        let mut db = EncryptedDatabase::new();
        db.create_table(TableSchema::new("t").with_column(ColumnDef {
            name: "c".into(),
            column_type: ColumnType::U32,
        }))
        .unwrap();

        let good = encryptor.encrypt_u64(1, ScalarWidth::U32).unwrap();
        let bad = encryptor.encrypt_u64(2, ScalarWidth::U64).unwrap();
        let err = db
            .insert_ciphertexts("t", "c", vec![good, bad])
            .unwrap_err();
        assert!(matches!(err, DbError::ColumnWidthMismatch { .. }));
        assert_eq!(db.column("t", "c").unwrap().len(), 0);
    }
}
