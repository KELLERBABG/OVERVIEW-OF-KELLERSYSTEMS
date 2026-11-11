//! Table and schema definitions for the encrypted database.
//!
//! FHE-DB is a **columnar** database: rows are scattered across width-homogeneous
//! columns because homomorphic aggregation (SUM/AVG/COUNT) operates per column.
//!
//! A [`TableSchema`] declares the columns of a table. Column *names* are kept
//! in plaintext (they are metadata, not data); only the *values* are encrypted.

use serde::{Deserialize, Serialize};

/// The column type (arithmetic width) of an encrypted column.
///
/// Mirrors [`fhe_core::ScalarWidth`] but is intentionally a separate type so
/// that the schema format is stable and independent of the FHE core's internal
/// representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColumnType {
    /// 8-bit unsigned integer
    U8,
    /// 16-bit unsigned integer
    U16,
    /// 32-bit unsigned integer
    U32,
    /// 64-bit unsigned integer
    U64,
}

impl std::fmt::Display for ColumnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ColumnType::U8 => "U8",
            ColumnType::U16 => "U16",
            ColumnType::U32 => "U32",
            ColumnType::U64 => "U64",
        };
        f.write_str(name)
    }
}

impl ColumnType {
    /// Number of bits in this column type.
    pub fn bits(self) -> usize {
        match self {
            ColumnType::U8 => 8,
            ColumnType::U16 => 16,
            ColumnType::U32 => 32,
            ColumnType::U64 => 64,
        }
    }

    /// Map to the FHE core's width representation.
    pub fn to_scalar_width(self) -> fhe_core::ScalarWidth {
        match self {
            ColumnType::U8 => fhe_core::ScalarWidth::U8,
            ColumnType::U16 => fhe_core::ScalarWidth::U16,
            ColumnType::U32 => fhe_core::ScalarWidth::U32,
            ColumnType::U64 => fhe_core::ScalarWidth::U64,
        }
    }

    /// Map from the FHE core's width representation.
    pub fn from_scalar_width(width: fhe_core::ScalarWidth) -> Self {
        match width {
            fhe_core::ScalarWidth::U8 => ColumnType::U8,
            fhe_core::ScalarWidth::U16 => ColumnType::U16,
            fhe_core::ScalarWidth::U32 => ColumnType::U32,
            fhe_core::ScalarWidth::U64 => ColumnType::U64,
        }
    }
}

/// A single column definition in a table schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnDef {
    /// Column name (plaintext metadata).
    pub name: String,
    /// Column value type / width.
    pub column_type: ColumnType,
}

/// The schema of a table: its name and ordered column definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableSchema {
    /// Table name (plaintext metadata).
    pub name: String,
    /// Ordered column definitions.
    pub columns: Vec<ColumnDef>,
}

impl TableSchema {
    /// Create a new schema with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: Vec::new(),
        }
    }

    /// Add a column definition.
    pub fn with_column(mut self, column: ColumnDef) -> Self {
        self.columns.push(column);
        self
    }

    /// Look up a column definition by name.
    pub fn column(&self, name: &str) -> Option<&ColumnDef> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// Validate a set of column names against this schema, returning a
    /// descriptive error on the first mismatch.
    pub fn validate(&self, names: &[String]) -> Result<(), SchemaError> {
        for name in names {
            if self.column(name).is_none() {
                return Err(SchemaError::UnknownColumn {
                    table: self.name.clone(),
                    column: name.clone(),
                });
            }
        }
        Ok(())
    }
}

/// Errors produced while working with schemas.
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    /// A query referenced a column that does not exist in the table schema.
    #[error("unknown column `{column}` in table `{table}`")]
    UnknownColumn { table: String, column: String },

    /// A column value was supplied with a mismatched type.
    #[error("column `{column}` expects {expected}, got {found}")]
    TypeMismatch {
        column: String,
        expected: ColumnType,
        found: ColumnType,
    },
}
