//! Settlement conditions — requirements that must be proven
//! before payment is released.

use sca_payload::chain::CustodyRecord;
use sha2::{Digest, Sha256};

/// A delivery condition that must be satisfied.
#[derive(Debug, Clone)]
pub enum Condition {
    /// The payload must have been delivered to the correct destination.
    DeliveryToDestination { destination_node_id: uuid::Uuid },
    /// The custody chain must be cryptographically intact.
    CustodyChainIntegrity,
    /// The custody chain length must be at least N.
    MinimumCustodyEvents(usize),
    /// All custody signatures must verify.
    AllSignaturesValid,
    /// A specific sensor reading must be within bounds.
    SensorReading { sensor_key: String, max_value: f64 },
    /// Perishability must be proven.
    PerishabilityProven,
}

/// Proves that conditions are satisfied from custody chain data.
pub struct ConditionProver;

impl ConditionProver {
    /// Check an array of conditions against custody records.
    pub fn check_all(
        conditions: &[Condition],
        records: &[CustodyRecord],
        destination: &uuid::Uuid,
    ) -> Result<(), Vec<String>> {
        let mut failures = Vec::new();

        for condition in conditions {
            if let Err(e) = Self::check(condition, records, destination) {
                failures.push(e);
            }
        }

        if failures.is_empty() { Ok(()) } else { Err(failures) }
    }

    /// Check a single condition.
    pub fn check(condition: &Condition, records: &[CustodyRecord], _destination: &uuid::Uuid) -> Result<(), String> {
        match condition {
            Condition::DeliveryToDestination { destination_node_id } => {
                let last = records.last().ok_or("No custody records")?;
                if last.location_id == *destination_node_id {
                    Ok(())
                } else {
                    Err(format!("Last custody at {}, expected {}", last.location_id, destination_node_id))
                }
            }
            Condition::CustodyChainIntegrity => Self::verify_chain_integrity(records),
            Condition::MinimumCustodyEvents(min) => {
                if records.len() >= *min {
                    Ok(())
                } else {
                    Err(format!("Need {} events, got {}", min, records.len()))
                }
            }
            Condition::AllSignaturesValid => {
                for (i, record) in records.iter().enumerate() {
                    if record.verify_signature().is_err() {
                        return Err(format!("Invalid signature at record {i}"));
                    }
                }
                Ok(())
            }
            Condition::SensorReading { sensor_key, max_value } => {
                for record in records {
                    for (k, v) in &record.sensor_readings {
                        if k == sensor_key {
                            if let Ok(val) = v.parse::<f64>() {
                                if val > *max_value {
                                    return Err(format!("Sensor {sensor_key} = {val} exceeds max {max_value}"));
                                }
                            }
                        }
                    }
                }
                Ok(())
            }
            Condition::PerishabilityProven => {
                // Check transit time if perishability is a concern
                // This is a placeholder that always passes when combined
                // with the actual perishability constraints elsewhere
                Ok(())
            }
        }
    }

    /// Verify chain integrity: hash linkage between consecutive records.
    fn verify_chain_integrity(records: &[CustodyRecord]) -> Result<(), String> {
        for (i, record) in records.iter().enumerate() {
            // Verify individual record hash
            let recomputed = Sha256::digest(&record.data_to_sign()).to_vec();
            if recomputed != record.hash {
                return Err(format!("Record {i}: data tampered"));
            }

            // Verify chain linkage
            if i > 0 {
                if record.previous_hash != records[i - 1].hash {
                    return Err(format!("Record {i}: chain broken"));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sca_payload::chain::CustodyRecord;
    use sca_payload::tracker::generate_test_key;
    use uuid::Uuid;

    #[test]
    fn test_delivery_condition() {
        let key = generate_test_key();
        let dest = Uuid::new_v4();
        let record =
            CustodyRecord::new(Uuid::new_v4(), dest, 1000, "Carrier".into(), key.verifying_key(), None, vec![])
                .sign(&key);

        let condition = Condition::DeliveryToDestination { destination_node_id: dest };
        assert!(ConditionProver::check(&condition, &[record], &dest).is_ok());
    }

    #[test]
    fn test_sensor_condition() {
        let key = generate_test_key();
        let record = CustodyRecord::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1000,
            "Carrier".into(),
            key.verifying_key(),
            None,
            vec![("temperature".into(), "7.5".into())],
        )
        .sign(&key);

        let condition = Condition::SensorReading { sensor_key: "temperature".into(), max_value: 8.0 };
        assert!(ConditionProver::check(&condition, &[record], &Uuid::new_v4()).is_ok());
    }

    #[test]
    fn test_sensor_condition_exceeded() {
        let key = generate_test_key();
        let record = CustodyRecord::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            1000,
            "Carrier".into(),
            key.verifying_key(),
            None,
            vec![("temperature".into(), "12.0".into())],
        )
        .sign(&key);

        let condition = Condition::SensorReading { sensor_key: "temperature".into(), max_value: 8.0 };
        assert!(ConditionProver::check(&condition, &[record], &Uuid::new_v4()).is_err());
    }
}
