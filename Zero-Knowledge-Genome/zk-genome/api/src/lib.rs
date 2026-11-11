//! # zk-genome-api
//!
//! REST API server for the Zero-Knowledge Genome marketplace.
//!
//! Endpoints:
//! - `POST /api/v1/auth/{register,login}` -- researcher access tokens
//! - `POST /api/v1/queries` / `GET /api/v1/queries/{id}` -- query lifecycle
//! - `POST /api/v1/queries/{id}/proofs` -- proof submission
//! - `GET  /api/v1/stats/aggregate` -- marketplace aggregate stats
//! - `GET  /healthz` -- liveness + state introspection

#![allow(missing_docs)]
#![allow(clippy::pedantic)]
#![allow(clippy::nursery)]

pub mod models;
pub mod state;
pub mod verify;

pub mod routes;
