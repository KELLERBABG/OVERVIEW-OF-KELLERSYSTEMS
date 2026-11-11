//! # Sybil-Resistentes Core
//!
//! Core types, traits, errors, and primitives used across all
//! Proof-of-Personhood subsystems.

pub mod error;
pub mod types;
pub mod traits;
pub mod primitives;
pub mod constants;
pub mod economics;
pub mod events;
pub mod jury;

pub use error::*;
pub use types::*;
pub use traits::*;
pub use primitives::*;
pub use constants::*;
pub use economics::EconomicsEngine;
pub use economics::SlashEvent;
pub use economics::VouchingStake;

// Re-export events module — avoid conflict with types::VerificationEvent
// by exporting the manager only
pub use events::EventsManager;
pub use events::LivenessAttestation;
pub use jury::*;
