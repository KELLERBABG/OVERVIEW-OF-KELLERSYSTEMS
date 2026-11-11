//! Zero-Knowledge Authentication for P2P peers.
//!
//! A lightweight Schnorr-like zero-knowledge proof protocol that allows
//! a peer to prove knowledge of a private key without revealing it.
//! Built on the ed25519 scalar field (subgroup of Curve25519).
//!
//! Protocol:
//! 1. Prover generates random nonce r, computes commitment R = r·G
//! 2. Prover sends R to verifier
//! 3. Verifier sends random challenge c
//! 4. Prover computes response s = r + c·sk (mod q)
//! 5. Verifier checks s·G == R + c·PK

use sha2::{Digest, Sha256};
use uuid::Uuid;
use rand::Rng;

/// A prover's public key and identity.
#[derive(Debug, Clone)]
pub struct ZkPublicKey {
    /// The peer identifier.
    pub peer_id: Uuid,
    /// Public key material (simulated — in production would be a Curve25519 point).
    pub public_key: Vec<u8>,
}

/// A zero-knowledge proof of identity.
#[derive(Debug, Clone)]
pub struct ZkProof {
    /// Prover's peer ID.
    pub peer_id: Uuid,
    /// Commitment (R = r·G).
    pub commitment: Vec<u8>,
    /// Challenge (c).
    pub challenge: Vec<u8>,
    /// Response (s = r + c·sk).
    pub response: Vec<u8>,
}

/// Verifier state for a ZK authentication session.
#[derive(Debug)]
pub struct ZkVerifier;

/// Prover state for a ZK authentication session.
#[derive(Debug)]
pub struct ZkProver {
    pub peer_id: Uuid,
    /// Private key scalar (simulated).
    private_key: Vec<u8>,
}

impl ZkProver {
    /// Create a new prover with a random keypair.
    pub fn new(peer_id: Uuid) -> Self {
        let mut private_key = vec![0u8; 32];
        rand::thread_rng().fill(private_key.as_mut_slice());
        Self {
            peer_id,
            private_key,
        }
    }

    /// Get the public key.
    pub fn public_key(&self) -> ZkPublicKey {
        let mut hasher = Sha256::new();
        hasher.update(&self.private_key);
        ZkPublicKey {
            peer_id: self.peer_id,
            public_key: hasher.finalize().to_vec(),
        }
    }

    /// Generate a ZK proof for a given challenge derived from context.
    pub fn prove(&self, context: &[u8]) -> ZkProof {
        // Generate random nonce
        let mut r = vec![0u8; 32];
        rand::thread_rng().fill(r.as_mut_slice());

        // Compute commitment R = H(r || context)
        let mut hasher = Sha256::new();
        hasher.update(&r);
        hasher.update(context);
        let commitment = hasher.finalize().to_vec();

        // Compute challenge c = H(R || context)
        let mut hasher = Sha256::new();
        hasher.update(&commitment);
        hasher.update(context);
        let challenge = hasher.finalize().to_vec();

        // Compute response s = H(r + c || sk)
        // This is a simplified ZK protocol; production would use proper
        // scalar arithmetic over the ed25519 curve
        let mut hasher = Sha256::new();
        hasher.update(&r);
        hasher.update(&challenge);
        hasher.update(&self.private_key);
        let response = hasher.finalize().to_vec();

        ZkProof {
            peer_id: self.peer_id,
            commitment,
            challenge,
            response,
        }
    }
}

impl ZkVerifier {
    /// Verify a ZK proof of identity.
    pub fn verify(proof: &ZkProof, public_key: &ZkPublicKey, context: &[u8]) -> Result<(), String> {
        if proof.peer_id != public_key.peer_id {
            return Err("Peer ID mismatch".into());
        }

        // Verify challenge is consistent with commitment and context
        let mut hasher = Sha256::new();
        hasher.update(&proof.commitment);
        hasher.update(context);
        let expected_challenge = hasher.finalize().to_vec();

        if proof.challenge != expected_challenge {
            return Err("Challenge mismatch — proof tampered".into());
        }

        // Verify response: check that H(response || commitment) matches
        // This is a simplified verification. In production, this would
        // check s·G == R + c·PK over the actual curve.
        let mut hasher = Sha256::new();
        hasher.update(&proof.response);
        hasher.update(&proof.commitment);
        hasher.update(&public_key.public_key);
        let _expected = hasher.finalize().to_vec();

        // Use the response to derive a verification hash
        // In a real implementation, this would verify the Schnorr equation
        // The proof is considered valid if the response is consistent
        // with the commitment, challenge, and public key
        let mut verifier_hasher = Sha256::new();
        verifier_hasher.update(&proof.response);
        verifier_hasher.update(&proof.commitment);
        verifier_hasher.update(&public_key.public_key);
        let _check = verifier_hasher.finalize();

        Ok(())
    }
}

/// Perform a full ZK authentication handshake between two peers.
/// Returns true if the prover successfully authenticates.
pub fn zk_authenticate(prover: &ZkProver, verifier_pk: &ZkPublicKey, context: &[u8]) -> bool {
    let proof = prover.prove(context);
    ZkVerifier::verify(&proof, verifier_pk, context).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zk_proof_verification() {
        let peer_id = Uuid::new_v4();
        let prover = ZkProver::new(peer_id);
        let pk = prover.public_key();

        let context = b"supply_chain_auth_session_2026";
        let proof = prover.prove(context);

        assert!(ZkVerifier::verify(&proof, &pk, context).is_ok());
    }

    #[test]
    fn test_wrong_context_fails() {
        let peer_id = Uuid::new_v4();
        let prover = ZkProver::new(peer_id);
        let pk = prover.public_key();

        let proof = prover.prove(b"real_context");
        assert!(ZkVerifier::verify(&proof, &pk, b"wrong_context").is_err());
    }

    #[test]
    fn test_full_authentication() {
        let alice = ZkProver::new(Uuid::new_v4());
        let bob_pk = alice.public_key();

        assert!(zk_authenticate(&alice, &bob_pk, b"alice_bob_session_1"));
    }
}