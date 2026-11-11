//! # sca-contracts
//!
//! Trustless Settlement Gate.
//!
//! Smart contract logic for automated payment settlement in the
//! supply chain. Funds are held in escrow and released only when
//! the protocol proves correct, untampered delivery of freight
//! via the custody chain and SMT constraint verification.
//!
//! ## Core Concepts
//!
//! - **SettlementContract** — An escrow agreement between two parties
//! - **EscrowAccount** — Holds funds pending delivery conditions
//! - **Condition** — Delivery conditions that must be proven satisfied
//! - **Proof** — Cryptographic proof that conditions are met
//!
//! This is a pure-Rust implementation of the settlement logic that
//! can be compiled to Solidity or deployed as a native module.

pub mod conditions;
pub mod contract;
pub mod escrow;
pub mod proof;

pub use conditions::{Condition, ConditionProver};
pub use contract::{ContractStatus, ContractType, SettlementContract};
pub use escrow::EscrowAccount;
pub use proof::DeliveryProof;
