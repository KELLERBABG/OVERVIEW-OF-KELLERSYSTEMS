//! Cryptographic delivery proof — evidence package for settlement.

use sca_payload::chain::CustodyRecord;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::conditions::{Condition, ConditionProver};

/// A complete delivery proof package submitted to the settlement gate.
///
/// Contains all evidence needed to trigger payment release:
/// custody records, sensor readings, chain-of-custody proof.
#[derive(Debug, Clone)]
pub struct DeliveryProof {
    /// The payload/shipment ID.
    pub payload_id: Uuid,
    /// All custody records in the chain.
    pub custody_records: Vec<CustodyRecord>,
    /// SHA-256 root hash of the custody chain.
    pub chain_root_hash: Vec<u8>,
    /// Whether chain integrity was verified.
    pub chain_integrity_verified: bool,
    /// Whether all conditions are satisfied.
    pub conditions_satisfied: bool,
    /// List of condition failures (if any).
    pub condition_failures: Vec<String>,
}

impl DeliveryProof {
    /// Create a new delivery proof from custody records.
    pub fn new(payload_id: Uuid, custody_records: Vec<CustodyRecord>) -> Self {
        let chain_root = Self::compute_chain_root(&custody_records);

        Self {
            payload_id,
            custody_records,
            chain_root_hash: chain_root,
            chain_integrity_verified: false,
            conditions_satisfied: false,
            condition_failures: Vec::new(),
        }
    }

    /// Verify chain integrity and all conditions.
    pub fn verify(&mut self, conditions: &[Condition], destination_node: &Uuid) -> bool {
        // Verify chain integrity
        self.chain_integrity_verified = self.verify_chain_integrity();

        // Verify conditions
        match ConditionProver::check_all(conditions, &self.custody_records, destination_node) {
            Ok(()) => {
                self.conditions_satisfied = true;
                self.condition_failures.clear();
                true
            }
            Err(failures) => {
                self.conditions_satisfied = false;
                self.condition_failures = failures;
                false
            }
        }
    }

    /// Verify the custody chain is cryptographically intact.
    fn verify_chain_integrity(&self) -> bool {
        for (i, record) in self.custody_records.iter().enumerate() {
            let recomputed = Sha256::digest(&record.data_to_sign()).to_vec();
            if recomputed != record.hash {
                return false;
            }

            if i > 0 {
                if record.previous_hash != self.custody_records[i - 1].hash {
                    return false;
                }
            }
        }
        true
    }

    /// Compute the root hash of the custody chain (hash of concatenated record hashes).
    fn compute_chain_root(records: &[CustodyRecord]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        for record in records {
            hasher.update(&record.hash);
        }
        hasher.finalize().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sca_payload::tracker::generate_test_key;

    #[test]
    fn test_create_and_verify_proof() {
        let key = generate_test_key();
        let dest = Uuid::new_v4();
        let r1 = CustodyRecord::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1000,
            "Origin".into(),
            key.verifying_key(),
            None,
            vec![],
        )
        .sign(&key);
        let r2 = CustodyRecord::new(
            Uuid::new_v4(),
            dest,
            2000,
            "Delivered".into(),
            key.verifying_key(),
            Some(r1.hash.clone()),
            vec![],
        )
        .sign(&key);

        let mut proof = DeliveryProof::new(Uuid::new_v4(), vec![r1, r2]);

        let conditions = vec![
            Condition::DeliveryToDestination { destination_node_id: dest },
            Condition::AllSignaturesValid,
            Condition::CustodyChainIntegrity,
        ];

        assert!(proof.verify(&conditions, &dest));
        assert!(proof.chain_integrity_verified);
        assert!(proof.conditions_satisfied);
    }

    #[test]
    fn test_proof_with_failing_condition() {
        let key = generate_test_key();
        let wrong_dest = Uuid::new_v4();
        let actual_dest = Uuid::new_v4();
        let r1 = CustodyRecord::new(
            Uuid::new_v4(),
            actual_dest,
            1000,
            "Delivered".into(),
            key.verifying_key(),
            None,
            vec![],
        )
        .sign(&key);

        let mut proof = DeliveryProof::new(Uuid::new_v4(), vec![r1]);

        let conditions = vec![Condition::DeliveryToDestination { destination_node_id: wrong_dest }];

        assert!(!proof.verify(&conditions, &actual_dest));
        assert!(!proof.conditions_satisfied);
    }
}
