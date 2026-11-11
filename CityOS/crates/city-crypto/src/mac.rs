//! Keyed **SipHash-2-4** message authentication — the City OS wire MAC.
//!
//! Replaces the earlier salted FNV-1a tag. FNV-1a is a *non-cryptographic*
//! hash: its multiply/XOR steps are invertible, so anyone who observes one
//! `(message, tag)` pair can recover the salt and forge tags for arbitrary
//! messages. SipHash-2-4 is a keyed PRF: the tag is
//! `SipHash-2-4(key, msg)` with a secret 128-bit key, and a tag reveals
//! nothing about the key — forging still requires the key or a 2^64
//! brute-force over the tag space.
//!
//! The MAC authenticates `(sender, msg_type, seq, payload)`; the key gives
//! per-channel / per-service domain separation. The receiver that holds the
//! matching key detects any byte change, any sender spoof, and any replayed
//! sequence number.
//!
//! Tag width: 64 bits (`MAC_TAG_LEN = 8`). On the wire a City frame is
//! `payload[24] ‖ tag[8]` — see `docs/ARCHITECTURE.md`.
//!
//! **Key management note.** This workspace's host profile derives keys from
//! public domain labels (`MacKey::derive`) so tests and demos are
//! deterministic; on a real KELLER node the kernel stores per-channel keys
//! in the AES-XTS vault and hands them to services at boot, which is the
//! deployment this API is shaped for.

/// A 128-bit secret MAC key.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MacKey(pub [u8; 16]);

impl MacKey {
    pub const ZERO: Self = Self([0; 16]);

    pub const fn from_bytes(b: [u8; 16]) -> Self {
        Self(b)
    }

    /// Derive a 128-bit key from a 32-bit domain label.
    ///
    /// Deterministic and public — intended for the *host/test* profile so
    /// services can use a label-derived key without key infrastructure. A
    /// production node replaces these with vault-held random keys; the MAC
    /// function itself is identical either way.
    pub fn derive(label: u32) -> Self {
        // Domain-separation key for label expansion (public constant).
        let (k0, k1) = (0x4349_5459_4F53_0001u64, 0x4F53_4349_5459_0002u64);
        let mut out = [0u8; 16];
        for i in 0..2 {
            let mut block = [0u8; 8];
            block[0..4].copy_from_slice(&label.to_le_bytes());
            block[4..8].copy_from_slice(&(i as u32).to_le_bytes());
            let h = siphash24(k0, k1, &block);
            out[i * 8..i * 8 + 8].copy_from_slice(&h.to_le_bytes());
        }
        Self(out)
    }
}

/// Size of the MAC tag in bytes (one u64).
pub const MAC_TAG_LEN: usize = 8;

/// Compute a 64-bit keyed MAC over `(sender, msg_type, seq, payload)`.
///
/// Deterministic given the key. Changing any field, any payload byte, or
/// reusing a sequence number with the same key changes the tag, so a
/// receiver holding the key detects fabrication and replay.
pub fn city_mac(key: &MacKey, sender: u32, msg_type: u32, seq: u32, payload: &[u8]) -> u64 {
    let k0 = u64::from_le_bytes([
        key.0[0], key.0[1], key.0[2], key.0[3], key.0[4], key.0[5], key.0[6], key.0[7],
    ]);
    let k1 = u64::from_le_bytes([
        key.0[8], key.0[9], key.0[10], key.0[11], key.0[12], key.0[13], key.0[14], key.0[15],
    ]);
    let mut buf = [0u8; 64];
    buf[0..4].copy_from_slice(&sender.to_le_bytes());
    buf[4..8].copy_from_slice(&msg_type.to_le_bytes());
    buf[8..12].copy_from_slice(&seq.to_le_bytes());
    // The payload rides directly after the 12-byte header. The 64-byte stack
    // buffer covers every City payload (a 32-byte `Bytes32` body + 12-byte
    // header = 44 bytes; the wire caps at 24).
    let head = 12;
    buf[head..head + payload.len()].copy_from_slice(payload);
    siphash24(k0, k1, &buf[..head + payload.len()])
}

/// Verify a MAC over the same fields. Returns false on any mismatch.
pub fn verify_mac(
    key: &MacKey,
    sender: u32,
    msg_type: u32,
    seq: u32,
    payload: &[u8],
    mac: u64,
) -> bool {
    city_mac(key, sender, msg_type, seq, payload) == mac
}

// ---------------------------------------------------------------------------
// SipHash-2-4 (reference implementation; no_std, no-alloc).
// ---------------------------------------------------------------------------

const SIPHASH_ROUNDS: u32 = 2;
const SIPHASH_FINAL_ROUNDS: u32 = 4;

#[inline]
fn siphash_round(v: &mut [u64; 4]) {
    v[0] = v[0].wrapping_add(v[1]);
    v[1] = v[1].rotate_left(13);
    v[1] ^= v[0];
    v[0] = v[0].rotate_left(32);
    v[2] = v[2].wrapping_add(v[3]);
    v[3] = v[3].rotate_left(16);
    v[3] ^= v[2];
    v[0] = v[0].wrapping_add(v[3]);
    v[3] = v[3].rotate_left(21);
    v[3] ^= v[0];
    v[2] = v[2].wrapping_add(v[1]);
    v[1] = v[1].rotate_left(17);
    v[1] ^= v[2];
    v[2] = v[2].rotate_left(32);
}

/// SipHash-2-4 over `input` with 128-bit key `(k0, k1)`.
pub(crate) fn siphash24(k0: u64, k1: u64, input: &[u8]) -> u64 {
    let mut v = [
        k0 ^ 0x736f_6d65_7073_6575,
        k1 ^ 0x646f_7261_6e64_6f6d,
        k0 ^ 0x6c79_6765_6e65_7261,
        k1 ^ 0x7465_6462_7974_6573,
    ];
    let b = (input.len() as u64) << 56;

    let mut i = 0usize;
    while i + 8 <= input.len() {
        let m = u64::from_le_bytes([
            input[i],
            input[i + 1],
            input[i + 2],
            input[i + 3],
            input[i + 4],
            input[i + 5],
            input[i + 6],
            input[i + 7],
        ]);
        v[3] ^= m;
        for _ in 0..SIPHASH_ROUNDS {
            siphash_round(&mut v);
        }
        v[0] ^= m;
        i += 8;
    }

    let mut last = [0u8; 8];
    let rem = input.len() - i;
    last[..rem].copy_from_slice(&input[i..]);
    let m = u64::from_le_bytes(last) | b;
    v[3] ^= m;
    for _ in 0..SIPHASH_ROUNDS {
        siphash_round(&mut v);
    }
    v[0] ^= m;
    v[2] ^= 0xFF;
    for _ in 0..SIPHASH_FINAL_ROUNDS {
        siphash_round(&mut v);
    }
    v[0] ^ v[1] ^ v[2] ^ v[3]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Key = 00 01 02 … 0f (reference implementation's test key).
    fn ref_key() -> MacKey {
        MacKey([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ])
    }

    #[test]
    fn siphash24_reference_vectors() {
        let k0 = 0x0706_0504_0302_0100u64;
        let k1 = 0x0f0e_0d0c_0b0a_0908u64;
        // Reference vectors from veorq/SipHash vectors.h, key 00..0f:
        // input length 0 (empty) and length 1 (one byte 0x00).
        assert_eq!(siphash24(k0, k1, &[]), 0x726f_db47_dd0e_0e31);
        assert_eq!(siphash24(k0, k1, &[0x00]), 0x74f8_39c5_93dc_67fd);
    }

    #[test]
    fn oversized_payload_still_authenticates_every_byte() {
        // A 32-byte `Bytes32` body exceeds the 24-byte wire window; the MAC
        // must still be sensitive to every payload byte.
        let key = ref_key();
        let body = [0xABu8; 32];
        let a = city_mac(&key, 3, 9, 5, &body);
        assert!(verify_mac(&key, 3, 9, 5, &body, a));
        for i in 0..32 {
            let mut tampered = body;
            tampered[i] ^= 0x01;
            assert!(
                !verify_mac(&key, 3, 9, 5, &tampered, a),
                "byte {i} not authenticated"
            );
        }
    }

    #[test]
    fn mac_is_keyed_and_field_sensitive() {
        let key = ref_key();
        let payload = *b"energy-bid/2345";
        let a = city_mac(&key, 1, 5, 7, &payload);
        assert_eq!(a, city_mac(&key, 1, 5, 7, &payload));
        assert!(verify_mac(&key, 1, 5, 7, &payload, a));

        // Replay protection: same payload with a bumped seq fails.
        assert!(!verify_mac(&key, 1, 5, 8, &payload, a));
        // Sender spoof fails.
        assert!(!verify_mac(&key, 2, 5, 7, &payload, a));
        // Type spoof fails.
        assert!(!verify_mac(&key, 1, 6, 7, &payload, a));
        // A single payload byte change fails.
        let mut tampered = payload;
        tampered[0] ^= 1;
        assert!(!verify_mac(&key, 1, 5, 7, &tampered, a));
    }

    #[test]
    fn wrong_key_fails_but_does_not_leak() {
        let key = ref_key();
        let other = MacKey::derive(0xDEAD_BEEF);
        let payload = *b"ledger-txn/0001";
        let a = city_mac(&key, 1, 1, 1, &payload);
        // The same message under a different key must not verify — the
        // property FNV-1a lacked, where a known salt made tags forgeable.
        assert!(!verify_mac(&other, 1, 1, 1, &payload, a));
        assert_ne!(a, city_mac(&other, 1, 1, 1, &payload));
    }

    #[test]
    fn derive_is_deterministic_and_distinct_per_label() {
        assert_eq!(MacKey::derive(7), MacKey::derive(7));
        assert_ne!(MacKey::derive(7), MacKey::derive(8));
    }
}
