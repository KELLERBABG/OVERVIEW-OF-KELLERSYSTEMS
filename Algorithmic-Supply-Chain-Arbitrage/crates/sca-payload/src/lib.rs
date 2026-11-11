//! # sca-payload
//!
//! Taint-coded payload tracking for cargo integrity.
//!
//! Provides an end-to-end cryptographic chain-of-custody for freight
//! shipments. Each custody transfer is signed with an ed25519 key,
//! forming an immutable audit trail from producer to final consumer.
//!
//! ## How it works
//!
//! 1. A `PayloadTracker` is created for a shipment with a known origin
//! 2. At each handover point, a `CustodyRecord` is created containing
//!    the location, timestamp, custodian identity, and sensor readings
//! 3. The record is signed with the current custodian's private key
//! 4. Each record includes the hash of the previous record, forming
//!    a blockchain-like chain of custody
//! 5. At any point, the chain can be verified for integrity

pub mod chain;
pub mod integrity;
pub mod tracker;

pub use chain::{CustodyChain, CustodyRecord};
pub use integrity::IntegrityVerifier;
pub use tracker::{PayloadTracker, TrackingError};
