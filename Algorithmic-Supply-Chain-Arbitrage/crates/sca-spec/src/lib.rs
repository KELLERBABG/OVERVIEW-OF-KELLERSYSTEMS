//! # sca-spec
//!
//! Automated logistics specification generator.
//!
//! Generates formal, machine-readable freight documents from real
//! movement and transshipment data. These specifications serve as
//! tamper-proof references for bills of lading, customs clearance,
//! and smart contract settlement conditions.
//!
//! ## Key types
//!
//! - **`LogisticsSpecification`** — The complete freight document
//! - **`ShipmentLeg`** — A single leg of a multi-modal shipment
//! - **`CustodyEvidence`** — Proof of custody at each transfer point
//! - **`SpecGenerator`** — Engine that produces specs from graph + payload data

pub mod generator;
pub mod spec;

pub use generator::SpecGenerator;
pub use spec::{CustodyEvidence, LogisticsSpecification, ShipmentLeg, SpecStatus};
