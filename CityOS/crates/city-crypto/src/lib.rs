//! City OS cryptographic core.
//!
//! This crate **reuses the KELLER OS verified primitives verbatim**:
//! - `bigint` — 256-bit secp256k1 field/group arithmetic
//! - `ec`     — affine secp256k1 point arithmetic (double/add/scalar-mul)
//! - `aes`    — software AES-128 + XTS mode (IEEE 1619)
//! - `vault`  — AES-XTS encrypted RAM secret store with integrity tags
//!   and Panic-Zeroing disposal
//!
//! on top of which City OS adds:
//! - `mac`    — keyed SipHash-2-4 message authentication (64-bit tag,
//!   128-bit secret key; replaces the forgeable FNV-1a salt)
//! - `osrng`  — 256-bit-seed PRF entropy expansion (SipHash counter mode)
//!   plus a host OS-entropy source (`std` feature)
//!
//! All code is `no_std` / no-alloc, matching the kernel constraint that City
//! services run on the KELLER microkernel without a heap.
#![no_std]
#![allow(static_mut_refs)]

#[cfg(test)]
extern crate std;

pub mod aes;
pub mod bigint;
pub mod ec;
pub mod spinlock;
pub mod vault;

pub mod mac;
mod osrng;

pub use bigint::{GX, GY, N, P, U256};
pub use ec::{Point, GENERATOR};
pub use mac::{city_mac, verify_mac, MacKey, MAC_TAG_LEN};
pub use osrng::{derive_entropy, entropy_seed};

/// Host-profile OS entropy (see `osrng::fill_os_random`).
#[cfg(feature = "std")]
pub use osrng::fill_os_random;

/// Re-export the vault as `KellerVault` so callers can name it unambiguously.
pub use vault::{Vault, VAULT_SECTORS, VAULT_SECTOR_SIZE};

#[cfg(test)]
mod tests {
    pub use super::*;

    #[test]
    fn keller_bigint_selftests_pass() {
        assert!(bigint::selftest_field());
        assert!(bigint::selftest_inverse());
        assert!(bigint::selftest_mul());
        assert!(bigint::selftest_serialize());
    }

    #[test]
    fn keller_ec_selftest_passes() {
        assert!(ec::selftest_ec());
    }

    #[test]
    fn keller_vault_xts_roundtrip_and_tamper() {
        assert!(vault::VAULT.lock().store(&[1, 2, 3, 4, 5]).is_some());
        let sector = 0;
        let mut out = [0u8; 5];
        assert!(vault::VAULT.lock().retrieve(sector, &mut out));
        assert_eq!(&out, &[1, 2, 3, 4, 5]);

        // Tamper detection: flip a bit in the ciphertext, retrieve must fail.
        assert!(vault::tamper_bit(sector, 3));
        let mut out2 = [0u8; 5];
        assert!(!vault::VAULT.lock().retrieve(sector, &mut out2));

        // Clean up for other tests.
        vault::dispose_all();
    }

    #[test]
    fn keyed_mac_and_entropy() {
        let key = MacKey::derive(0xA5C34B71);
        let body = *b"city-os/identity-less";
        let mac = city_mac(&key, 7, 42, 1, &body);
        assert!(verify_mac(&key, 7, 42, 1, &body, mac));

        // Same inputs -> same MAC; changing any byte -> different MAC.
        let mac2 = city_mac(&key, 7, 42, 1, &body);
        assert_eq!(mac, mac2);
        let mut tampered = body;
        tampered[0] ^= 1;
        assert!(!verify_mac(&key, 7, 42, 1, &tampered, mac));
        // A different key fails (the old FNV salt could not do this).
        assert!(!verify_mac(&MacKey::derive(1), 7, 42, 1, &body, mac));

        // Entropy expansion covers all 8 limbs and is deterministic per seed.
        let a = derive_entropy(&entropy_seed(0x1234_5678));
        let b = derive_entropy(&entropy_seed(0x1234_5678));
        let c = derive_entropy(&entropy_seed(0xABCD_EF01));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, [0u32; 8]);
    }

    #[test]
    fn schnorr_style_response_uses_group_order() {
        use bigint::{from_u32, mod_add_n, mod_mul_n, mod_n_reduce};

        // Identical to Keller Auth compute_response semantics:
        // r = (k + c·x) mod n, with all scalars reduced mod n.
        let k = derive_entropy(&entropy_seed(0xC0FFEE));
        let c = derive_entropy(&entropy_seed(0xBADF00D));
        let x = from_u32(0x0102_0304);
        let k_n = mod_n_reduce(&k);
        let c_n = mod_n_reduce(&c);
        let x_n = mod_n_reduce(&x);
        let cx = mod_mul_n(&c_n, &x_n);
        let r = mod_add_n(&k_n, &cx);

        // Sanity: r < n and non-zero for a non-trivial secret.
        assert!(bigint::cmp(&r, &bigint::N) < 0);
        assert!(bigint::cmp(&k_n, &bigint::N) < 0);
        assert!(!bigint::is_zero(&r));
    }
}
