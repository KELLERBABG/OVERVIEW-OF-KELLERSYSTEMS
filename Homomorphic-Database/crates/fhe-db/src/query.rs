//! Query engine: homomorphic SUM / AVG / COUNT over encrypted columns.
//!
//! The server evaluates these queries on ciphertexts only. SUM and the
//! sum-component of AVG are computed homomorphically with
//! [`fhe_core::CiphertextAlgebra`]. Division (for AVG) happens client-side
//! after decryption, because homomorphic division would require much deeper
//! circuits — this is the standard FHE approach.

use fhe_core::{Ciphertext, CiphertextAlgebra, ColumnAggregate, EncryptedColumn};

use crate::error::{DbError, DbResult};
use crate::schema::ColumnType;

/// Result of a homomorphic aggregation over a column.
///
/// Deliberately does **not** derive `Debug`: `value` is a ciphertext, which
/// never exposes plaintext via debug output.
#[derive(Clone)]
pub struct AggregateResult {
    /// The encrypted aggregate value (SUM, or the sum component of AVG).
    pub value: Ciphertext,
    /// Number of rows aggregated. Public metadata, not sensitive.
    pub count: u64,
    /// The column's declared type (width of `value`).
    pub column_type: ColumnType,
}

/// Executes homomorphic aggregations over encrypted columns.
///
/// All operations require the TFHE server key to be installed for the current
/// thread; callers must wrap computation in a
/// [`fhe_core::ServerKeyGuard`].
#[derive(Clone, Copy, Debug, Default)]
pub struct QueryEngine {
    algebra: CiphertextAlgebra,
}

impl QueryEngine {
    /// Create a query engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Homomorphic SUM over a column.
    ///
    /// # Errors
    ///
    /// - [`DbError::EmptyColumn`] if the column has no rows.
    /// - [`DbError::Fhe`] if the underlying FHE layer rejects the operation
    ///   (e.g. no server key installed).
    pub fn sum(
        &self,
        column: &EncryptedColumn,
        column_type: ColumnType,
    ) -> DbResult<AggregateResult> {
        let count = column.len() as u64;
        let value = self.algebra.sum_column(column).map_err(map_fhe_err)?;
        Ok(AggregateResult {
            value,
            count,
            column_type,
        })
    }

    /// Homomorphic AVG over a column.
    ///
    /// The returned `value` is the encrypted **sum**; the client divides by
    /// `count` after decryption.
    ///
    /// # Errors
    ///
    /// - [`DbError::EmptyColumn`] if the column has no rows.
    /// - [`DbError::Fhe`] if the underlying FHE layer rejects the operation
    ///   (e.g. no server key installed).
    pub fn avg(
        &self,
        column: &EncryptedColumn,
        column_type: ColumnType,
    ) -> DbResult<AggregateResult> {
        let ColumnAggregate { sum, count, .. } =
            self.algebra.average_column(column).map_err(map_fhe_err)?;
        Ok(AggregateResult {
            value: sum,
            count,
            column_type,
        })
    }

    /// Public COUNT over a column.
    ///
    /// Row counts are public metadata — this returns the number of encrypted
    /// rows without doing any FHE work.
    pub fn count(&self, column: &EncryptedColumn) -> u64 {
        column.len() as u64
    }
}

fn map_fhe_err(e: fhe_core::FheError) -> DbError {
    match e {
        fhe_core::FheError::EmptyColumn => DbError::EmptyColumn("aggregation".to_string()),
        other => DbError::Fhe(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fhe_core::{ClientKey, Encryptor, ScalarWidth, ServerKeyGuard};

    fn setup_u32(values: &[u64]) -> (ClientKey, EncryptedColumn, ColumnType) {
        let (client_key, server_key) = ClientKey::new();
        let _guard = ServerKeyGuard::new(&server_key);
        let encryptor = Encryptor::new(&client_key);
        let mut col = EncryptedColumn::new(ScalarWidth::U32);
        for v in values {
            let ct = encryptor.encrypt_u64(*v, ScalarWidth::U32).unwrap();
            col.push(ct).unwrap();
        }
        (client_key, col, ColumnType::U32)
    }

    #[test]
    fn sum_is_homomorphic() {
        let (client_key, col, col_type) = setup_u32(&[10, 20, 30, 40]);
        let engine = QueryEngine::new();
        let result = engine.sum(&col, col_type).unwrap();
        assert_eq!(result.count, 4);
        assert_eq!(result.value.decrypt(&client_key).as_u64(), 100);
    }

    #[test]
    fn avg_returns_sum_and_count() {
        let (client_key, col, col_type) = setup_u32(&[10, 20, 30, 40]);
        let engine = QueryEngine::new();
        let result = engine.avg(&col, col_type).unwrap();
        assert_eq!(result.count, 4);
        let sum = result.value.decrypt(&client_key).as_u64();
        assert_eq!(sum, 100);
        assert_eq!(sum / result.count, 25);
    }

    #[test]
    fn count_is_public() {
        let (_, col, col_type) = setup_u32(&[1, 2, 3]);
        let engine = QueryEngine::new();
        assert_eq!(engine.count(&col), 3);
        assert_eq!(engine.sum(&col, col_type).unwrap().count, 3);
    }

    #[test]
    fn empty_column_returns_empty_error() {
        let col = EncryptedColumn::new(ScalarWidth::U32);
        let engine = QueryEngine::new();
        assert!(matches!(
            engine.sum(&col, ColumnType::U32),
            Err(DbError::EmptyColumn(_))
        ));
        assert!(matches!(
            engine.avg(&col, ColumnType::U32),
            Err(DbError::EmptyColumn(_))
        ));
        assert_eq!(engine.count(&col), 0);
    }

    #[test]
    fn sum_single_row() {
        let (client_key, col, col_type) = setup_u32(&[42]);
        let engine = QueryEngine::new();
        let result = engine.sum(&col, col_type).unwrap();
        assert_eq!(result.count, 1);
        assert_eq!(result.value.decrypt(&client_key).as_u64(), 42);
    }
}
