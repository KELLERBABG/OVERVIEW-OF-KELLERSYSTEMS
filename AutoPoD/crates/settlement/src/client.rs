use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::proof::SettlementPayload;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettlementResult {
    Pending,
    Submitted { tx_hash: String },
    Verified { block_number: u64 },
    Paid { amount: u64, recipient: String },
    Failed { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainState {
    pub block_number: u64,
    pub balance: u64,
    pub reports_submitted: Vec<String>,
    pub reports_paid: Vec<String>,
    pub used_signatures: Vec<Vec<u8>>,
}

pub struct MockChainClient {
    state: ChainState,
    /// address -> balance
    accounts: HashMap<String, u64>,
    /// policy_id -> coverage remaining
    coverages: HashMap<String, u64>,
}

impl MockChainClient {
    pub fn new() -> Self {
        MockChainClient {
            state: ChainState {
                block_number: 0,
                balance: 1_000_000,
                reports_submitted: Vec::new(),
                reports_paid: Vec::new(),
                used_signatures: Vec::new(),
            },
            accounts: HashMap::new(),
            coverages: HashMap::new(),
        }
    }

    pub fn fund_account(&mut self, address: &str, amount: u64) {
        *self.accounts.entry(address.to_string()).or_insert(0) += amount;
    }

    pub fn set_coverage(&mut self, policy_id: &str, amount: u64) {
        self.coverages.insert(policy_id.to_string(), amount);
    }

    pub fn submit_report(&mut self, payload: &SettlementPayload) -> SettlementResult {
        if self
            .state
            .used_signatures
            .contains(&payload.proof.signature)
        {
            return SettlementResult::Failed { reason: "I5: signature replay detected".into() };
        }

        self.state.reports_submitted.push(payload.proof.report_id.clone());
        self.state.used_signatures.push(payload.proof.signature.clone());
        self.state.block_number += 1;

        SettlementResult::Submitted {
            tx_hash: format!("0x{}", hex::encode(&payload.proof.report_hash[..8])),
        }
    }

    pub fn verify_and_payout(&mut self, payload: &SettlementPayload) -> SettlementResult {
        if payload.payout_amount == 0 {
            return SettlementResult::Failed { reason: "I4: zero payout".into() };
        }

        let coverage = self.coverages.get(&payload.proof.policy_id).copied().unwrap_or(0);
        if payload.payout_amount > coverage {
            return SettlementResult::Failed { reason: "I4: exceeds coverage".into() };
        }

        let insured = &payload.insured_address;
        let balance = self.accounts.get(insured).copied().unwrap_or(0);
        if payload.payout_amount > balance {
            return SettlementResult::Failed { reason: "I4: insufficient balance".into() };
        }

        if self.state.reports_paid.contains(&payload.proof.report_id) {
            return SettlementResult::Failed { reason: "I3: duplicate payout".into() };
        }

        let sig_used = self
            .state
            .used_signatures
            .contains(&payload.proof.signature);
        if sig_used {
            return SettlementResult::Failed { reason: "I5: signature replay".into() };
        }

        // I1: payout is atomic with verification
        self.state.block_number += 1;
        self.state.reports_submitted.push(payload.proof.report_id.clone());
        self.state.reports_paid.push(payload.proof.report_id.clone());
        self.state.used_signatures.push(payload.proof.signature.clone());

        let account = self.accounts.get_mut(insured).unwrap();
        *account -= payload.payout_amount;
        self.state.balance += payload.payout_amount;

        SettlementResult::Paid {
            amount: payload.payout_amount,
            recipient: insured.clone(),
        }
    }

    pub fn state(&self) -> &ChainState {
        &self.state
    }
}

impl Default for MockChainClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::SettlementPayload;
    use incident_audit::generate_report_from_data;
    use pod_core::policy::{CombinationLogic, DamagePolicy, DamageThreshold, ThresholdParameter};
    use pod_core::sensor::{SensorReading, SensorType};
    use uuid::Uuid;

    fn sample_payload() -> SettlementPayload {
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
        let report = generate_report_from_data(&readings, &policy, None);
        SettlementPayload::from_report(&report, "policy-001", "0xinsured", 500)
    }

    #[test]
    fn test_full_settlement_flow() {
        let mut client = MockChainClient::new();
        client.fund_account("0xinsured", 1000);
        client.set_coverage("policy-001", 1000);

        let payload = sample_payload();
        assert!(payload.verify_invariants().is_ok());

        let result = client.verify_and_payout(&payload);
        match result {
            SettlementResult::Paid { amount, recipient } => {
                assert_eq!(amount, 500);
                assert_eq!(recipient, "0xinsured");
            }
            _ => panic!("expected Paid, got {:?}", result),
        }

        assert_eq!(client.state().reports_paid.len(), 1);
        assert_eq!(client.state().block_number, 1);
    }

    #[test]
    fn test_rejects_duplicate_payout() {
        let mut client = MockChainClient::new();
        client.fund_account("0xinsured", 1000);
        client.set_coverage("policy-001", 1000);

        let payload = sample_payload();
        let r1 = client.verify_and_payout(&payload);
        assert!(matches!(r1, SettlementResult::Paid { .. }));

        let r2 = client.verify_and_payout(&payload);
        assert!(matches!(r2, SettlementResult::Failed { .. }));
    }

    #[test]
    fn test_rejects_zero_payout() {
        let mut client = MockChainClient::new();
        client.fund_account("0xinsured", 1000);
        client.set_coverage("policy-001", 1000);

        let policy = DamagePolicy {
            name: "test".into(),
            thresholds: vec![],
            combination: CombinationLogic::Any,
        };
        let report = generate_report_from_data(&[], &policy, None);
        let payload = SettlementPayload::from_report(&report, "policy-001", "0xinsured", 0);

        let result = client.verify_and_payout(&payload);
        assert!(matches!(result, SettlementResult::Failed { .. }));
    }

    #[test]
    fn test_rejects_excessive_payout() {
        let mut client = MockChainClient::new();
        client.fund_account("0xinsured", 100);
        client.set_coverage("policy-001", 100);

        let payload = sample_payload();
        let result = client.verify_and_payout(&payload);
        assert!(matches!(result, SettlementResult::Failed { reason: _ }));
    }

    #[test]
    fn test_submit_before_payout() {
        let mut client = MockChainClient::new();
        client.fund_account("0xinsured", 1000);
        client.set_coverage("policy-001", 1000);

        let payload = sample_payload();
        let result = client.submit_report(&payload);
        assert!(matches!(result, SettlementResult::Submitted { .. }));
    }

    #[test]
    fn test_private_proof_settlement_flow() {
        use pod_core::policy::DamagePolicy;
        use pod_core::sensor::SensorReading;

        let policy = DamagePolicy {
            name: "zk-test".into(),
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
        let report = generate_report_from_data(&readings, &policy, None);
        let payload = SettlementPayload::from_report(&report, "policy-001", "0xinsured", 500);

        // Generate a zero-knowledge proof that the readings satisfy the policy
        let zk_proof = payload.generate_private_proof(&readings, &policy).unwrap();

        // Verify the zero-knowledge proof
        let valid = payload.verify_private_proof(&zk_proof, &policy).unwrap();
        assert!(valid, "ZK proof should verify");

        // Submit the proof and settle
        let mut client = MockChainClient::new();
        client.fund_account("0xinsured", 1000);
        client.set_coverage("policy-001", 1000);

        let result = client.verify_and_payout(&payload);
        assert!(matches!(result, SettlementResult::Paid { .. }));
    }

    #[test]
    fn test_private_proof_rejects_bad_readings() {
        use pod_core::policy::DamagePolicy;

        let policy = DamagePolicy {
            name: "zk-test".into(),
            thresholds: vec![DamageThreshold {
                name: "g".into(),
                sensor_type: SensorType::Accelerometer,
                parameter: ThresholdParameter::GreaterThan(50.0),
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
        let report = generate_report_from_data(&readings, &policy, None);
        let payload = SettlementPayload::from_report(&report, "policy-001", "0xinsured", 500);

        let zk_proof = payload.generate_private_proof(&readings, &policy).unwrap();
        let valid = payload.verify_private_proof(&zk_proof, &policy).unwrap();
        // Reading 12.0 does NOT satisfy GreaterThan(50.0), so the proof should fail
        assert!(!valid, "ZK proof should reject non-matching reading");
    }
}
