//! # Trustless Execution Gate — Smart Contract Layer
//!
//! Defines the on-chain contract interfaces for the Decoupled Fluid IP Network.
//!
//! Architecture:
//! - **PatentRegistry** — On-chain DAG storage (Merkle commitments only)
//! - **RoyaltySplitter** — Invariant-enforced payment distribution
//! - **VerificationOracle** — Bridge to off-chain SMT solver results with ZK proof
//! - **ExecutionGate** — Atomic blueprint release on CLEAN + PAID
//!
//! This crate provides Rust-native types and logic that mirror Solidity/ink!
//! contract implementations. In production, these would be compiled to WASM
//! (ink!) or EVM bytecode (Solidity).

pub mod registry;
pub mod royalties;
pub mod oracle;
pub mod gate;

pub use registry::*;
pub use royalties::*;
pub use oracle::*;
pub use gate::*;

pub fn init_contracts() {
    tracing::info!("dfip-contracts initialized");
}