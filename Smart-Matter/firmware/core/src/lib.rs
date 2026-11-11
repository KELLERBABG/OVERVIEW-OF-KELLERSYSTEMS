#![no_std]
#![cfg_attr(feature = "embedded", feature(asm_experimental_arch))]

//! # Smart Matter Core
//!
//! Core types, traits, and protocol definitions for the Smart Matter
//! infrastructure-embedded intelligent materials system.
//!
//! ## Design
//! - Fully `no_std` compatible for embedded targets (ARM Cortex-M, RISC-V)
//! - `heapless` fixed-size containers prevent dynamic allocation
//! - `serde` for message serialization/deserialization
//! - Lightweight ChaCha20-Poly1305 for authenticated encryption
//!
//! ## Feature Flags
//! - `std`: Enable standard library (for desktop tooling, testing)
//! - `embedded`: Enable Cortex-M specific features

#[cfg(feature = "std")]
extern crate std;

pub mod broadcast;
pub mod crypto;
pub mod energy;
pub mod integrity;
pub mod memmap;
pub mod mesh;
pub mod radio;
pub mod sensor;
pub mod storage;
pub mod thermal;
pub mod types;
pub mod wire;

// Re-export commonly used items
pub use broadcast::*;
pub use crypto::*;
pub use energy::*;
pub use integrity::*;
pub use mesh::*;
pub use sensor::*;
pub use storage::*;
pub use thermal::*;
pub use types::*;
pub use wire::*;
