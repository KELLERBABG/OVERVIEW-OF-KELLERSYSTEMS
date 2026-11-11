//! # Sybil-Resistentes REST API
//!
//! Axum-based REST API server exposing endpoints for:
//! - Node management (create, get, list)
//! - Graph operations (add edge, analyze topology)
//! - Web of Trust (vouch, verify, revoke)
//! - Reputation scoring
//! - Sybil detection
//! - DID management
//! - Nullifier management
//! - Health and metrics

pub mod routes;
pub mod server;
pub mod middleware;
pub mod handlers;

pub use routes::*;
pub use server::*;
pub use middleware::*;
pub use handlers::*;
