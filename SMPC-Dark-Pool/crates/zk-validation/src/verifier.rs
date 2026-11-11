use ark_bn254::{Bn254, Fr};
use ark_groth16::Groth16;
use ark_groth16::PreparedVerifyingKey;
use ark_groth16::Proof as Groth16Proof;
use ark_relations::r1cs::SynthesisError;
use ark_serialize::CanonicalDeserialize;

#[derive(Debug, Clone)]
pub struct Proof {
    pub proof_bytes: Vec<u8>,
    pub public_inputs: Vec<Fr>,
}

pub struct Verifier {
    pub pvk: PreparedVerifyingKey<Bn254>,
}

impl Verifier {
    pub fn new(pvk: PreparedVerifyingKey<Bn254>) -> Self {
        Self { pvk }
    }

    pub fn verify(&self, proof: &Proof) -> Result<bool, SynthesisError> {
        let groth16_proof = Groth16Proof::<Bn254>::deserialize_compressed(&proof.proof_bytes[..])
            .map_err(|_| SynthesisError::AssignmentMissing)?;
        Groth16::<Bn254>::verify_proof(&self.pvk, &groth16_proof, &proof.public_inputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuits::BalanceCircuit;
    use ark_bn254::Bn254;
    use ark_groth16::Groth16;
    use ark_groth16::ProvingKey;
    use ark_groth16::prepare_verifying_key;
    use ark_serialize::CanonicalSerialize;
    use rand::thread_rng;

    fn setup() -> (ProvingKey<Bn254>, PreparedVerifyingKey<Bn254>) {
        let dummy = BalanceCircuit {
            balance: Default::default(),
            nonce: Default::default(),
            balance_commitment: Default::default(),
            order_value: Default::default(),
        };
        let mut rng = thread_rng();
        let pk =
            Groth16::<Bn254>::generate_random_parameters_with_reduction(dummy, &mut rng).unwrap();
        let pvk = prepare_verifying_key(&pk.vk);
        (pk, pvk)
    }

    fn make_proof(
        pk: &ProvingKey<Bn254>,
        balance: Fr,
        nonce: Fr,
        balance_commitment: Fr,
        order_value: Fr,
    ) -> Proof {
        let circuit = BalanceCircuit {
            balance,
            nonce,
            balance_commitment,
            order_value,
        };
        let mut rng = thread_rng();
        let groth16_proof =
            Groth16::<Bn254>::create_random_proof_with_reduction(circuit, pk, &mut rng).unwrap();
        let mut proof_bytes = Vec::new();
        groth16_proof
            .serialize_compressed(&mut proof_bytes)
            .unwrap();
        Proof {
            proof_bytes,
            public_inputs: vec![balance_commitment, order_value],
        }
    }

    #[test]
    fn test_verifier_accepts_valid_proof() {
        let (pk, pvk) = setup();
        let verifier = Verifier::new(pvk);
        let balance = Fr::from(1000u64);
        let nonce = Fr::from(12345u64);
        let commitment = BalanceCircuit::compute_commitment(balance, nonce);
        let proof = make_proof(&pk, balance, nonce, commitment, Fr::from(100u64));
        assert!(verifier.verify(&proof).unwrap());
    }

    #[test]
    fn test_verifier_rejects_tampered_proof() {
        let (pk, pvk) = setup();
        let verifier = Verifier::new(pvk);
        let balance = Fr::from(1000u64);
        let nonce = Fr::from(12345u64);
        let commitment = BalanceCircuit::compute_commitment(balance, nonce);
        let mut proof = make_proof(&pk, balance, nonce, commitment, Fr::from(100u64));
        if !proof.proof_bytes.is_empty() {
            proof.proof_bytes[0] ^= 0xff;
        }
        let result = verifier.verify(&proof);
        if let Ok(valid) = result {
            assert!(!valid)
        }
    }
}
