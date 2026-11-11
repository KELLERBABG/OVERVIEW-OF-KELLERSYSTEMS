//! Client-side encryption of plaintext values.
//!
//! [`Encryptor`] is the single entry point for turning plaintext integers
//! into [`Ciphertext`]s. It holds a reference to the secret [`ClientKey`]
//! and validates every value against the requested arithmetic width before
//! encrypting, so out-of-range values are rejected **before** any expensive
//! FHE work happens.

use tfhe::prelude::*;
use tfhe::{FheUint16, FheUint32, FheUint64, FheUint8};

use crate::ciphertext::Ciphertext;
use crate::error::{FheError, FheResult};
use crate::keys::ClientKey;
use crate::ScalarWidth;

/// Client-side encryptor.
///
/// Construct it once per key and reuse it for many values; it is cheap to
/// clone the key inside if you need to pass it around.
///
/// # Example
///
/// ```
/// use fhe_core::{ClientKey, Encryptor, ScalarWidth};
///
/// let (client_key, _server_key) = ClientKey::new();
/// let encryptor = Encryptor::new(&client_key);
///
/// let ct = encryptor.encrypt_u64(9001, ScalarWidth::U32).unwrap();
/// assert_eq!(ct.decrypt(&client_key).as_u64(), 9001);
/// ```
#[derive(Clone)]
pub struct Encryptor {
    client_key: ClientKey,
}

impl Encryptor {
    /// Create an encryptor bound to the given client key.
    pub fn new(client_key: &ClientKey) -> Self {
        Self {
            client_key: client_key.clone(),
        }
    }

    /// Encrypt a `u8` value.
    ///
    /// # Errors
    ///
    /// Returns [`FheError::ValueOutOfRange`] if `value` does not fit in the
    /// requested width.
    pub fn encrypt_u8(&self, value: u8, width: ScalarWidth) -> FheResult<Ciphertext> {
        if u64::from(value) > width.max_value() {
            return Err(FheError::ValueOutOfRange {
                value: u64::from(value),
                width: width.bits(),
                max: width.max_value(),
            });
        }
        let ct = self.encrypt_u64_as(width, u64::from(value))?;
        Ok(ct)
    }

    /// Encrypt a `u16` value.
    ///
    /// # Errors
    ///
    /// Returns [`FheError::ValueOutOfRange`] if `value` does not fit in the
    /// requested width.
    pub fn encrypt_u16(&self, value: u16, width: ScalarWidth) -> FheResult<Ciphertext> {
        if u64::from(value) > width.max_value() {
            return Err(FheError::ValueOutOfRange {
                value: u64::from(value),
                width: width.bits(),
                max: width.max_value(),
            });
        }
        self.encrypt_u64_as(width, u64::from(value))
    }

    /// Encrypt a `u32` value.
    ///
    /// # Errors
    ///
    /// Returns [`FheError::ValueOutOfRange`] if `value` does not fit in the
    /// requested width.
    pub fn encrypt_u32(&self, value: u32, width: ScalarWidth) -> FheResult<Ciphertext> {
        if u64::from(value) > width.max_value() {
            return Err(FheError::ValueOutOfRange {
                value: u64::from(value),
                width: width.bits(),
                max: width.max_value(),
            });
        }
        self.encrypt_u64_as(width, u64::from(value))
    }

    /// Encrypt a `u64` value.
    ///
    /// # Errors
    ///
    /// Returns [`FheError::ValueOutOfRange`] if `value` does not fit in the
    /// requested width.
    pub fn encrypt_u64(&self, value: u64, width: ScalarWidth) -> FheResult<Ciphertext> {
        if value > width.max_value() {
            return Err(FheError::ValueOutOfRange {
                value,
                width: width.bits(),
                max: width.max_value(),
            });
        }
        self.encrypt_u64_as(width, value)
    }

    /// Internal: dispatch to the concrete TFHE-rs type for the width.
    fn encrypt_u64_as(&self, width: ScalarWidth, value: u64) -> FheResult<Ciphertext> {
        let client_key = &self.client_key;
        let ct: Ciphertext = match width {
            ScalarWidth::U8 => {
                let plain = value as u8;
                Ciphertext::U8(
                    FheUint8::try_encrypt(plain, &client_key.inner)
                        .map_err(|e| FheError::Tfhe(e.to_string()))?,
                )
            }
            ScalarWidth::U16 => {
                let plain = value as u16;
                Ciphertext::U16(
                    FheUint16::try_encrypt(plain, &client_key.inner)
                        .map_err(|e| FheError::Tfhe(e.to_string()))?,
                )
            }
            ScalarWidth::U32 => {
                let plain = value as u32;
                Ciphertext::U32(
                    FheUint32::try_encrypt(plain, &client_key.inner)
                        .map_err(|e| FheError::Tfhe(e.to_string()))?,
                )
            }
            ScalarWidth::U64 => {
                let plain = value;
                Ciphertext::U64(
                    FheUint64::try_encrypt(plain, &client_key.inner)
                        .map_err(|e| FheError::Tfhe(e.to_string()))?,
                )
            }
        };
        Ok(ct)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypts_all_widths() {
        let (client_key, _server_key) = ClientKey::new();
        let encryptor = Encryptor::new(&client_key);
        for (value, width) in [
            (7u64, ScalarWidth::U8),
            (1000u64, ScalarWidth::U16),
            (500_000_000u64, ScalarWidth::U32),
            (10_000_000_000_000_000_000u64, ScalarWidth::U64),
        ] {
            let ct = encryptor.encrypt_u64(value, width).unwrap();
            assert_eq!(ct.decrypt(&client_key).as_u64(), value);
        }
    }

    #[test]
    fn rejects_out_of_range() {
        let (client_key, _server_key) = ClientKey::new();
        let encryptor = Encryptor::new(&client_key);
        let result = encryptor.encrypt_u64(300, ScalarWidth::U8);
        assert!(matches!(result, Err(FheError::ValueOutOfRange { .. })));
        let result = encryptor.encrypt_u64(1 << 16, ScalarWidth::U16);
        assert!(matches!(result, Err(FheError::ValueOutOfRange { .. })));
    }

    #[test]
    fn encryption_is_randomized() {
        let (client_key, _server_key) = ClientKey::new();
        let encryptor = Encryptor::new(&client_key);
        let a = encryptor.encrypt_u64(42, ScalarWidth::U16).unwrap();
        let b = encryptor.encrypt_u64(42, ScalarWidth::U16).unwrap();

        let a_bytes = bincode::serialize(&a).unwrap();
        let b_bytes = bincode::serialize(&b).unwrap();
        assert_ne!(a_bytes, b_bytes, "encryption must be randomized");
    }
}
