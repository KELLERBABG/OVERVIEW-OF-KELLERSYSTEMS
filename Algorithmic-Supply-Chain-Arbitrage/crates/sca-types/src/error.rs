//! Common error types for the platform.

use std::fmt;

/// Errors that can occur in domain operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ScaError {
    /// An entity with the given ID was not found.
    NotFound(String),
    /// A validation constraint was violated.
    ValidationError(String),
    /// An edge would create a cycle in a DAG.
    CycleDetected(String),
    /// A constraint evaluation failed due to missing variables.
    MissingVariables(String),
}

impl fmt::Display for ScaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "not found: {msg}"),
            Self::ValidationError(msg) => write!(f, "validation error: {msg}"),
            Self::CycleDetected(msg) => write!(f, "cycle detected: {msg}"),
            Self::MissingVariables(msg) => write!(f, "missing variables: {msg}"),
        }
    }
}

impl std::error::Error for ScaError {}