//! C Frontend with C-RMM Memory Safety Axioms.
//!
//! Parses C source into an AST, lowers malloc/free and pointer arithmetic
//! to C-RMM capabilities, and injects spatial and temporal VIR obligations.

pub mod ast;
pub mod lower;
pub mod parser;

pub use ast::*;
pub use lower::{lower, SsaFunction};
pub use parser::parse;
