//! # SQL Query & Relational Schema Verifier
//!
//! Parses SQL AST, lowers relations and WHERE/JOIN constraints to Keller-VIR
//! array assertions. Detects SQL Injection taint paths, DivisionByZero in
//! aggregations, NOT NULL schema violations, and unbounded recursive CTE loops.
//! Operates in audit-only mode: saves reports, never mutates source.

pub mod ast;
pub mod parser;
pub mod schema;
pub mod lower;
pub mod verify;
pub mod report;
pub mod taint;

pub use report::{AuditReport, Finding, Severity};
pub use verify::Verifier;
