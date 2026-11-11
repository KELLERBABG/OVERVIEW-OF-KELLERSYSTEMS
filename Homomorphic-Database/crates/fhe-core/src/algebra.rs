//! Homomorphic algebra over ciphertexts.
//!
//! These are the operations that make the database "fully homomorphic":
//! they run on the **server** on encrypted data, without ever seeing the
//! plaintext.
//!
//! See [`CiphertextAlgebra`].

use crate::ciphertext::{Ciphertext, EncryptedColumn};
use crate::error::{FheError, FheResult};
use crate::ScalarWidth;

/// Server-side homomorphic operations on [`Ciphertext`]s.
#[derive(Clone, Copy, Debug, Default)]
pub struct CiphertextAlgebra;

impl CiphertextAlgebra {
    /// Homomorphic addition. Returns `a + b` (mod `2^width`).
    ///
    /// # Errors
    ///
    /// - [`FheError::WidthMismatch`] if the widths differ.
    /// - [`FheError::MissingServerKey`] if no server key is installed.
    pub fn add(&self, a: &Ciphertext, b: &Ciphertext) -> FheResult<Ciphertext> {
        ensure_same_width(a, b)?;
        Ok(match (a, b) {
            (Ciphertext::U8(x), Ciphertext::U8(y)) => Ciphertext::U8(x + y),
            (Ciphertext::U16(x), Ciphertext::U16(y)) => Ciphertext::U16(x + y),
            (Ciphertext::U32(x), Ciphertext::U32(y)) => Ciphertext::U32(x + y),
            (Ciphertext::U64(x), Ciphertext::U64(y)) => Ciphertext::U64(x + y),
            _ => unreachable!("width match checked above"),
        })
    }

    /// Homomorphic multiplication. Returns `a * b` (mod `2^width`).
    ///
    /// # Errors
    ///
    /// - [`FheError::WidthMismatch`] if the widths differ.
    /// - [`FheError::MissingServerKey`] if no server key is installed.
    pub fn mul(&self, a: &Ciphertext, b: &Ciphertext) -> FheResult<Ciphertext> {
        ensure_same_width(a, b)?;
        Ok(match (a, b) {
            (Ciphertext::U8(x), Ciphertext::U8(y)) => Ciphertext::U8(x * y),
            (Ciphertext::U16(x), Ciphertext::U16(y)) => Ciphertext::U16(x * y),
            (Ciphertext::U32(x), Ciphertext::U32(y)) => Ciphertext::U32(x * y),
            (Ciphertext::U64(x), Ciphertext::U64(y)) => Ciphertext::U64(x * y),
            _ => unreachable!("width match checked above"),
        })
    }

    /// Homomorphic sum of an entire [`EncryptedColumn`].
    ///
    /// Returns the encrypted sum `v[0] + v[1] + ... + v[n-1]` (mod
    /// `2^width`).
    ///
    /// # Errors
    ///
    /// - [`FheError::EmptyColumn`] if the column is empty.
    /// - [`FheError::MissingServerKey`] if no server key is installed.
    pub fn sum_column(&self, column: &EncryptedColumn) -> FheResult<Ciphertext> {
        match column.values.first() {
            Some(first) => {
                let mut acc = first.clone();
                for value in &column.values[1..] {
                    acc = self.add(&acc, value)?;
                }
                Ok(acc)
            }
            None => Err(FheError::EmptyColumn),
        }
    }

    /// Homomorphic sum of a slice of ciphertexts.
    ///
    /// Returns the encrypted sum `v[0] + ... + v[n-1]` (mod `2^width`). All
    /// ciphertexts must share the same width.
    ///
    /// # Errors
    ///
    /// - [`FheError::EmptySlice`] if the slice is empty.
    /// - [`FheError::WidthMismatch`] if widths differ.
    /// - [`FheError::MissingServerKey`] if no server key is installed.
    pub fn sum(&self, values: &[Ciphertext]) -> FheResult<Ciphertext> {
        let first = values.first().ok_or(FheError::EmptySlice)?;
        let mut acc = first.clone();
        for value in &values[1..] {
            acc = self.add(&acc, value)?;
        }
        Ok(acc)
    }

    /// Homomorphic dot product of two ciphertext vectors.
    ///
    /// Computes `a[0]*b[0] + ... + a[n-1]*b[n-1]` (mod `2^width`). This is
    /// the core primitive behind fully-connected layers: one operand is the
    /// encrypted input vector, the other is the encrypted (obfuscated)
    /// weight vector.
    ///
    /// # Errors
    ///
    /// - [`FheError::EmptySlice`] if either vector is empty.
    /// - [`FheError::LengthMismatch`] if the vectors have different lengths.
    /// - [`FheError::WidthMismatch`] if a width mismatch is detected during
    ///   the multiplication/summation.
    /// - [`FheError::MissingServerKey`] if no server key is installed.
    pub fn dot_product(&self, a: &[Ciphertext], b: &[Ciphertext]) -> FheResult<Ciphertext> {
        if a.is_empty() || b.is_empty() {
            return Err(FheError::EmptySlice);
        }
        if a.len() != b.len() {
            return Err(FheError::LengthMismatch {
                expected: a.len(),
                found: b.len(),
            });
        }
        let mut products = Vec::with_capacity(a.len());
        for (x, w) in a.iter().zip(b.iter()) {
            products.push(self.mul(x, w)?);
        }
        self.sum(&products)
    }

    /// Homomorphic elementwise sum of two ciphertext vectors.
    ///
    /// # Errors
    ///
    /// - [`FheError::EmptySlice`] if either vector is empty.
    /// - [`FheError::LengthMismatch`] if the vectors have different lengths.
    /// - [`FheError::WidthMismatch`] if widths differ.
    pub fn add_vectors(&self, a: &[Ciphertext], b: &[Ciphertext]) -> FheResult<Vec<Ciphertext>> {
        if a.is_empty() || b.is_empty() {
            return Err(FheError::EmptySlice);
        }
        if a.len() != b.len() {
            return Err(FheError::LengthMismatch {
                expected: a.len(),
                found: b.len(),
            });
        }
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| self.add(x, y))
            .collect()
    }

    /// Add a plaintext scalar to a ciphertext.
    ///
    /// This is a *trivial* (keyless) operation: adding a known constant to an
    /// encrypted value requires no homomorphic machinery. It is the standard
    /// way to fold a bias term into an encrypted neuron. The scalar is
    /// interpreted modulo `2^width`.
    ///
    /// # Errors
    ///
    /// - [`FheError::WidthMismatch`] if the scalar does not fit the
    ///   ciphertext width.
    pub fn add_scalar(&self, ct: &Ciphertext, scalar: u64) -> FheResult<Ciphertext> {
        let width = ct.width();
        if scalar > width.max_value() {
            return Err(FheError::ValueOutOfRange {
                value: scalar,
                width: width.bits(),
                max: width.max_value(),
            });
        }
        Ok(match ct {
            Ciphertext::U8(x) => Ciphertext::U8(x + (scalar as u8)),
            Ciphertext::U16(x) => Ciphertext::U16(x + (scalar as u16)),
            Ciphertext::U32(x) => Ciphertext::U32(x + (scalar as u32)),
            Ciphertext::U64(x) => Ciphertext::U64(x + scalar),
        })
    }

    /// Multiply a ciphertext by a plaintext scalar.
    ///
    /// A trivial (keyless) multiplication: scaling an encrypted value by a
    /// known constant. Used to fold learned scale factors into encrypted
    /// weights and to apply polynomial coefficients.
    ///
    /// # Errors
    ///
    /// - [`FheError::WidthMismatch`] if the scalar does not fit the
    ///   ciphertext width.
    pub fn mul_scalar(&self, ct: &Ciphertext, scalar: u64) -> FheResult<Ciphertext> {
        let width = ct.width();
        if scalar > width.max_value() {
            return Err(FheError::ValueOutOfRange {
                value: scalar,
                width: width.bits(),
                max: width.max_value(),
            });
        }
        Ok(match ct {
            Ciphertext::U8(x) => Ciphertext::U8(x * (scalar as u8)),
            Ciphertext::U16(x) => Ciphertext::U16(x * (scalar as u16)),
            Ciphertext::U32(x) => Ciphertext::U32(x * (scalar as u32)),
            Ciphertext::U64(x) => Ciphertext::U64(x * scalar),
        })
    }

    /// Two's-complement negation of a ciphertext.
    ///
    /// Computes `-x` (mod `2^width`), the building block for polynomial
    /// activations that need subtraction (e.g. `(x-c)^2`).
    ///
    /// # Errors
    ///
    /// - [`FheError::MissingServerKey`] if no server key is installed.
    pub fn negate(&self, ct: &Ciphertext) -> FheResult<Ciphertext> {
        Ok(match ct {
            Ciphertext::U8(x) => Ciphertext::U8(-x),
            Ciphertext::U16(x) => Ciphertext::U16(-x),
            Ciphertext::U32(x) => Ciphertext::U32(-x),
            Ciphertext::U64(x) => Ciphertext::U64(-x),
        })
    }

    /// Homomorphic square of a ciphertext.
    ///
    /// `x*x` (mod `2^width`).
    ///
    /// # Errors
    ///
    /// - [`FheError::MissingServerKey`] if no server key is installed.
    pub fn square(&self, ct: &Ciphertext) -> FheResult<Ciphertext> {
        self.mul(ct, ct)
    }

    /// Homomorphic cube of a ciphertext.
    ///
    /// `x*x*x` (mod `2^width`).
    ///
    /// # Errors
    ///
    /// - [`FheError::MissingServerKey`] if no server key is installed.
    pub fn cube(&self, ct: &Ciphertext) -> FheResult<Ciphertext> {
        let sq = self.square(ct)?;
        self.mul(&sq, ct)
    }

    /// Homomorphic integer power `base^exponent` (mod `2^width`).
    ///
    /// Uses exponentiation by squaring, so the circuit depth is
    /// `O(log2(exponent))`. `exponent == 0` is an error: the constant-1 term
    /// of a polynomial is folded by the caller with [`Self::add_scalar`]
    /// rather than manufacturing a ciphertext.
    ///
    /// # Errors
    ///
    /// - [`FheError::UnsupportedOperation`] if `exponent == 0`.
    /// - [`FheError::MissingServerKey`] if no server key is installed.
    pub fn power(&self, base: &Ciphertext, exponent: usize) -> FheResult<Ciphertext> {
        match exponent {
            0 => Err(FheError::UnsupportedOperation(base.width().bits())),
            1 => Ok(base.clone()),
            2 => self.square(base),
            3 => self.cube(base),
            _ => {
                let mut result: Option<Ciphertext> = None;
                let mut acc = base.clone();
                let mut e = exponent;
                while e > 0 {
                    if e & 1 == 1 {
                        result = Some(match result {
                            None => acc.clone(),
                            Some(r) => self.mul(&r, &acc)?,
                        });
                    }
                    e >>= 1;
                    if e > 0 {
                        acc = self.square(&acc)?;
                    }
                }
                // exponent >= 4 guarantees at least one set bit contributes.
                Ok(result.expect("exponent >= 4 always sets result"))
            }
        }
    }

    /// Homomorphic average of an entire [`EncryptedColumn`].
    ///
    /// The **sum** is computed homomorphically. The division by the (public)
    /// row count is performed on the client side after decryption, because
    /// homomorphic division would require much deeper circuits. This is the
    /// standard FHE approach.
    ///
    /// The returned [`ColumnAggregate`] pairs the encrypted sum with the row
    /// count so the client can decrypt and divide exactly.
    ///
    /// # Errors
    ///
    /// - [`FheError::EmptyColumn`] if the column is empty.
    /// - [`FheError::MissingServerKey`] if no server key is installed.
    pub fn average_column(&self, column: &EncryptedColumn) -> FheResult<ColumnAggregate> {
        let count = column.len() as u64;
        let sum = self.sum_column(column)?;
        let width = sum.width();
        Ok(ColumnAggregate { sum, count, width })
    }
}

fn ensure_same_width(a: &Ciphertext, b: &Ciphertext) -> FheResult<()> {
    if a.width() != b.width() {
        return Err(FheError::WidthMismatch {
            expected: a.width().bits(),
            found: b.width().bits(),
        });
    }
    Ok(())
}

/// Result of a homomorphic aggregation over a column.
pub struct ColumnAggregate {
    /// Encrypted sum of the column.
    pub sum: Ciphertext,
    /// Number of rows aggregated (public, not sensitive).
    pub count: u64,
    /// Width of the sum ciphertext.
    pub width: ScalarWidth,
}
