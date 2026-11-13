//! Verification Intermediate Representation (VIR).
//!
//! SSA Hypergraphs, C-RMM model, and obligation tracking.

pub mod ast;
pub mod bridge;
pub mod crmm;
pub mod effect;
pub mod heal;
pub mod hypergraph;
pub mod induct;
pub mod mir_bridge;
pub mod obligations;
pub mod opt;
pub mod portfolio;
pub mod smt;

pub use ast::*;
pub use bridge::*;
pub use effect::*;
pub use heal::*;
pub use hypergraph::*;
pub use induct::*;
pub use obligations::*;
pub use opt::*;
pub use portfolio::*;
pub use smt::*;
