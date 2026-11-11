//! Elliptic curve point arithmetic on secp256k1 over the field prime P.
//!
//! Affine coordinates (x, y). Point at infinity is represented by (0, 0).
//! All operations are no_std / no-alloc.
//!
//! The group law for y² = x³ + 7:
//!   - Doubling:  λ = (3x²) / (2y),  x3 = λ² - 2x,  y3 = λ(x - x3) - y
//!   - Addition:  λ = (y2 - y1) / (x2 - x1),  x3 = λ² - x1 - x2,  y3 = λ(x1 - x3) - y1
//!
//! VERBATIM REUSE from KELLER OS (`sovereign-core/src/ec.rs`).

#![allow(clippy::needless_range_loop)]

use crate::bigint::*;

/// A point on secp256k1 in affine coordinates. (0,0) encodes the point at
/// infinity.
#[derive(Copy, Clone, PartialEq)]
pub struct Point {
    pub x: U256,
    pub y: U256,
}

impl Point {
    pub const fn new(x: U256, y: U256) -> Self {
        Self { x, y }
    }

    /// The point at infinity.
    pub const fn infinity() -> Self {
        Self {
            x: [0; 8],
            y: [0; 8],
        }
    }

    pub fn is_infinity(&self) -> bool {
        is_zero(&self.x) && is_zero(&self.y)
    }

    /// Check whether this point lies on the curve y² = x³ + 7 (mod p).
    pub fn on_curve(&self) -> bool {
        if self.is_infinity() {
            return true;
        }
        let y2 = mod_sqr(&self.y);
        let x2 = mod_sqr(&self.x);
        let x3 = mod_mul(&x2, &self.x);
        let x3_7 = mod_add(&x3, &from_u32(7));
        cmp(&y2, &x3_7) == 0
    }
}

/// The secp256k1 generator point.
pub const GENERATOR: Point = Point::new(GX, GY);

/// Double a point (P + P). Assumes P is not the point at infinity and
/// y != 0 (which holds for all points of prime order n).
pub fn point_double(p: &Point) -> Point {
    if p.is_infinity() {
        return *p;
    }
    let x2 = mod_sqr(&p.x);
    let three_x2 = mod_add(&mod_add(&x2, &x2), &x2);
    let two_y = mod_add(&p.y, &p.y);
    let two_y_inv = mod_inv(&two_y);
    let lambda = mod_mul(&three_x2, &two_y_inv);

    let lambda2 = mod_sqr(&lambda);
    let two_x = mod_add(&p.x, &p.x);
    let x3 = mod_sub(&lambda2, &two_x);

    let x_minus_x3 = mod_sub(&p.x, &x3);
    let lambda_x = mod_mul(&lambda, &x_minus_x3);
    let y3 = mod_sub(&lambda_x, &p.y);

    Point::new(x3, y3)
}

/// Add two points (P + Q). Handles the point at infinity.
pub fn point_add(p: &Point, q: &Point) -> Point {
    if p.is_infinity() {
        return *q;
    }
    if q.is_infinity() {
        return *p;
    }
    if cmp(&p.x, &q.x) == 0 {
        if cmp(&p.y, &q.y) == 0 {
            return point_double(p);
        }
        return Point::infinity();
    }

    let dy = mod_sub(&q.y, &p.y);
    let dx = mod_sub(&q.x, &p.x);
    let dx_inv = mod_inv(&dx);
    let lambda = mod_mul(&dy, &dx_inv);

    let lambda2 = mod_sqr(&lambda);
    let x3 = mod_sub(&mod_sub(&lambda2, &p.x), &q.x);

    let x1_minus_x3 = mod_sub(&p.x, &x3);
    let lambda_x = mod_mul(&lambda, &x1_minus_x3);
    let y3 = mod_sub(&lambda_x, &p.y);

    Point::new(x3, y3)
}

/// Scalar multiplication: k * P using double-and-add.
pub fn scalar_mul(k: &U256, p: &Point) -> Point {
    let mut result = Point::infinity();
    let mut addend = *p;
    for i in 0..8 {
        for bit in 0..32 {
            if (k[i] >> bit) & 1 != 0 {
                result = point_add(&result, &addend);
            }
            addend = point_double(&addend);
        }
    }
    result
}

/// Negate a point: -P = (x, -y mod p).
pub fn point_neg(p: &Point) -> Point {
    Point::new(p.x, mod_neg(&p.y))
}

/// Serialize a point to uncompressed SEC1 format:
/// 0x04 || x (32 bytes BE) || y (32 bytes BE)
pub fn point_to_bytes(p: &Point) -> [u8; 65] {
    let mut out = [0u8; 65];
    out[0] = 0x04;
    let xb = to_be_bytes(&p.x);
    let yb = to_be_bytes(&p.y);
    out[1..33].copy_from_slice(&xb);
    out[33..65].copy_from_slice(&yb);
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Verify the generator satisfies the curve equation and that scalar
/// multiplication by 1 yields G, by 2 yields 2G (with known coordinates).
pub fn selftest_ec() -> bool {
    if !GENERATOR.on_curve() {
        return false;
    }
    let g2 = point_double(&GENERATOR);
    let expected_x: U256 = from_be_bytes(&[
        0xC6, 0x04, 0x7F, 0x94, 0x41, 0xED, 0x7D, 0x6D, 0x30, 0x45, 0x40, 0x6E, 0x95, 0xC0, 0x7C,
        0xD8, 0x5C, 0x77, 0x8E, 0x4B, 0x8C, 0xEF, 0x3C, 0xA7, 0xAB, 0xAC, 0x09, 0xB9, 0x5C, 0x70,
        0x9E, 0xE5,
    ]);
    if cmp(&g2.x, &expected_x) != 0 {
        return false;
    }
    let one = from_u32(1);
    let g1 = scalar_mul(&one, &GENERATOR);
    if cmp(&g1.x, &GENERATOR.x) != 0 || cmp(&g1.y, &GENERATOR.y) != 0 {
        return false;
    }
    let two = from_u32(2);
    let g2_scalar = scalar_mul(&two, &GENERATOR);
    if cmp(&g2_scalar.x, &g2.x) != 0 || cmp(&g2_scalar.y, &g2.y) != 0 {
        return false;
    }
    true
}
