//! Real API handlers wired to the Sybil engines.
//!
//! Each handler accepts JSON, calls the relevant crate,
//! and returns structured responses with proper error handling.

pub mod graph;
pub mod nodes;
pub mod wot;
pub mod sybil;
pub mod reputation;
pub mod did;
pub mod nullifier;