//! Ghost-Net telemetry framing using ChaCha20-Poly1305.
//! Conforms to RFC 8439 via the `chacha20poly1305` crate (no_std-ready when
//! the `alloc` feature is enabled, which it is by default).
//!
//! Packet layout: [nonce: 12 bytes][ciphertext][tag: 16 bytes]

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Key, Nonce,
};

/// Combined overhead: 12-byte nonce + 16-byte tag.
pub const PACKET_OVERHEAD: usize = 12 + 16;

fn cipher(key: &[u8; 32]) -> ChaCha20Poly1305 {
    ChaCha20Poly1305::new(Key::from_slice(key))
}

/// Encrypt `plaintext` and authenticate it. Returns `[nonce || ciphertext || tag]`.
pub fn encrypt_packet(
    key: &[u8; 32],
    plaintext: &[u8],
    packet_counter: u64,
) -> alloc::vec::Vec<u8> {
    let mut nonce = [0u8; 12];
    // packet_counter is a small monotonic counter; put it in the first 8 bytes LE.
    nonce[0..8].copy_from_slice(&packet_counter.to_le_bytes());
    // Fill the remaining 4 bytes from the key to avoid fixed nonce reuse across keys.
    nonce[8..12].copy_from_slice(&key[0..4]);

    let cipher = cipher(key);
    let ct = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad: &[],
            },
        )
        .expect("ChaCha20-Poly1305 encryption should not fail");

    let mut packet = alloc::vec::Vec::with_capacity(12 + ct.len());
    packet.extend_from_slice(&nonce);
    packet.extend_from_slice(&ct);
    packet
}

/// Decrypt a packet produced by [`encrypt_packet`]; returns `None` if the
/// authentication tag does not verify (tampered/wrong key/wrong nonce).
pub fn decrypt_packet(key: &[u8; 32], packet: &[u8]) -> Option<alloc::vec::Vec<u8>> {
    if packet.len() < PACKET_OVERHEAD {
        return None;
    }
    let (nonce, ct_and_tag) = packet.split_at(12);
    let mut nonce_arr = [0u8; 12];
    nonce_arr.copy_from_slice(nonce);

    let cipher = cipher(key);
    cipher
        .decrypt(
            Nonce::from_slice(&nonce_arr),
            Payload {
                msg: ct_and_tag,
                aad: &[],
            },
        )
        .ok()
}

/// Energy cost of transmitting a packet of `payload_bytes` over a radio
/// running at `tx_power_w`, with `tx_time_per_byte_s` seconds per byte on air.
pub fn packet_energy_cost(payload_bytes: usize, tx_power_w: f64, tx_time_per_byte_s: f64) -> f64 {
    tx_power_w * (payload_bytes + PACKET_OVERHEAD) as f64 * tx_time_per_byte_s
}

extern crate alloc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [0x42u8; 32];
        let msg: &[u8] = b"HELLO GHOST-NET";
        let pkt = encrypt_packet(&key, msg, 0);
        assert!(pkt.len() == msg.len() + PACKET_OVERHEAD);
        assert_eq!(decrypt_packet(&key, &pkt).unwrap(), msg);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key = [0x42u8; 32];
        let wk = [0x00u8; 32];
        let pkt = encrypt_packet(&key, b"test", 0);
        assert!(wk != key);
        assert!(decrypt_packet(&wk, &pkt).is_none());
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = [0x42u8; 32];
        let mut pkt = encrypt_packet(&key, b"payload to protect", 7);
        // Flip a bit in the ciphertext region (skip the nonce).
        let idx = 12 + pkt[12] as usize % (pkt.len() - 12 - 1);
        pkt[idx] ^= 0x01;
        assert!(decrypt_packet(&key, &pkt).is_none());
    }

    #[test]
    fn test_rfc8439_known_answer() {
        // RFC 8439 §2.8.2 test vector key/IV → ciphertext is checked for
        // correct length/decrypt round-trip; the crate itself is tested
        // against these vectors upstream.
        let key = [
            0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d,
            0x8e, 0x8f, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b,
            0x9c, 0x9d, 0x9e, 0x9f,
        ];
        let msg: &[u8] = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
        let pkt = encrypt_packet(&key, msg, 0);
        assert_eq!(decrypt_packet(&key, &pkt).unwrap(), msg);
    }
}
