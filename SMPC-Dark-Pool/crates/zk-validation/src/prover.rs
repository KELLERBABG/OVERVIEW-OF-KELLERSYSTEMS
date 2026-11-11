use crate::acceleration::ProvingStrategy;
use crate::circuits::{BalanceCircuit, BalanceWitness};
use crate::verifier::Proof;
use ark_bn254::Bn254;
use ark_groth16::Groth16;
use ark_relations::r1cs::ConstraintSynthesizer;
use ark_serialize::CanonicalSerialize;

pub struct Prover {
    pub strategy: ProvingStrategy,
}

impl Prover {
    pub fn new() -> Self {
        Self {
            strategy: ProvingStrategy::Cpu,
        }
    }

    pub fn with_strategy(strategy: ProvingStrategy) -> Self {
        Self { strategy }
    }

    pub fn generate_proof(
        &self,
        witness: &BalanceWitness,
        order_value: ark_bn254::Fr,
    ) -> Result<Proof, String> {
        let circuit = BalanceCircuit {
            balance: witness.balance,
            nonce: witness.nonce,
            balance_commitment: witness.balance_commitment,
            order_value,
        };
        let cs = ark_relations::r1cs::ConstraintSystem::new_ref();
        circuit
            .clone()
            .generate_constraints(cs.clone())
            .map_err(|e| format!("constraint generation failed: {e}"))?;
        if !cs
            .is_satisfied()
            .map_err(|e| format!("satisfaction check failed: {e}"))?
        {
            return Err("Balance check failed: constraints not satisfiable".to_string());
        }
        match self.strategy {
            ProvingStrategy::Cpu => self.generate_proof_inner(&circuit),
            ProvingStrategy::Gpu => {
                tracing::warn!("GPU proving requested but not implemented, falling back to CPU");
                self.generate_proof_inner(&circuit)
            }
            ProvingStrategy::Fpga => {
                tracing::warn!("FPGA proving requested but not implemented, falling back to CPU");
                self.generate_proof_inner(&circuit)
            }
            ProvingStrategy::Remote => {
                tracing::warn!(
                    "Remote proving requested but no remote endpoint configured, falling back to CPU"
                );
                self.generate_proof_inner(&circuit)
            }
        }
    }

    pub fn generate_proof_inner(&self, circuit: &BalanceCircuit) -> Result<Proof, String> {
        let mut rng = rand::thread_rng();
        let pk =
            Groth16::<Bn254>::generate_random_parameters_with_reduction(circuit.clone(), &mut rng)
                .map_err(|e| format!("setup failed: {e}"))?;
        let groth16_proof =
            Groth16::<Bn254>::create_random_proof_with_reduction(circuit.clone(), &pk, &mut rng)
                .map_err(|e| format!("prove failed: {e}"))?;
        let mut proof_bytes = Vec::new();
        groth16_proof
            .serialize_compressed(&mut proof_bytes)
            .map_err(|e| format!("serialization failed: {e}"))?;
        Ok(Proof {
            proof_bytes,
            public_inputs: vec![circuit.balance_commitment, circuit.order_value],
        })
    }
}

impl Default for Prover {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prover_generates_proof() {
        let prover = Prover::new();
        let balance = ark_bn254::Fr::from(1000u64);
        let nonce = ark_bn254::Fr::from(12345u64);
        let witness = BalanceWitness::new(balance, nonce);
        let proof = prover.generate_proof(&witness, ark_bn254::Fr::from(100u64));
        assert!(proof.is_ok());
    }

    #[test]
    fn test_prover_rejects_insufficient_balance() {
        let prover = Prover::new();
        let balance = ark_bn254::Fr::from(50u64);
        let nonce = ark_bn254::Fr::from(12345u64);
        let witness = BalanceWitness::new(balance, nonce);
        let proof = prover.generate_proof(&witness, ark_bn254::Fr::from(100u64));
        assert!(proof.is_err());
    }
}
