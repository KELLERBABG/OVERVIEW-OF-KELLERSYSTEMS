//! Cryptographic helpers used by the API.
//!
//! The API is intentionally limited — it never sees raw genomic data —
//! but it must compute a deterministic SHA-3-512 hash of a submitted
//! proof blob to use as a key and replay-protection token.

use sha3::{Digest, Sha3_512};

/// Compute the SHA-3-512 hash of `data` and return it as lowercase hex.
pub fn sha3_512_hex(data: &[u8]) -> String {
    let mut hasher = Sha3_512::new();
    hasher.update(data);
    let out = hasher.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha3_512_is_deterministic() {
        assert_eq!(sha3_512_hex(b"hello"), sha3_512_hex(b"hello"));
    }

    #[test]
    fn sha3_512_output_is_128_hex_chars() {
        assert_eq!(sha3_512_hex(b"x").len(), 128);
    }

    #[test]
    fn sha3_512_distinguishes_inputs() {
        assert_ne!(sha3_512_hex(b""), sha3_512_hex(b"a"));
    }
}
