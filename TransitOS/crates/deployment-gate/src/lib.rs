//! Trustless deployment gate — proof-gated actuator control.
//!
//! Acts as an interceptor between the control system and physical actuators.
//! Every actuator command must pass three verification stages before
//! reaching the vehicle: taint integrity, safety invariants, and collision
//! freedom. Configurable gate policies allow strict, normal, or permissive
//! enforcement. All decisions are audited.
//!
//! # Architecture
//! - [`gate`] — Interceptor and DeploymentGate
//! - [`verdict`] — GateDecision, GateVerdict with timestamps, MRM fallback
//! - [`mrm`] — Minimum Risk Maneuver controller (v1.0.0): straight-line brake, pull-over, gradual decel
//! - [`policy`] — GatePolicy (Strict, Normal, Permissive)
//! - [`audit`] — AuditLog with latency tracking
//! - [`pipeline`] — VerificationPipeline orchestrating the full check chain

pub mod audit;
pub mod gate;
pub mod mrm;
pub mod pipeline;
pub mod policy;
pub mod verdict;

pub use audit::*;
pub use gate::*;
pub use mrm::*;
pub use pipeline::*;
pub use policy::*;
pub use verdict::*;
