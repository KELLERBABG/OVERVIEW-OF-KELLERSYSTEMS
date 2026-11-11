use crate::traits::CommitmentScheme;
use ark_bn254::{Fr, G1Projective};
use ark_ff::{PrimeField, UniformRand};
use rand::thread_rng;
use rand_chacha::ChaCha12Rng;
use rand_core::SeedableRng;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PedersenError {
    #[error("point not in subgroup")]
    InvalidPoint,
}

#[derive(Debug, Clone)]
pub struct PedersenOpening {
    pub r: Fr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PedersenCommitment {
    pub c: G1Projective,
}

pub struct PedersenScheme {
    g: G1Projective,
    h: G1Projective,
}

impl PedersenScheme {
    pub fn new(g: G1Projective, h: G1Projective) -> Self {
        Self { g, h }
    }

    pub fn commit_raw(&self, value: Fr) -> (PedersenCommitment, PedersenOpening) {
        let mut rng = ChaCha12Rng::from_rng(thread_rng()).expect("failed to seed RNG");
        let r = Fr::rand(&mut rng);
        let c = self.g * value + self.h * r;
        (PedersenCommitment { c }, PedersenOpening { r })
    }

    pub fn verify_raw(
        &self,
        comm: &PedersenCommitment,
        value: Fr,
        opening: &PedersenOpening,
    ) -> bool {
        let expected = self.g * value + self.h * opening.r;
        expected == comm.c
    }
}

impl CommitmentScheme for PedersenScheme {
    type Commitment = PedersenCommitment;
    type Opening = PedersenOpening;
    type Error = PedersenError;

    fn commit(&self, value: &[u8]) -> (Self::Commitment, Self::Opening) {
        let fr_val = Fr::from_le_bytes_mod_order(value);
        self.commit_raw(fr_val)
    }

    fn verify(
        &self,
        commitment: &Self::Commitment,
        value: &[u8],
        opening: &Self::Opening,
    ) -> Result<bool, Self::Error> {
        Ok(self.verify_raw(commitment, Fr::from_le_bytes_mod_order(value), opening))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::G1Projective;
    use ark_ec::Group;
    use ark_ff::Field;

    fn setup() -> PedersenScheme {
        let g = G1Projective::generator();
        let h = g.double();
        PedersenScheme::new(g, h)
    }

    #[test]
    fn test_pedersen_commit_verify() {
        let scheme = setup();
        let value = b"order_secret_42";
        let (comm, opening) = scheme.commit(value);
        assert!(scheme.verify(&comm, value, &opening).unwrap());
    }

    #[test]
    fn test_pedersen_commit_verify_fails_wrong_value() {
        let scheme = setup();
        let value = b"order_secret_42";
        let wrong = b"order_secret_99";
        let (comm, opening) = scheme.commit(value);
        assert!(!scheme.verify(&comm, wrong, &opening).unwrap());
    }

    #[test]
    fn test_pedersen_commit_verify_fails_wrong_opening() {
        let scheme = setup();
        let (comm, _) = scheme.commit(b"secret");
        let fake_opening = PedersenOpening { r: Fr::ONE };
        assert!(!scheme.verify_raw(&comm, Fr::from_le_bytes_mod_order(b"secret"), &fake_opening));
    }
}
