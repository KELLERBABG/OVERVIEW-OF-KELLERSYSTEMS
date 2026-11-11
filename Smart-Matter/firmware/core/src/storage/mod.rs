//! # Storage Subsystem
//!
//! Smart Matter storage subsystem with two layers:
//!
//! ## 1. In-Memory Dead-Drop Store (`dead_drop`)
//!
//! Volatile fragment storage in MCU SRAM. Fragments are opportunistically
//! stored and garbage-collected by TTL. Used for active message routing.
//!
//! ## 2. Flash Wear-Leveling Store (`wear_level`)
//!
//! Log-structured persistent storage for critical state that must survive
//! power loss: routing table checkpoints, configuration parameters,
//! integrity report snapshots.
//!
//! Uses SRAM buffering + rate limiting to reduce Flash write cycles,
//! extending STM32G0 Flash lifetime to >50 years at typical update rates.

pub mod dead_drop;
pub mod wear_level;

pub use dead_drop::*;
pub use wear_level::*;
