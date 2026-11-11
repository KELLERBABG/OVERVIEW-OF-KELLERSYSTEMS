//! # Symbolic Execution Engine — Core Library
//!
//! A formal verification engine for smart contracts using abstract interpretation,
//! symbolic execution, SMT solving (Z3), path constraint generation, vulnerability
//! pattern matching, taint analysis, and counter-example generation.
//!
//! ## Architecture
//!
//! The engine is organized into the following modules:
//!
//! - **[`types`]**: Core domain types (addresses, values, execution state, errors)
//! - **[`evm`]**: EVM bytecode parsing, opcode definitions, symbolic executor, gas costs
//! - **[`ir`]**: Intermediate representation & control flow graph construction
//! - **[`abstract_interpretation`]**: Abstract domain engine for symbolic state
//! - **[`path_constraints`]**: Branch condition collection and path constraint building
//! - **[`state_explorer`]**: State space exploration strategies (BFS, DFS, heuristic)
//! - **[`smt`]**: SMT solver interface (SMT-LIB translation, Z3 integration)
//! - **[`verification`]**: Formal verification of invariants and properties
//! - **[`counter_examples`]**: Exploit input synthesis from SMT models
//! - **[`vulnerabilities`]**: Vulnerability pattern matching (reentrancy, overflow, etc.)
//! - **[`gas`]**: Gas consumption analysis and unbounded loop detection
//! - **[`taint`]**: Taint analysis for untrusted input flow tracking
//! - **[`spec_gen`]**: Automated formal specification generation
//! - **[`analysis`]**: Top-level analysis orchestration

// Public API
pub mod types;
pub mod evm;
pub mod ir;
pub mod abstract_interpretation;
pub mod path_constraints;
pub mod state_explorer;
pub mod smt;
pub mod verification;
pub mod counter_examples;
pub mod vulnerabilities;
pub mod gas;
pub mod taint;
pub mod spec_gen;
pub mod analysis;

// New high-value architectural modules
pub mod concolic;
pub mod z3_backend;
pub mod hash;

// Re-export key types for convenience
pub use analysis::{AnalysisEngine, AnalysisConfig, AnalysisReport};
pub use types::{
    Address, ExecutionError, ExecutionState, ExecutionStatus,
    SymbolicValue, SymExpr, SymVarId, U256,
};
pub use evm::{
    executor::SymbolicExecutor,
    parser::ParsedBytecode,
    opcodes::Instruction,
};
pub use ir::{build_ir, ContractIR, ControlFlowGraph};
pub use smt::{SmtSolver, SmtConfig, SmtResult};
pub use vulnerabilities::{VulnerabilityScanner, ScanResult, Vulnerability};
pub use hash::SymbolicHashTable;
pub use concolic::{ConcolicEngine, ConcolicConfig, ConcreteEVM, ConcreteTrace};
pub use z3_backend::{Z3Solver, SolverBackend, SmtLibBackend, Z3NativeBackend};

/// The current version of the symbolic execution engine.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_exists() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_core_types_reexported() {
        let _addr = Address::zero();
        let _val = SymbolicValue::concrete(42);
        let _state = ExecutionState::default();
    }
}