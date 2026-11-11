//! Finite field arithmetic over Z_p for Shamir's Secret Sharing.
//!
//! Uses a large prime p = 2^61 - 1 (a Mersenne prime) for efficient
//! modular arithmetic with 64-bit integers.

use std::ops::{Add, Div, Mul, Neg, Sub};

/// A prime field Z_p where p = 2^61 - 1 (Mersenne prime).
/// This prime is large enough for practical applications and enables
/// efficient modular reduction using bit shifts.
pub const PRIME: u64 = (1u64 << 61) - 1; // 2305843009213693951

/// An element of the finite field Z_p.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FiniteField(pub u64);

impl FiniteField {
    /// Create a new field element, automatically reduced mod p.
    pub fn new(value: u64) -> Self {
        Self(value % PRIME)
    }

    /// Create from a signed 64-bit integer (handles negative values).
    pub fn from_i64(value: i64) -> Self {
        if value >= 0 {
            Self::new(value as u64)
        } else {
            // Map negative to positive in Z_p
            let abs = value.unsigned_abs();
            let reduced = abs % PRIME;
            if reduced == 0 { Self(0) } else { Self(PRIME - reduced) }
        }
    }

    /// Convert to i64 (reinterpret field element as signed).
    pub fn to_i64(&self) -> i64 {
        if self.0 > (PRIME / 2) {
            // Negative representation
            -((PRIME - self.0) as i64)
        } else {
            self.0 as i64
        }
    }

    /// Get the zero element.
    pub fn zero() -> Self {
        Self(0)
    }

    /// Get the one element.
    pub fn one() -> Self {
        Self(1)
    }

    /// Check if zero.
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    /// Modular inverse using Fermat's little theorem: a^(p-2) mod p.
    pub fn inverse(&self) -> Self {
        if self.is_zero() {
            panic!("Cannot invert zero");
        }
        self.pow(PRIME - 2)
    }

    /// Modular exponentiation by squaring.
    pub fn pow(&self, exp: u64) -> Self {
        let mut result = Self::one();
        let mut base = *self;
        let mut e = exp;
        while e > 0 {
            if e & 1 == 1 {
                result = result * base;
            }
            base = base * base;
            e >>= 1;
        }
        result
    }
}

// --- Arithmetic operations with modular reduction ---

impl Add for FiniteField {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        let sum = self.0 as u128 + rhs.0 as u128;
        Self((sum % PRIME as u128) as u64)
    }
}

impl Sub for FiniteField {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        if self.0 >= rhs.0 { Self(self.0 - rhs.0) } else { Self(PRIME - (rhs.0 - self.0)) }
    }
}

impl Mul for FiniteField {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let product = self.0 as u128 * rhs.0 as u128;
        Self((product % PRIME as u128) as u64)
    }
}

impl Div for FiniteField {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        self * rhs.inverse()
    }
}

impl Neg for FiniteField {
    type Output = Self;
    fn neg(self) -> Self {
        if self.0 == 0 { self } else { Self(PRIME - self.0) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let a = FiniteField::new(10);
        let b = FiniteField::new(20);
        assert_eq!((a + b).0, 30);
    }

    #[test]
    fn test_sub() {
        let a = FiniteField::new(20);
        let b = FiniteField::new(10);
        assert_eq!((a - b).0, 10);
    }

    #[test]
    fn test_mul() {
        let a = FiniteField::new(7);
        let b = FiniteField::new(8);
        assert_eq!((a * b).0, 56);
    }

    #[test]
    fn test_div() {
        let a = FiniteField::new(10);
        let b = FiniteField::new(2);
        assert_eq!((a / b).0, 5);
    }

    #[test]
    fn test_neg() {
        let a = FiniteField::new(10);
        assert_eq!((-a).0, PRIME - 10);
        assert_eq!((-(-a)).0, 10);
    }

    #[test]
    fn test_inverse() {
        let a = FiniteField::new(3);
        let inv = a.inverse();
        assert_eq!((a * inv).0, 1);
    }

    #[test]
    fn test_negative_representation() {
        let neg = FiniteField::from_i64(-5);
        let pos = FiniteField::new(5);
        // -5 + 5 should be 0
        assert_eq!((neg + pos).0, 0);
    }

    #[test]
    fn test_commutativity() {
        let a = FiniteField::new(123456);
        let b = FiniteField::new(789012);
        assert_eq!(a + b, b + a);
        assert_eq!(a * b, b * a);
    }
}
