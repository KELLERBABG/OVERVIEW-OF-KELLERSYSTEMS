//! 256-bit unsigned integer arithmetic for secp256k1 Schnorr.
//!
//! Representation: little-endian `[u32; 8]` (limb 0 = least significant).
//! All operations are no_std / no-alloc and use u64 intermediates to avoid
//! overflow on the 32-bit target.
//!
//! The cryptographic strength comes from the 256-bit field — a 32-bit group
//! can be broken in milliseconds; this cannot.
//!
//! VERBATIM REUSE from KELLER OS (`sovereign-core/src/bigint.rs`).

#![allow(clippy::needless_range_loop)]
pub type U256 = [u32; 8];

// ---------------------------------------------------------------------------
// Constants (secp256k1)
// ---------------------------------------------------------------------------

/// Field prime p = 2^256 - 2^32 - 977
/// = FFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFF FFFFFFFE FFFFFC2F
pub const P: U256 = [
    0xFFFF_FC2F,
    0xFFFF_FFFE,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
];

/// Group order n (order of the generator G).
pub const N: U256 = [
    0xD036_4141,
    0xBFD2_5E8C,
    0xAF48_A03B,
    0xBAAE_DCE6,
    0xFFFF_FFFE,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
    0xFFFF_FFFF,
];

/// Generator point Gx.
pub const GX: U256 = [
    0x16F8_1798,
    0x59F2_815B,
    0x2DCE_28D9,
    0x029B_FCDB,
    0xCE87_0B07,
    0x55A0_6295,
    0xF9DC_BBAC,
    0x79BE_667E,
];

/// Generator point Gy.
pub const GY: U256 = [
    0xFB10_D4B8,
    0x9C47_D08F,
    0xA685_5419,
    0xFD17_B448,
    0x0E11_08A8,
    0x5DA4_FBFC,
    0x26A3_C465,
    0x483A_DA77,
];

/// c = 2^32 + 977. Since p = 2^256 - c, we have 2^256 ≡ c (mod p).
const C: u64 = (1u64 << 32) + 977; // 4_294_968_273

// ---------------------------------------------------------------------------
// Basic arithmetic
// ---------------------------------------------------------------------------

/// Add two 256-bit numbers modulo 2^256 (wrapping).
pub fn add(a: &U256, b: &U256) -> U256 {
    let mut r = [0u32; 8];
    let mut carry = 0u64;
    for i in 0..8 {
        let s = a[i] as u64 + b[i] as u64 + carry;
        r[i] = s as u32;
        carry = s >> 32;
    }
    r
}

/// Add with carry: (a + b mod 2^256, carry).
pub fn add_c(a: &U256, b: &U256) -> (U256, u32) {
    let mut r = [0u32; 8];
    let mut carry = 0u64;
    for i in 0..8 {
        let s = a[i] as u64 + b[i] as u64 + carry;
        r[i] = s as u32;
        carry = s >> 32;
    }
    (r, carry as u32)
}

/// Add a small value (< 2^64) to a 256-bit number.
/// Returns (sum mod 2^256, carry).
pub fn add_small64(a: &U256, v: u64) -> (U256, u32) {
    let v_lo = v as u32;
    let v_hi = (v >> 32) as u32;
    let mut r = [0u32; 8];
    let mut carry = 0u64;
    for i in 0..8 {
        let av: u64 = match i {
            0 => v_lo as u64,
            1 => v_hi as u64,
            _ => 0,
        };
        let s = a[i] as u64 + av + carry;
        r[i] = s as u32;
        carry = s >> 32;
    }
    (r, carry as u32)
}

/// Subtract two 256-bit numbers modulo 2^256 (wrapping).
pub fn sub(a: &U256, b: &U256) -> U256 {
    let mut r = [0u32; 8];
    let mut borrow = 0u64;
    for i in 0..8 {
        let ai = a[i] as u64;
        let bi = b[i] as u64 + borrow;
        if ai >= bi {
            r[i] = (ai - bi) as u32;
            borrow = 0;
        } else {
            r[i] = (ai + (1u64 << 32) - bi) as u32;
            borrow = 1;
        }
    }
    r
}

/// Compare two 256-bit numbers: 1 if a > b, -1 if a < b, 0 if equal.
pub fn cmp(a: &U256, b: &U256) -> i8 {
    for i in (0..8).rev() {
        if a[i] > b[i] {
            return 1;
        }
        if a[i] < b[i] {
            return -1;
        }
    }
    0
}

/// Zero check.
pub fn is_zero(a: &U256) -> bool {
    for &v in a {
        if v != 0 {
            return false;
        }
    }
    true
}

/// Multiply two 256-bit numbers, returning the FULL 512-bit product
/// (little-endian [u32; 16]).
pub fn mul_full(a: &U256, b: &U256) -> [u32; 16] {
    let mut r = [0u64; 16];
    for i in 0..8 {
        let mut carry = 0u64;
        for j in 0..8 {
            let cur = r[i + j] + (a[i] as u64) * (b[j] as u64) + carry;
            r[i + j] = cur & 0xFFFF_FFFF;
            carry = cur >> 32;
        }
        let mut k = i + 8;
        while carry > 0 && k < 16 {
            let cur = r[k] + carry;
            r[k] = cur & 0xFFFF_FFFF;
            carry = cur >> 32;
            k += 1;
        }
    }
    let mut out = [0u32; 16];
    for i in 0..16 {
        out[i] = r[i] as u32;
    }
    out
}

/// Multiply a 256-bit number by a 32-bit value.
pub fn mul_u32(a: &U256, m: u32) -> (U256, u32) {
    let mut r = [0u32; 8];
    let mut carry: u64 = 0;
    for i in 0..8 {
        let cur = (a[i] as u64) * (m as u64) + carry;
        r[i] = cur as u32;
        carry = cur >> 32;
    }
    (r, carry as u32)
}

/// Shift left by 32 bits (multiply by 2^32).
pub fn shl32(a: &U256) -> (U256, u32) {
    let mut r = [0u32; 8];
    r[1..8].copy_from_slice(&a[..7]);
    (r, a[7])
}

/// Shift right by n bits (n in 1..31).
#[allow(dead_code)]
fn shiftr(a: &U256, n: u32) -> U256 {
    debug_assert!((1..=31).contains(&n));
    let mut r = [0u32; 8];
    for i in 0..8 {
        let lo = a[i] >> n;
        let hi = if i + 1 < 8 { a[i + 1] << (32 - n) } else { 0 };
        r[i] = lo | hi;
    }
    r
}

/// Shift left by n bits (n in 1..31).
#[allow(dead_code)]
fn shiftl(a: &U256, n: u32) -> U256 {
    debug_assert!((1..=31).contains(&n));
    let mut r = [0u32; 8];
    for i in (0..8).rev() {
        let hi = a[i] << n;
        let lo = if i > 0 { a[i - 1] >> (32 - n) } else { 0 };
        r[i] = hi | lo;
    }
    r
}

// ---------------------------------------------------------------------------
// Modular reduction
// ---------------------------------------------------------------------------

/// Reduce a value < 2^257 (at most 2 subtractions of p are needed).
pub fn mod_p_reduce(a: &U256) -> U256 {
    let mut r = *a;
    if cmp(&r, &P) >= 0 {
        r = sub(&r, &P);
    }
    if cmp(&r, &P) >= 0 {
        r = sub(&r, &P);
    }
    r
}

/// Fold a carry past 2^256: if carry != 0, add c (since 2^256 ≡ c mod p).
fn fold_carry(a: &U256, carry: u32) -> U256 {
    if carry == 0 {
        return mod_p_reduce(a);
    }
    debug_assert_eq!(carry, 1);
    let (r1, c1) = add_small64(a, C);
    if c1 != 0 {
        let (r2, c2) = add_small64(&r1, C);
        debug_assert_eq!(c2, 0);
        mod_p_reduce(&r2)
    } else {
        mod_p_reduce(&r1)
    }
}

/// Reduce a full 512-bit product modulo p (bit-by-bit binary long division).
pub fn mod_p_512(v: &[u32; 16]) -> U256 {
    let mut acc = [0u32; 9];
    for bit in (0..512usize).rev() {
        let word = bit / 32;
        let bit_in_word = bit % 32;
        let input_bit = (v[word] >> bit_in_word) & 1;
        let mut carry = input_bit as u64;
        for k in 0..9 {
            let s = ((acc[k] as u64) << 1) | carry;
            acc[k] = s as u32;
            carry = s >> 32;
        }
        let ge = if acc[8] != 0 {
            true
        } else {
            cmp(&acc[..8].try_into().unwrap(), &P) >= 0
        };
        if ge {
            let mut borrow = 0u64;
            for k in 0..8 {
                let ai = acc[k] as u64;
                let bi = P[k] as u64 + borrow;
                if ai >= bi {
                    acc[k] = (ai - bi) as u32;
                    borrow = 0;
                } else {
                    acc[k] = (ai + (1u64 << 32) - bi) as u32;
                    borrow = 1;
                }
            }
            acc[8] = acc[8].wrapping_sub(borrow as u32);
        }
    }
    let mut r: U256 = [0; 8];
    r.copy_from_slice(&acc[..8]);
    r
}

/// Reduce a full 512-bit product modulo n (group order).
pub fn mod_n_512(v: &[u32; 16]) -> U256 {
    let mut acc = [0u32; 9];
    for bit in (0..512usize).rev() {
        let word = bit / 32;
        let bit_in_word = bit % 32;
        let input_bit = (v[word] >> bit_in_word) & 1;
        let mut carry = input_bit as u64;
        for k in 0..9 {
            let s = ((acc[k] as u64) << 1) | carry;
            acc[k] = s as u32;
            carry = s >> 32;
        }
        let ge = if acc[8] != 0 {
            true
        } else {
            cmp(&acc[..8].try_into().unwrap(), &N) >= 0
        };
        if ge {
            let mut borrow = 0u64;
            for k in 0..8 {
                let ai = acc[k] as u64;
                let bi = N[k] as u64 + borrow;
                if ai >= bi {
                    acc[k] = (ai - bi) as u32;
                    borrow = 0;
                } else {
                    acc[k] = (ai + (1u64 << 32) - bi) as u32;
                    borrow = 1;
                }
            }
            acc[8] = acc[8].wrapping_sub(borrow as u32);
        }
    }
    let mut r: U256 = [0; 8];
    r.copy_from_slice(&acc[..8]);
    r
}

/// Reduce a value < 2n down to < n using one conditional subtraction.
pub fn mod_n_reduce(a: &U256) -> U256 {
    if cmp(a, &N) >= 0 {
        sub(a, &N)
    } else {
        *a
    }
}

/// Modular multiplication modulo n (group order): a * b mod n.
pub fn mod_mul_n(a: &U256, b: &U256) -> U256 {
    let prod = mul_full(a, b);
    mod_n_512(&prod)
}

/// Modular addition modulo n (group order): (a + b) mod n.
pub fn mod_add_n(a: &U256, b: &U256) -> U256 {
    let (s, c) = add_c(a, b);
    if c != 0 {
        let q: U256 = [
            0x2FC9_BEBF,
            0x402D_A173,
            0x50B7_5FC4,
            0x4551_2319,
            0x0000_0001,
            0x0000_0000,
            0x0000_0000,
            0x0000_0000,
        ];
        let (sum, c2) = add_c(&s, &q);
        debug_assert_eq!(c2, 0);
        mod_n_reduce(&sum)
    } else {
        mod_n_reduce(&s)
    }
}

/// Modular multiplication: a * b mod p (full 512-bit product reduced).
pub fn mod_mul(a: &U256, b: &U256) -> U256 {
    let prod = mul_full(a, b);
    mod_p_512(&prod)
}

/// Modular squaring: a^2 mod p.
pub fn mod_sqr(a: &U256) -> U256 {
    mod_mul(a, a)
}

/// Modular addition: a + b mod p (a, b < p).
pub fn mod_add(a: &U256, b: &U256) -> U256 {
    let (s, c) = add_c(a, b);
    fold_carry(&s, c)
}

/// Modular subtraction: a - b mod p (a, b < p).
pub fn mod_sub(a: &U256, b: &U256) -> U256 {
    if cmp(a, b) >= 0 {
        sub(a, b)
    } else {
        let d = sub(b, a);
        sub(&P, &d)
    }
}

/// Modular negation: -a mod p.
pub fn mod_neg(a: &U256) -> U256 {
    if is_zero(a) {
        [0u32; 8]
    } else {
        sub(&P, a)
    }
}

/// Modular exponentiation: base^exp mod p (fixed 256 iterations).
pub fn mod_pow(base: U256, exp: &U256) -> U256 {
    let mut result = from_u32(1);
    for i in (0..8).rev() {
        for bit in (0..32).rev() {
            result = mod_sqr(&result);
            if (exp[i] >> bit) & 1 != 0 {
                result = mod_mul(&result, &base);
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Modular inverse (Fermat's Little Theorem)
// ---------------------------------------------------------------------------

/// Modular inverse: a^(-1) mod p (Fermat's Little Theorem, fixed loop).
pub fn mod_inv(a: &U256) -> U256 {
    if is_zero(a) {
        return [0u32; 8];
    }
    let p_minus_2 = sub(&P, &from_u32(2));
    mod_pow(*a, &p_minus_2)
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

/// Serialize to 32 bytes big-endian (standard wire format).
pub fn to_be_bytes(a: &U256) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..8 {
        let limb = a[i].to_be_bytes();
        let base = 28 - i * 4;
        out[base..base + 4].copy_from_slice(&limb);
    }
    out
}

/// Parse 32 bytes big-endian into a U256.
pub fn from_be_bytes(b: &[u8; 32]) -> U256 {
    let mut r = [0u32; 8];
    for i in 0..8 {
        let base = 28 - i * 4;
        r[i] = u32::from_be_bytes([b[base], b[base + 1], b[base + 2], b[base + 3]]);
    }
    r
}

/// Build a U256 from a u32.
pub const fn from_u32(v: u32) -> U256 {
    let mut r = [0u32; 8];
    r[0] = v;
    r
}

/// Build a U256 from a u64.
pub const fn from_u64(v: u64) -> U256 {
    let mut r = [0u32; 8];
    r[0] = v as u32;
    r[1] = (v >> 32) as u32;
    r
}

// ---------------------------------------------------------------------------
// Tests (used by selftest)
// ---------------------------------------------------------------------------

/// Verify field arithmetic: 1+1=2, (p-1)+1=0, curve equation holds.
pub fn selftest_field() -> bool {
    let one = from_u32(1);
    let two = mod_add(&one, &one);
    if cmp(&two, &from_u32(2)) != 0 {
        return false;
    }
    let p_minus_1 = sub(&P, &one);
    let zero = mod_add(&p_minus_1, &one);
    if !is_zero(&zero) {
        return false;
    }
    let x3 = mod_mul(&mod_sqr(&GX), &GX);
    let x3_7 = mod_add(&x3, &from_u32(7));
    let y2 = mod_sqr(&GY);
    if cmp(&x3_7, &y2) != 0 {
        return false;
    }
    true
}

/// Verify modular inverse: a * a^(-1) == 1 mod p.
pub fn selftest_inverse() -> bool {
    let a = from_u32(0x1234_5678);
    let inv = mod_inv(&a);
    if is_zero(&inv) {
        return false;
    }
    let prod = mod_mul(&a, &inv);
    cmp(&prod, &from_u32(1)) == 0
}

/// Verify multiplication: (2^128)^2 mod p == 2^256 mod p == 2^32 + 977.
pub fn selftest_mul() -> bool {
    let two_128: U256 = [0, 0, 0, 0, 1, 0, 0, 0];
    let sq = mod_sqr(&two_128);
    let expected: U256 = [0x3D1, 1, 0, 0, 0, 0, 0, 0];
    cmp(&sq, &expected) == 0
}

/// Verify big-endian round-trip.
pub fn selftest_serialize() -> bool {
    let a = GX;
    let bytes = to_be_bytes(&a);
    let b = from_be_bytes(&bytes);
    cmp(&a, &b) == 0
}
