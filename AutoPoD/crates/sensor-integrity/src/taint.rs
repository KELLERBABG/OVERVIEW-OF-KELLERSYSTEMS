use std::collections::HashSet;

use pod_core::sensor::SignedReading;
use serde::{Deserialize, Serialize};

use crate::chain::ChainLink;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaintReport {
    pub total_readings: usize,
    pub replay_count: usize,
    pub replay_details: Vec<ReplayEvent>,
    pub injection_count: usize,
    pub injection_details: Vec<InjectionEvent>,
    pub temporal_anomaly_count: usize,
    pub temporal_details: Vec<TemporalAnomaly>,
    pub trust_score: f64,
    pub verdict: TaintVerdict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayEvent {
    pub timestamp: i64,
    pub value: f64,
    pub occurrences: usize,
    pub positions: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionEvent {
    pub index: usize,
    pub timestamp: i64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalAnomaly {
    pub index: usize,
    pub timestamp: i64,
    pub prev_timestamp: i64,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaintVerdict {
    Clean,
    Suspicious,
    Corrupted,
}

pub fn audit_signed_readings(readings: &[SignedReading]) -> TaintReport {
    let mut replay_count = 0;
    let mut replay_details = Vec::new();
    let mut injection_count = 0;
    let mut injection_details = Vec::new();
    let mut temporal_anomaly_count = 0;
    let mut temporal_details = Vec::new();

    let mut seen_signatures: HashSet<Vec<u8>> = HashSet::new();

    for (i, sr) in readings.iter().enumerate() {
        if !seen_signatures.insert(sr.signature.clone()) {
            replay_count += 1;
            replay_details.push(ReplayEvent {
                timestamp: sr.reading.timestamp,
                value: sr.reading.value,
                occurrences: 2,
                positions: vec![i - 1, i],
            });
        }

        if sr.signature.is_empty() || sr.signer_public_key.is_empty() {
            injection_count += 1;
            injection_details.push(InjectionEvent {
                index: i,
                timestamp: sr.reading.timestamp,
                detail: if sr.signature.is_empty() {
                    "missing signature".into()
                } else {
                    "missing public key".into()
                },
            });
        }
    }

    for (i, sr) in readings.iter().enumerate().skip(1) {
        let prev = &readings[i - 1];
        if sr.reading.timestamp < prev.reading.timestamp {
            temporal_anomaly_count += 1;
            temporal_details.push(TemporalAnomaly {
                index: i,
                timestamp: sr.reading.timestamp,
                prev_timestamp: prev.reading.timestamp,
                detail: "timestamp moving backward — possible reordering or injection".into(),
            });
        }
    }

    let total = readings.len() as f64;
    let flaw_count = (replay_count + injection_count + temporal_anomaly_count) as f64;
    let trust_score = if total == 0.0 {
        1.0
    } else {
        (1.0 - (flaw_count / total).min(1.0)).max(0.0)
    };

    let verdict = if trust_score >= 0.9 {
        TaintVerdict::Clean
    } else if trust_score >= 0.5 {
        TaintVerdict::Suspicious
    } else {
        TaintVerdict::Corrupted
    };

    TaintReport {
        total_readings: readings.len(),
        replay_count,
        replay_details,
        injection_count,
        injection_details,
        temporal_anomaly_count,
        temporal_details,
        trust_score,
        verdict,
    }
}

pub fn audit_chain_links(links: &[ChainLink]) -> TaintReport {
    let signed: Vec<SignedReading> = links
        .iter()
        .map(|l| SignedReading {
            reading: l.reading.clone(),
            signature: l.signature.clone(),
            signer_public_key: Vec::new(),
        })
        .collect();
    let mut report = audit_signed_readings(&signed);

    for (i, link) in links.iter().enumerate() {
        if i > 0 && link.prev_hash != links[i - 1].hash {
            report.injection_count += 1;
            report.injection_details.push(InjectionEvent {
                index: i,
                timestamp: link.reading.timestamp,
                detail: format!(
                    "hash chain broken at link {i}: prev_hash doesn't match"
                ),
            });
        }
    }

    let total = links.len() as f64;
    let flaw_count = (report.replay_count + report.injection_count + report.temporal_anomaly_count) as f64;
    report.trust_score = if total == 0.0 {
        1.0
    } else {
        (1.0 - (flaw_count / total).min(1.0)).max(0.0)
    };

    report.verdict = if report.trust_score >= 0.9 {
        TaintVerdict::Clean
    } else if report.trust_score >= 0.5 {
        TaintVerdict::Suspicious
    } else {
        TaintVerdict::Corrupted
    };

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use pod_core::sensor::{SensorReading, SensorType};
    use uuid::Uuid;

    fn signed_reading(val: f64, ts: i64, signed: bool) -> SignedReading {
        let sig = if signed {
            let mut b = vec![0u8; 64];
            let ts_bytes = ts.to_le_bytes();
            b[..8].copy_from_slice(&ts_bytes);
            b
        } else {
            vec![]
        };
        SignedReading {
            reading: SensorReading {
                sensor_id: Uuid::nil(),
                sensor_type: SensorType::Accelerometer,
                timestamp: ts,
                value: val,
                unit: "g".into(),
            },
            signature: sig,
            signer_public_key: if signed { vec![2u8; 32] } else { vec![] },
        }
    }

    #[test]
    fn test_clean_stream() {
        let readings = vec![
            signed_reading(1.0, 1000, true),
            signed_reading(2.0, 2000, true),
            signed_reading(3.0, 3000, true),
        ];
        let report = audit_signed_readings(&readings);
        assert_eq!(report.verdict, TaintVerdict::Clean);
        assert!((report.trust_score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_detects_injection() {
        let readings = vec![
            signed_reading(1.0, 1000, true),
            signed_reading(99.9, 2000, false),
        ];
        let report = audit_signed_readings(&readings);
        assert_eq!(report.injection_count, 1);
        assert_eq!(report.verdict, TaintVerdict::Suspicious);
    }

    #[test]
    fn test_detects_replay_duplicate_signature() {
        let r1 = signed_reading(1.0, 1000, true);
        let mut r2 = signed_reading(1.0, 1000, true);
        r2.signature = r1.signature.clone();
        let report = audit_signed_readings(&[r1, r2]);
        assert_eq!(report.replay_count, 1);
    }

    #[test]
    fn test_detects_temporal_anomaly() {
        let readings = vec![
            signed_reading(1.0, 3000, true),
            signed_reading(2.0, 1000, true),
        ];
        let report = audit_signed_readings(&readings);
        assert_eq!(report.temporal_anomaly_count, 1);
    }

    #[test]
    fn test_corrupted_verdict() {
        let readings = vec![
            signed_reading(1.0, 1000, false),
            signed_reading(2.0, 2000, false),
            signed_reading(3.0, 3000, false),
            signed_reading(4.0, 4000, false),
            signed_reading(5.0, 5000, true),
        ];
        let report = audit_signed_readings(&readings);
        assert_eq!(report.verdict, TaintVerdict::Corrupted);
        assert!(report.trust_score < 0.7);
    }

    #[test]
    fn test_empty_stream() {
        let report = audit_signed_readings(&[]);
        assert_eq!(report.verdict, TaintVerdict::Clean);
        assert!((report.trust_score - 1.0).abs() < 1e-6);
    }
}
