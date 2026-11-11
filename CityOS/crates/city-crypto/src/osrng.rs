//! Deterministic 256-bit entropy expansion and a host entropy source.
//!
//! Replaces the earlier 32-bit-seeded LCG. An LCG is a *linear* map over a
//! 2^32 seed space: with at most 32 bits of seed entropy, any value derived
//! from it is brute-forceable in seconds. The new construction is a keyed
//! PRF (SipHash-2-4) run in counter mode over a **256-bit seed**, so the
//! output space and the seed space are both full-width:
//!
//! ```text
//! seed[0..16]  → SipHash key (128 bits)
//! seed[16..32] → message base mixed with a block counter → 4 × 64 bits
//! ```
//!
//! Deterministic per seed, `no_std`, no-alloc — a service can re-derive and
//! validate without storing per-holder state. On the host, `fill_os_random`
//! seeds it from the OS (via `std` hasher seeds); on a KELLER node the seed
//! comes from the tick counter plus sensor entropy (see docs).

use crate::bigint::{from_be_bytes, U256};
use crate::mac::siphash24;

/// Domain-separation key used by `entropy_seed` (public, non-secret).
const SEED_DOMAIN_K0: u64 = 0x5345_4544_5F44_4F4D;
const SEED_DOMAIN_K1: u64 = 0x4149_4E5F_4349_5459;

/// Expand a 256-bit seed into a full 256-bit value.
///
/// SipHash-2-4 counter mode: the first 16 seed bytes are the PRF key, the
/// last 16 are the message base, and a running block counter separates the
/// four output limbs. All 256 seed bits contribute to every output bit.
pub fn derive_entropy(seed: &[u8; 32]) -> U256 {
    let k0 = u64::from_le_bytes([
        seed[0], seed[1], seed[2], seed[3], seed[4], seed[5], seed[6], seed[7],
    ]);
    let k1 = u64::from_le_bytes([
        seed[8], seed[9], seed[10], seed[11], seed[12], seed[13], seed[14], seed[15],
    ]);
    let mut out = [0u8; 32];
    for i in 0..4 {
        // block = seed[16..32] ‖ counter(i)  (24 bytes)
        let mut block = [0u8; 24];
        block[0..16].copy_from_slice(&seed[16..32]);
        block[16..24].copy_from_slice(&(i as u64).to_le_bytes());
        let h = siphash24(k0, k1, &block);
        out[i * 8..i * 8 + 8].copy_from_slice(&h.to_le_bytes());
    }
    from_be_bytes(&out)
}

/// Expand a 32-bit value into a deterministic 256-bit seed.
///
/// Convenience for tests, demos, and small counters that need a full-width
/// seed (e.g. `city-heal`'s jitter). For real secrets, use a fresh
/// `fill_os_random` seed instead — a 32-bit input still only contributes
/// 32 bits of entropy.
pub fn entropy_seed(v: u32) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..4 {
        let mut block = [0u8; 8];
        block[0..4].copy_from_slice(&v.to_le_bytes());
        block[4..8].copy_from_slice(&(i as u32).to_le_bytes());
        let h = siphash24(SEED_DOMAIN_K0, SEED_DOMAIN_K1, &block);
        out[i * 8..i * 8 + 8].copy_from_slice(&h.to_le_bytes());
    }
    out
}

/// Fill a buffer with OS-entropy-derived bytes (host profile only).
///
/// Uses std's hasher randomisation (seeded from the OS RNG at process
/// start) mixed with system time and pid — a genuine, if modest, entropy
/// source for the host demo. The kernel profile replaces this with the
/// tick counter + sensor entropy.
#[cfg(feature = "std")]
pub fn fill_os_random(out: &mut [u8]) {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();

    let mut h = RandomState::new().build_hasher();
    h.write_u128(nanos);
    h.write_u32(pid);
    let a = h.finish();

    let mut h2 = RandomState::new().build_hasher();
    h2.write_u64(a);
    h2.write_u128(nanos.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let b = h2.finish();

    let mut seed = [0u8; 32];
    seed[0..8].copy_from_slice(&a.to_le_bytes());
    seed[8..16].copy_from_slice(&b.to_le_bytes());
    seed[16..24].copy_from_slice(&a.wrapping_mul(0xC2B2_AE3D_27D4_EB4F).to_le_bytes());
    seed[24..32].copy_from_slice(&b.wrapping_mul(0x94D0_49BB_1331_11EB).to_le_bytes());

    let mut off = 0usize;
    while off < out.len() {
        let chunk = derive_entropy(&seed);
        let bytes = crate::bigint::to_be_bytes(&chunk);
        let n = core::cmp::min(bytes.len(), out.len() - off);
        out[off..off + n].copy_from_slice(&bytes[..n]);
        off += n;
        seed = bytes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_from(v: u32) -> [u8; 32] {
        let mut s = [0u8; 32];
        s[0..4].copy_from_slice(&v.to_le_bytes());
        s
    }

    #[test]
    fn entropy_covers_all_limbs_and_is_deterministic() {
        let a = derive_entropy(&seed_from(42));
        assert_eq!(a, derive_entropy(&seed_from(42)));
        assert_ne!(a, derive_entropy(&seed_from(43)));
        assert!(a.iter().any(|&l| l != 0));
    }

    #[test]
    fn full_width_seed_uses_all_32_bytes() {
        // Seeds differing only in the *last* 16 bytes (the message base) must
        // still produce different outputs — the LCG never had this property
        // at all.
        let s1 = [0u8; 32];
        let mut s2 = [0u8; 32];
        s2[31] = 1;
        assert_ne!(derive_entropy(&s1), derive_entropy(&s2));
        // And seeds differing only in the key half too.
        let mut s3 = [0u8; 32];
        s3[0] = 1;
        assert_ne!(derive_entropy(&s1), derive_entropy(&s3));
    }

    #[test]
    fn entropy_seed_is_deterministic_and_full_width() {
        assert_eq!(entropy_seed(7), entropy_seed(7));
        assert_ne!(entropy_seed(7), entropy_seed(8));
        assert!(entropy_seed(7).iter().any(|&b| b != 0));
    }

    #[cfg(feature = "std")]
    #[test]
    fn os_random_fills_and_differs() {
        let mut a = [0u8; 64];
        let mut b = [0u8; 64];
        fill_os_random(&mut a);
        fill_os_random(&mut b);
        assert_ne!(a, b);
        assert!(a.iter().any(|&x| x != 0));
    }
}
