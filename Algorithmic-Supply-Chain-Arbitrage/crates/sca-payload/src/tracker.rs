//! Payload tracker — the main entry point for taint-coded tracking.

use ed25519_dalek::SigningKey;
use sca_types::payload::{CustodyEvent, Payload};
use uuid::Uuid;

use crate::chain::CustodyRecord;

/// Errors that can occur during payload tracking.
#[derive(Debug, thiserror::Error)]
pub enum TrackingError {
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    #[error("Chain hash mismatch at event {0}")]
    ChainBroken(usize),
    #[error("No signing key provided for event {0}")]
    NoSigningKey(usize),
    #[error("Payload already sealed")]
    AlreadySealed,
    #[error("Perishability window exceeded: {actual}s > {max}s")]
    PerishabilityViolation { actual: i64, max: i64 },
}

/// Tracks a payload through the supply chain with cryptographic
/// chain-of-custody.
pub struct PayloadTracker {
    /// The underlying payload being tracked.
    pub payload: Payload,
    /// The chain of custody records.
    chain: Vec<CustodyRecord>,
    /// Whether the tracking is complete (delivered).
    sealed: bool,
}

impl PayloadTracker {
    /// Create a new tracker for a payload.
    pub fn new(payload: Payload) -> Self {
        Self { payload, chain: Vec::new(), sealed: false }
    }

    /// Create a tracker with a new payload.
    pub fn create(
        product: impl Into<String>,
        quantity: f64,
        unit: impl Into<String>,
        origin: Uuid,
        destination: Uuid,
    ) -> Self {
        Self::new(Payload::new(product, quantity, unit, origin, destination))
    }

    /// Mark payload as perishable.
    pub fn with_perishability(mut self, window_secs: i64) -> Self {
        self.payload = self.payload.with_perishability(window_secs);
        self
    }

    /// Get the current custody chain.
    pub fn chain(&self) -> &[CustodyRecord] {
        &self.chain
    }

    /// The number of custody events recorded.
    pub fn event_count(&self) -> usize {
        self.chain.len()
    }

    /// Record a new custody transfer signed by the current custodian.
    /// The first event creates the origin record; subsequent events
    /// link to the previous one.
    pub fn record_transfer(
        &mut self,
        location_id: Uuid,
        custodian: impl Into<String>,
        signing_key: &SigningKey,
        sensor_readings: Vec<(String, String)>,
    ) -> Result<(), TrackingError> {
        if self.sealed {
            return Err(TrackingError::AlreadySealed);
        }

        let timestamp = chrono::Utc::now().timestamp();
        let verifying_key = signing_key.verifying_key();

        // Build the record data to sign
        let previous_hash = self.chain.last().map(|r| r.hash.clone());
        let record = CustodyRecord::new(
            self.payload.id,
            location_id,
            timestamp,
            custodian.into(),
            verifying_key,
            previous_hash,
            sensor_readings,
        );

        // Sign the record
        let record = record.sign(signing_key);

        // Update the payload's custody events
        self.payload.add_custody_event(CustodyEvent {
            location_id,
            timestamp,
            custodian: record.custodian.clone(),
            signature: Some(record.signature.to_bytes().to_vec()),
            previous_hash: Some(record.previous_hash.clone()),
        });

        self.chain.push(record);
        Ok(())
    }

    /// Verify the entire custody chain integrity.
    pub fn verify_integrity(&self) -> Result<(), TrackingError> {
        for (i, record) in self.chain.iter().enumerate() {
            // Verify signature
            record.verify_signature().map_err(|_| TrackingError::SignatureVerificationFailed)?;

            // Verify chain linkage
            if i > 0 {
                let expected_prev = self.chain[i - 1].hash.clone();
                if record.previous_hash != expected_prev {
                    return Err(TrackingError::ChainBroken(i));
                }
            }
        }
        Ok(())
    }

    /// Seal the payload as delivered. After sealing, no more transfers
    /// can be recorded.
    pub fn seal(&mut self) -> Result<(), TrackingError> {
        if self.sealed {
            return Err(TrackingError::AlreadySealed);
        }
        self.verify_integrity()?;
        self.sealed = true;
        Ok(())
    }

    /// Check if the payload has been sealed (delivered).
    pub fn is_sealed(&self) -> bool {
        self.sealed
    }

    /// Check perishability: ensure total transit time does not exceed
    /// the perishability window.
    pub fn check_perishability(&self) -> Result<(), TrackingError> {
        let window = match self.payload.perishability_window_secs {
            Some(w) => w,
            None => return Ok(()),
        };

        let transit = match self.payload.transit_time_secs() {
            Some(t) => t,
            None => return Ok(()),
        };

        if transit > window {
            return Err(TrackingError::PerishabilityViolation { actual: transit, max: window });
        }
        Ok(())
    }
}

/// Helper to generate a random signing key for testing.
pub fn generate_test_key() -> SigningKey {
    let mut bytes = [0u8; 32];
    use rand::RngCore;
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    SigningKey::from_bytes(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_payload_tracker() -> PayloadTracker {
        PayloadTracker::create("Test Goods", 100.0, "kg", Uuid::new_v4(), Uuid::new_v4()).with_perishability(86400) // 24 hours
    }

    #[test]
    fn test_record_and_verify() {
        let mut tracker = test_payload_tracker();
        let key = generate_test_key();

        tracker.record_transfer(Uuid::new_v4(), "Warehouse A", &key, vec![]).unwrap();

        assert_eq!(tracker.event_count(), 1);
        assert!(tracker.verify_integrity().is_ok());
    }

    #[test]
    fn test_chain_of_custody() {
        let mut tracker = test_payload_tracker();
        let key1 = generate_test_key();
        let key2 = generate_test_key();

        tracker.record_transfer(Uuid::new_v4(), "Farmer", &key1, vec![]).unwrap();
        tracker.record_transfer(Uuid::new_v4(), "Distributor", &key2, vec![("temp".into(), "4.5".into())]).unwrap();

        assert_eq!(tracker.event_count(), 2);
        assert!(tracker.verify_integrity().is_ok());
    }

    #[test]
    fn test_seal_prevents_more_transfers() {
        let mut tracker = test_payload_tracker();
        let key = generate_test_key();

        tracker.record_transfer(Uuid::new_v4(), "Origin", &key, vec![]).unwrap();
        tracker.seal().unwrap();

        let result = tracker.record_transfer(Uuid::new_v4(), "After seal", &key, vec![]);
        assert!(matches!(result, Err(TrackingError::AlreadySealed)));
    }

    #[test]
    fn test_tampered_chain() {
        let mut tracker = test_payload_tracker();
        let key = generate_test_key();

        tracker.record_transfer(Uuid::new_v4(), "Honest", &key, vec![]).unwrap();

        // Tamper with the record
        if let Some(record) = tracker.chain.last_mut() {
            record.custodian = "Tampered".to_string();
        }

        assert!(tracker.verify_integrity().is_err());
    }

    #[test]
    fn test_perishability_check() {
        let mut tracker = test_payload_tracker();
        let key = generate_test_key();

        // Record with a recent timestamp
        tracker.record_transfer(Uuid::new_v4(), "Origin", &key, vec![]).unwrap();

        // Perishability should pass (just created)
        assert!(tracker.check_perishability().is_ok());
    }
}
