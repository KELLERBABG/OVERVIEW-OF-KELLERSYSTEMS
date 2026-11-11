use incident_audit::DamageReport;
use pod_core::policy::DamagePolicy;
use pod_core::sensor::SensorReading;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementProof {
    pub report_id: String,
    pub policy_id: String,
    pub report_hash: [u8; 32],
    pub nonce: [u8; 16],
    pub signature: Vec<u8>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementPayload {
    pub proof: SettlementProof,
    pub insured_address: String,
    pub payout_amount: u64,
    pub classification: String,
    pub verdict: String,
}

impl SettlementPayload {
    pub fn from_report(
        report: &DamageReport,
        policy_id: &str,
        insured_address: &str,
        payout_amount: u64,
    ) -> Self {
        let report_json = serde_json::to_string(report).unwrap_or_default();
        let report_hash: [u8; 32] = Sha256::digest(report_json.as_bytes()).into();

        let nonce = Uuid::new_v4();
        let signature_data = [
            &report_hash[..],
            policy_id.as_bytes(),
            insured_address.as_bytes(),
            nonce.as_bytes(),
        ]
        .concat();
        let signature = Sha256::digest(&signature_data).to_vec();

        SettlementPayload {
            proof: SettlementProof {
                report_id: report.report_id.clone(),
                policy_id: policy_id.to_string(),
                report_hash,
                nonce: *nonce.as_bytes(),
                signature,
                timestamp: report.generated_at,
            },
            insured_address: insured_address.to_string(),
            payout_amount,
            classification: report.classification.label.clone(),
            verdict: report.classification.verdict.clone(),
        }
    }

    pub fn generate_private_proof(
        &self,
        readings: &[SensorReading],
        policy: &DamagePolicy,
    ) -> Result<privacy::PrivateDamageProof, String> {
        let circuit = privacy::DamageCircuit::from_policy(policy);
        let approved = self.verdict == "APPROVED";
        circuit.prove(readings, policy, approved)
    }

    pub fn verify_private_proof(
        &self,
        proof: &privacy::PrivateDamageProof,
        policy: &DamagePolicy,
    ) -> Result<bool, String> {
        let circuit = privacy::DamageCircuit::from_policy(policy);
        circuit.verify(proof)
    }

    pub fn verify_invariants(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.payout_amount == 0 {
            errors.push("I4 violated: payout amount must be > 0".into());
        }

        let signature_data = [
            &self.proof.report_hash[..],
            self.proof.policy_id.as_bytes(),
            self.insured_address.as_bytes(),
            &self.proof.nonce[..],
        ]
        .concat();
        let expected_sig = Sha256::digest(&signature_data).to_vec();
        if self.proof.signature != expected_sig {
            errors.push("I5 violated: invalid signature".into());
        }

        if self.proof.timestamp == 0 {
            errors.push("I1 violated: timestamp must be set".into());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use incident_audit::generate_report_from_data;
    use pod_core::policy::{CombinationLogic, DamagePolicy, DamageThreshold, ThresholdParameter};
    use pod_core::sensor::{SensorReading, SensorType};
    use uuid::Uuid;

    fn sample_report() -> DamageReport {
        let policy = DamagePolicy {
            name: "test".into(),
            thresholds: vec![DamageThreshold {
                name: "g".into(),
                sensor_type: SensorType::Accelerometer,
                parameter: ThresholdParameter::GreaterThan(5.0),
                duration_ms: None,
            }],
            combination: CombinationLogic::Any,
        };
        let readings = vec![SensorReading {
            sensor_id: Uuid::nil(),
            sensor_type: SensorType::Accelerometer,
            timestamp: 1000,
            value: 12.0,
            unit: "g".into(),
        }];
        generate_report_from_data(&readings, &policy, None)
    }

    #[test]
    fn test_payload_construction() {
        let report = sample_report();
        let payload = SettlementPayload::from_report(&report, "policy-001", "0xinsured", 1000);
        assert_eq!(payload.payout_amount, 1000);
        assert_eq!(payload.insured_address, "0xinsured");
        assert!(payload.proof.report_hash.iter().any(|&b| b != 0));
    }

    #[test]
    fn test_invariants_pass() {
        let report = sample_report();
        let payload = SettlementPayload::from_report(&report, "p1", "0xa", 500);
        assert!(payload.verify_invariants().is_ok());
    }

    #[test]
    fn test_invariants_fail_zero_payout() {
        let report = sample_report();
        let payload = SettlementPayload::from_report(&report, "p1", "0xa", 0);
        assert!(payload.verify_invariants().is_err());
    }

    #[test]
    fn test_report_hash_consistent() {
        let report = sample_report();
        let p1 = SettlementPayload::from_report(&report, "p1", "0xa", 100);
        let p2 = SettlementPayload::from_report(&report, "p1", "0xa", 100);
        assert_eq!(p1.proof.report_hash, p2.proof.report_hash);
        assert_ne!(p1.proof.signature, p2.proof.signature);
    }

    #[test]
    fn test_different_policy_different_signature() {
        let report = sample_report();
        let p1 = SettlementPayload::from_report(&report, "policy-a", "0xa", 100);
        let p2 = SettlementPayload::from_report(&report, "policy-b", "0xa", 100);
        assert_ne!(p1.proof.signature, p2.proof.signature);
    }
}
