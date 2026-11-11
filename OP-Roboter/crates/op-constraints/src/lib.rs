//! # op-constraints — Kinetic path constraint generation
//!
//! Translates a planned motion path (tool motion, incision depth, retraction
//! force, tool orientation) and the fixed surgical environment into a boolean
//! expression describing the *entire* action path. The expression is a
//! constraint formula that `op-verifier` hands to the SMT solver: a fault
//! state is reachable if and only if the formula is satisfiable.
//!
//! The expression language is deliberately small and Z3-friendly:
//! `And`, `Or`, `Not`, and scalar comparisons of affine atomic expressions.
//!
//! > **Research tooling only — not a deployable medical device.**

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod expr;
mod generate;

pub use expr::{BinaryOp, Comparison, ConstraintExpr, ConstraintSet, Evaluation, eval};
pub use generate::{ConstraintError, PathConstraintGenerator};