//! **BRP — Blind Randomization Protocol** (City OS identity-less service access).
//!
//! BRP lets a device access a City service without any identity being stored
//! or transmitted. It reuses the exact cryptographic foundation of KELLER OS
//! — the secp256k1 curve arithmetic (`city-crypto::ec`/`bigint`) and the
//! one-shot entropy expansion (`city-crypto::osrng`) — so the same code that
//! proves identity to the kernel (Keller Auth) can prove **token knowledge**
//! to a City service without leaking who is asking.
//!
//! Protocol:
//!   1. A service in a district issues a blinded token from a **256-bit
//!      nonce**: `token = PRF(service_id ‖ expiry_slot ‖ nonce)` where PRF
//!      is the keyed SipHash counter-mode expansion — a 32-byte value that
//!      fits one KELLER IPC message exactly. The full-width nonce means the
//!      token secret cannot be brute-forced (the old scheme's 32-bit nonce
//!      could).
//!   2. The holder later presents the token with a Schnorr-style proof of
//!      knowledge of the blinding scalar `k`:
//!        - commitment  K = k·G
//!        - challenge   c  (from the service, one-shot)
//!        - response    r = k + c·x (mod n), where x is the token-derived secret
//!   3. The service checks `r·G == K + c·V` — same equation Keller Auth uses.
//!
//! No identity, no persistent linkage: the service only ever sees blinded
//! values. Revocation is handled by the capability gate (`CAP_CITY_BRP`).

#![no_std]

#[cfg(test)]
extern crate std;

use city_caps::CAP_CITY_BRP;
use city_crypto::bigint::{cmp, from_be_bytes, mod_add_n, mod_mul_n, mod_n_reduce, to_be_bytes};
use city_crypto::ec::{self, Point, GENERATOR};
use city_crypto::{derive_entropy, U256};

/// A BRP token is exactly 32 bytes — one KELLER IPC payload.
pub const BRP_TOKEN_LEN: usize = 32;
pub const BRP_CHALLENGE_LEN: usize = 32;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BrpToken(pub [u8; BRP_TOKEN_LEN]);

impl BrpToken {
    pub const ZERO: Self = Self([0; BRP_TOKEN_LEN]);

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Capability slot required to issue/verify tokens.
pub const REQUIRED_CAP: u32 = CAP_CITY_BRP;

/// Issue a blinded token for a service within an expiry window.
///
/// Deterministic per `(service_id, expiry_slot, nonce)` — the same inputs
/// reproduce the same token, so a service can verify without storing any
/// per-holder state. The nonce is **256 bits**: it should come from an
/// entropy source (host: `city_crypto::fill_os_random`; kernel: tick
/// counter + sensor entropy), giving the token secret full 256-bit
/// security instead of the old 32-bit nonce space.
pub fn issue(service_id: u32, expiry_slot: u32, nonce: &[u8; 32]) -> BrpToken {
    // First pass: fold the full nonce into a 256-bit intermediate.
    let mut seed = to_be_bytes(&derive_entropy(nonce));
    // Second pass: XOR (not overwrite) the domain fields in, so the nonce
    // still contributes to every byte of the final seed.
    let svc = service_id ^ u32::from_le_bytes([seed[0], seed[1], seed[2], seed[3]]);
    let exp = expiry_slot ^ u32::from_le_bytes([seed[4], seed[5], seed[6], seed[7]]);
    seed[0..4].copy_from_slice(&svc.to_le_bytes());
    seed[4..8].copy_from_slice(&exp.to_le_bytes());
    let ent = derive_entropy(&seed);
    let mut bytes = [0u8; BRP_TOKEN_LEN];
    bytes.copy_from_slice(&to_be_bytes(&ent));
    BrpToken(bytes)
}

/// Verify a token against the issuing parameters.
pub fn verify(token: &BrpToken, service_id: u32, expiry_slot: u32, nonce: &[u8; 32]) -> bool {
    let expect = issue(service_id, expiry_slot, nonce);
    expect == *token
}

/// Derive the prover's "blind secret" `x` from a token (never transmitted).
pub fn token_secret(token: &BrpToken) -> U256 {
    let mut arr = [0u8; 32];
    arr.copy_from_slice(token.as_bytes());
    mod_n_reduce(&from_be_bytes(&arr))
}

/// The public point associated with a token: `V = x·G`.
pub fn token_public(token: &BrpToken) -> Point {
    let x = token_secret(token);
    ec::scalar_mul(&x, &GENERATOR)
}

/// Prover: commit to a fresh blinding scalar `k` → commitment `K = k·G`.
/// The secret `x` and the scalar `k` stay local — only K ever leaves.
pub fn commit(k: &U256) -> Point {
    let k_n = mod_n_reduce(k);
    ec::scalar_mul(&k_n, &GENERATOR)
}

/// Prover: compute the Schnorr response r = k + c·x (mod n).
/// This is the exact same computation Keller Auth performs
/// (`auth.rs::compute_response`).
pub fn compute_response(k: &U256, c: &U256, x: &U256) -> U256 {
    let k_n = mod_n_reduce(k);
    let c_n = mod_n_reduce(c);
    let x_n = mod_n_reduce(x);
    let cx = mod_mul_n(&c_n, &x_n);
    mod_add_n(&k_n, &cx)
}

/// Verifier: check `r·G == K + c·V`.
/// Returns true only if the prover knew the token secret `x`.
pub fn verify_proof(commitment: &Point, challenge: &U256, response: &U256, public: &Point) -> bool {
    let c = mod_n_reduce(challenge);
    let r = mod_n_reduce(response);

    let cvs = ec::scalar_mul(&c, public);
    let rhs = ec::point_add(commitment, &cvs);
    let lhs = ec::scalar_mul(&r, &GENERATOR);

    !lhs.is_infinity() && !rhs.is_infinity() && cmp(&lhs.x, &rhs.x) == 0 && cmp(&lhs.y, &rhs.y) == 0
}

/// One-shot end-to-end BRP proof for the common case:
/// (commitment, response) are derived internally and verified.
pub fn prove_and_verify(token: &BrpToken, challenge: &U256, k: &U256) -> bool {
    let x = token_secret(token);
    let k_n = mod_n_reduce(k);
    let commitment = ec::scalar_mul(&k_n, &GENERATOR);
    let r = compute_response(&k_n, challenge, &x);
    verify_proof(&commitment, challenge, &r, &token_public(token))
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_crypto::bigint::from_u32;
    use city_crypto::entropy_seed;

    #[test]
    fn token_issue_and_verify() {
        let nonce = entropy_seed(0xDEAD_BEEF);
        let tok = issue(0xC1, 7, &nonce);
        assert!(verify(&tok, 0xC1, 7, &nonce));
        assert!(!verify(&tok, 0xC1, 8, &nonce)); // wrong expiry
        assert!(!verify(&tok, 0xC2, 7, &nonce)); // wrong service
        assert_ne!(tok, BrpToken::ZERO);

        // The 256-bit nonce space is real: a one-bit nonce change produces
        // a completely different token (the old 32-bit LCG seeded scheme
        // made tokens guessable once the nonce space was brute-forced).
        let mut nonce2 = nonce;
        nonce2[0] ^= 1;
        assert_ne!(tok, issue(0xC1, 7, &nonce2));
    }

    #[test]
    fn schnorr_proof_roundtrip() {
        let nonce = entropy_seed(42);
        let tok = issue(1, 3, &nonce);
        let challenge = derive_entropy(&entropy_seed(0x1234_5678));
        let k = derive_entropy(&entropy_seed(0xCAFE_F00D));

        let x = token_secret(&tok);
        let commitment = commit(&k);
        let r = compute_response(&k, &challenge, &x);
        let pub_point = token_public(&tok);

        assert!(verify_proof(&commitment, &challenge, &r, &pub_point));

        // An attacker who does NOT know x (tries x = 1) fails.
        let attacker_x = from_u32(1);
        let bad = compute_response(&k, &challenge, &attacker_x);
        assert!(!verify_proof(&commitment, &challenge, &bad, &pub_point));

        // One-shot helper passes.
        assert!(prove_and_verify(&tok, &challenge, &k));
    }
}
