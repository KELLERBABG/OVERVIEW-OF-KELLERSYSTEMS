//! # Decentralized Identity (DID) Integration
//!
//! W3C compliant DID method `did:sybil:` implementation with
//! create, resolve, update, and deactivate operations.

pub mod manager;
pub mod resolver;
pub mod storage;

pub use manager::*;
pub use resolver::*;
pub use storage::*;