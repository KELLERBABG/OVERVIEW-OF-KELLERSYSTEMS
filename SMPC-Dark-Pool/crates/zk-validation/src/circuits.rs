pub use ark_bn254::Fr;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CircuitError {
    #[error("insufficient balance: balance={balance} < order_value={order_value}")]
    InsufficientBalance { balance: Fr, order_value: Fr },
    #[error("synthesis error: {0}")]
    Synthesis(#[from] SynthesisError),
}

#[derive(Clone)]
pub struct BalanceCircuit {
    pub balance: Fr,
    pub nonce: Fr,
    pub balance_commitment: Fr,
    pub order_value: Fr,
}

impl ConstraintSynthesizer<Fr> for BalanceCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let balance_commitment =
            FpVar::<Fr>::new_input(cs.clone(), || Ok(self.balance_commitment))?;
        let order_value = FpVar::<Fr>::new_input(cs.clone(), || Ok(self.order_value))?;

        let balance = FpVar::<Fr>::new_witness(cs.clone(), || Ok(self.balance))?;
        let nonce = FpVar::<Fr>::new_witness(cs.clone(), || Ok(self.nonce))?;

        let computed_commit = &balance * &nonce + &balance;
        computed_commit.enforce_equal(&balance_commitment)?;

        let balance_bits = balance.to_bits_le()?;
        for bit in balance_bits.iter().skip(64) {
            bit.enforce_equal(&Boolean::<Fr>::FALSE)?;
        }
        let order_bits = order_value.to_bits_le()?;
        for bit in order_bits.iter().skip(64) {
            bit.enforce_equal(&Boolean::<Fr>::FALSE)?;
        }
        let diff = &balance - &order_value;
        let diff_bits = diff.to_bits_le()?;
        for bit in diff_bits.iter().skip(64) {
            bit.enforce_equal(&Boolean::<Fr>::FALSE)?;
        }

        Ok(())
    }
}

impl BalanceCircuit {
    pub fn compute_commitment(balance: Fr, nonce: Fr) -> Fr {
        balance * nonce + balance
    }
}

#[derive(Debug, Clone)]
pub struct BalanceWitness {
    pub balance: Fr,
    pub nonce: Fr,
    pub balance_commitment: Fr,
}

impl BalanceWitness {
    pub fn new(balance: Fr, nonce: Fr) -> Self {
        let balance_commitment = BalanceCircuit::compute_commitment(balance, nonce);
        Self {
            balance,
            nonce,
            balance_commitment,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_balance() {
        let balance = Fr::from(1000u64);
        let nonce = Fr::from(12345u64);
        let commitment = BalanceCircuit::compute_commitment(balance, nonce);
        let circuit = BalanceCircuit {
            balance,
            nonce,
            balance_commitment: commitment,
            order_value: Fr::from(100u64),
        };
        let cs = ark_relations::r1cs::ConstraintSystem::new_ref();
        circuit.clone().generate_constraints(cs.clone()).unwrap();
        assert!(cs.is_satisfied().unwrap());
    }

    #[test]
    fn test_insufficient_balance() {
        let balance = Fr::from(50u64);
        let nonce = Fr::from(12345u64);
        let commitment = BalanceCircuit::compute_commitment(balance, nonce);
        let circuit = BalanceCircuit {
            balance,
            nonce,
            balance_commitment: commitment,
            order_value: Fr::from(100u64),
        };
        let cs = ark_relations::r1cs::ConstraintSystem::new_ref();
        circuit.clone().generate_constraints(cs.clone()).unwrap();
        assert!(!cs.is_satisfied().unwrap());
    }

    #[test]
    fn test_commitment_consistency() {
        let balance = Fr::from(500u64);
        let nonce = Fr::from(99999u64);
        let comm1 = BalanceCircuit::compute_commitment(balance, nonce);
        let comm2 = BalanceCircuit::compute_commitment(balance, nonce);
        assert_eq!(comm1, comm2);
    }
}
