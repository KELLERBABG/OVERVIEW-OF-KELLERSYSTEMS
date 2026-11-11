use pod_core::policy::DamagePolicy;
use pod_core::sensor::SensorReading;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::proof::{PrivateDamageProof, ProofSystem};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageCircuit {
    pub policy_hash: [u8; 32],
    pub verdict_hash: [u8; 32],
    pub threshold_count: usize,
    pub combination: String,
}

impl DamageCircuit {
    pub fn from_policy(policy: &DamagePolicy) -> Self {
        let policy_json = serde_json::to_string(policy).unwrap_or_default();
        let policy_hash = Sha256::digest(policy_json.as_bytes()).into();

        DamageCircuit {
            policy_hash,
            verdict_hash: [0u8; 32],
            threshold_count: policy.thresholds.len(),
            combination: format!("{:?}", policy.combination),
        }
    }

    pub fn prove(
        &self,
        readings: &[SensorReading],
        policy: &DamagePolicy,
        verdict: bool,
    ) -> Result<PrivateDamageProof, String> {
        let system = ProofSystem::new();

        let mut reading_commitments = Vec::new();
        let mut threshold_proofs = Vec::new();

        for threshold in &policy.thresholds {
            let matching: Vec<&SensorReading> = readings
                .iter()
                .filter(|r| r.sensor_type == threshold.sensor_type)
                .collect();

            if matching.is_empty() {
                continue;
            }

            let label = format!("threshold_{}", threshold.name);
            let reading = matching.last().unwrap();

            let (threshold_proof, commitment) = system.prove_threshold(
                reading.value,
                &threshold.parameter,
                &label,
            )?;
            threshold_proofs.push(threshold_proof);
            reading_commitments.push(crate::commit::CommittedReading {
                label,
                commitment,
                sensor_type: format!("{:?}", reading.sensor_type),
            });
        }

        let combination_proof = system.prove_combination(
            &threshold_proofs,
            &policy.combination,
        );

        let verdict_str = if verdict { "APPROVED" } else { "REJECTED" };
        let verdict_bytes = Sha256::digest(verdict_str.as_bytes());

        Ok(PrivateDamageProof {
            circuit: DamageCircuit {
                verdict_hash: verdict_bytes.into(),
                ..self.clone()
            },
            reading_commitments,
            threshold_proofs,
            combination_proof,
        })
    }

    pub fn verify(&self, proof: &PrivateDamageProof) -> Result<bool, String> {
        if proof.circuit.policy_hash != self.policy_hash {
            return Err("policy hash mismatch".into());
        }

        let system = ProofSystem::new();

        for (i, tp) in proof.threshold_proofs.iter().enumerate() {
            if i < proof.reading_commitments.len() {
                let committed = &proof.reading_commitments[i];
                if !system.verify_threshold(tp, &committed.commitment, &committed.label)? {
                    return Ok(false);
                }
            }
        }

        let combined = system.verify_combination(&proof.combination_proof);
        Ok(combined)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pod_core::policy::{DamagePolicy, DamageThreshold, CombinationLogic, ThresholdParameter};
    use pod_core::sensor::{SensorReading, SensorType};
    use uuid::Uuid;

    fn reading(val: f64) -> SensorReading {
        SensorReading {
            sensor_id: Uuid::nil(),
            sensor_type: SensorType::Accelerometer,
            timestamp: 1000,
            value: val,
            unit: "g".into(),
        }
    }

    fn collision_policy() -> DamagePolicy {
        DamagePolicy {
            name: "test".into(),
            thresholds: vec![DamageThreshold {
                name: "impact_g".into(),
                sensor_type: SensorType::Accelerometer,
                parameter: ThresholdParameter::GreaterThan(8.0),
                duration_ms: None,
            }],
            combination: CombinationLogic::Any,
        }
    }

    #[test]
    fn test_circuit_prove_and_verify() {
        let policy = collision_policy();
        let circuit = DamageCircuit::from_policy(&policy);
        let readings = vec![reading(12.0)];

        let proof = circuit.prove(&readings, &policy, true).unwrap();
        let valid = circuit.verify(&proof).unwrap();
        assert!(valid, "proof should verify");
    }

    #[test]
    fn test_circuit_rejects_wrong_readings() {
        let policy = collision_policy();
        let circuit = DamageCircuit::from_policy(&policy);
        let readings = vec![reading(3.0)];

        let proof = circuit.prove(&readings, &policy, false).unwrap();
        // Reading 3.0 does NOT satisfy GreaterThan(8.0), so the diff is negative.
        // The proof correctly shows the threshold is NOT met.
        let valid = circuit.verify(&proof).unwrap();
        assert!(!valid, "proof should fail for non-matching reading");
    }

    #[test]
    fn test_circuit_rejects_tampered_proof() {
        let policy = collision_policy();
        let circuit = DamageCircuit::from_policy(&policy);
        let readings = vec![reading(12.0)];

        let mut proof = circuit.prove(&readings, &policy, true).unwrap();
        // Tamper with the proof's threshold response value
        if let Some(tp) = proof.threshold_proofs.first_mut() {
            if let Some(r) = tp.responses.first_mut() {
                r.reveal_value = 3.0; // wrong value
            }
        }
        let result = circuit.verify(&proof).unwrap();
        assert!(!result, "tampered proof should fail verification");
    }
}
