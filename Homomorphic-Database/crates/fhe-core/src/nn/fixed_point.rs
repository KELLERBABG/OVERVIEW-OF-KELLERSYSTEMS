//! Signed fixed-point encoding for FHE-friendly neural computation.
//!
//! TFHE-rs' integer types are unsigned, but neural-network weights and
//! activations are signed. The standard trick is to represent a real number
//! `v` as a fixed-point integer `q = round(v * 2^m)` (with `m` fractional
//! bits) and then store `q` **modulo `2^width`** in two's-complement form.
//! Modular arithmetic makes this exact: adding two encoded values is the
//! same as adding their signed values, and multiplying two encoded values
//! yields the correctly-reduced signed product (as long as the true product
//! fits the signed range — the usual fixed-point constraint every FHE NN
//! must respect).
//!
//! The client uses [`FixedPoint::encode`] before encrypting and
//! [`FixedPoint::decode_plaintext`] after decrypting. The server never
//! decodes; it only manipulates ciphertexts.

use crate::error::{FheError, FheResult};
use crate::{PlaintextValue, ScalarWidth};

/// A signed fixed-point format: `Q(width-1).frac_bits`.
///
/// One bit is reserved for the sign (two's complement), leaving
/// `width - 1 - frac_bits` integer bits. Choose `width` large enough that
/// all intermediate products fit the signed range, exactly as you would when
/// scaling a plaintext network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedPoint {
    /// Storage width of the encoded integer.
    pub width: ScalarWidth,
    /// Number of fractional bits (`m` in `Qm.n`).
    pub frac_bits: u32,
}

impl FixedPoint {
    /// Create a fixed-point format.
    ///
    /// # Errors
    ///
    /// Returns [`FheError::ValueOutOfRange`] if `frac_bits >= width.bits()`
    /// (there would be no integer bit left).
    pub fn new(width: ScalarWidth, frac_bits: u32) -> FheResult<Self> {
        if frac_bits >= width.bits() as u32 {
            return Err(FheError::ValueOutOfRange {
                value: u64::from(frac_bits),
                width: width.bits(),
                max: (width.bits() as u64) - 1,
            });
        }
        Ok(Self { width, frac_bits })
    }

    /// The multiplier `2^frac_bits`.
    pub fn scale(&self) -> f64 {
        2f64.powi(self.frac_bits as i32)
    }

    /// The scale as an exact integer (`1 << frac_bits`).
    pub fn scale_int(&self) -> u64 {
        1u64 << self.frac_bits
    }

    /// Encode a signed real value into the unsigned two's-complement
    /// representation of this format.
    ///
    /// # Errors
    ///
    /// Returns [`FheError::ValueOutOfRange`] if the rounded value does not
    /// fit the signed range of the configured width.
    pub fn encode(&self, value: f64) -> FheResult<u64> {
        let scaled = value * self.scale();
        let rounded = scaled.round();
        let as_signed = rounded as i64;
        let min = -(1i64 << (self.width.bits() - 1));
        let max = (1i64 << (self.width.bits() - 1)) - 1;
        if as_signed < min || as_signed > max {
            return Err(FheError::ValueOutOfRange {
                value: as_signed as u64,
                width: self.width.bits(),
                max: max as u64,
            });
        }
        Ok(self.pack_i64(as_signed))
    }

    /// Encode an already-scaled signed integer (no rounding).
    ///
    /// Convenient when the caller works in exact integer arithmetic and the
    /// fixed-point scale has already been applied.
    ///
    /// # Errors
    ///
    /// Returns [`FheError::ValueOutOfRange`] if `value` does not fit the
    /// signed range of the configured width.
    pub fn encode_scaled(&self, value: i64) -> FheResult<u64> {
        let min = -(1i64 << (self.width.bits() - 1));
        let max = (1i64 << (self.width.bits() - 1)) - 1;
        if value < min || value > max {
            return Err(FheError::ValueOutOfRange {
                value: value as u64,
                width: self.width.bits(),
                max: max as u64,
            });
        }
        Ok(self.pack_i64(value))
    }

    /// Convert a signed integer to its unsigned two's-complement bit pattern
    /// in the configured width. The value must already be known to fit.
    pub(crate) fn pack_i64(&self, value: i64) -> u64 {
        let mask = if self.width.bits() < 64 {
            (1u64 << self.width.bits()) - 1
        } else {
            u64::MAX
        };
        (value as u64) & mask
    }

    /// Decode an unsigned two's-complement bit pattern back to the signed
    /// real value.
    pub fn decode(&self, bits: u64) -> f64 {
        self.decode_signed_i64(bits) as f64 / self.scale()
    }

    /// Decode a bit pattern to the signed integer before the fixed-point
    /// division.
    pub fn decode_signed_i64(&self, bits: u64) -> i64 {
        let width = self.width.bits();
        if width < 64 {
            let sign_bit = 1u64 << (width - 1);
            let full = if bits & sign_bit != 0 {
                (bits as i128) - (1i128 << width)
            } else {
                bits as i128
            };
            full as i64
        } else {
            bits as i64
        }
    }

    /// Decode a decrypted [`PlaintextValue`] to the signed real value.
    ///
    /// This is the client-side counterpart of [`FixedPoint::encode`].
    pub fn decode_plaintext(&self, value: &PlaintextValue) -> FheResult<f64> {
        if value.width() != self.width {
            return Err(FheError::WidthMismatch {
                expected: self.width.bits(),
                found: value.width().bits(),
            });
        }
        Ok(self.decode(value.as_u64()))
    }

    /// Encode a slice of real values.
    ///
    /// # Errors
    ///
    /// Propagates [`FheError::ValueOutOfRange`] on the first out-of-range
    /// value.
    pub fn encode_slice(&self, values: &[f64]) -> FheResult<Vec<u64>> {
        values.iter().map(|v| self.encode(*v)).collect()
    }

    /// Decode a slice of raw bit patterns.
    pub fn decode_slice(&self, bits: &[u64]) -> Vec<f64> {
        bits.iter().map(|b| self.decode(*b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admits_reasonable_fractional_bits() {
        let fmt = FixedPoint::new(ScalarWidth::U16, 8).unwrap();
        assert_eq!(fmt.scale_int(), 256);
    }

    #[test]
    fn rejects_fractional_bits_equal_to_width() {
        assert!(matches!(
            FixedPoint::new(ScalarWidth::U8, 8),
            Err(FheError::ValueOutOfRange { .. })
        ));
    }

    #[test]
    fn roundtrips_positive_and_negative() {
        let fmt = FixedPoint::new(ScalarWidth::U16, 8).unwrap();
        for v in [-2.5, -1.0, -0.5, 0.0, 0.25, 1.0, 7.75, 127.5] {
            let encoded = fmt.encode(v).unwrap();
            let decoded = fmt.decode(encoded);
            assert!((decoded - v).abs() < 1e-3, "roundtrip {v} -> {decoded}");
        }
    }

    #[test]
    fn two_complement_wraps_modularly() {
        // -1 encodes to all-ones in the width (valid two's complement).
        let fmt = FixedPoint::new(ScalarWidth::U8, 0).unwrap();
        assert_eq!(fmt.encode_scaled(-1).unwrap(), 0xFF);
        // Adding 1 to -1 wraps to 0 in the 8-bit ring; mask the u64 add
        // back to the width (u64::wrapping_add wraps at 64 bits, not 8).
        let mask = 0xFFu64;
        let sum = (fmt.encode_scaled(-1).unwrap().wrapping_add(1)) & mask;
        assert_eq!(fmt.decode_signed_i64(sum), 0);
    }

    #[test]
    fn rejects_out_of_signed_range() {
        let fmt = FixedPoint::new(ScalarWidth::U8, 0).unwrap();
        assert!(matches!(
            fmt.encode_scaled(128),
            Err(FheError::ValueOutOfRange { .. })
        ));
        assert!(matches!(
            fmt.encode_scaled(-129),
            Err(FheError::ValueOutOfRange { .. })
        ));
    }

    #[test]
    fn decode_plaintext_matches_width() {
        let fmt = FixedPoint::new(ScalarWidth::U16, 8).unwrap();
        let encoded = fmt.encode(3.5).unwrap();
        let pv = PlaintextValue::U16(encoded as u16);
        assert!((fmt.decode_plaintext(&pv).unwrap() - 3.5).abs() < 1e-3);
    }
}
