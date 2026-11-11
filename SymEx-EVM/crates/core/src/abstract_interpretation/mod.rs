//! Abstract Interpretation Layer
//!
//! Replaces concrete values with symbolic variables to model EVM state
//! (stack, memory, storage) over abstract domains. Provides the foundation
//! for the symbolic execution engine by lifting all operations into the
//! symbolic domain.

use crate::types::{ExecutionState, SymbolicValue, SymVarId};

/// The abstract domain used for interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbstractDomain {
    /// Full symbolic domain (all values are symbolic variables or expressions).
    Symbolic,
    /// Interval domain (track min/max possible values for integers).
    Interval,
    /// Constant-propagation domain (track only concrete values).
    Concrete,
}

/// Configuration for the abstract interpretation layer.
#[derive(Debug, Clone)]
pub struct AbstractConfig {
    /// Which abstract domain to use.
    pub domain: AbstractDomain,
    /// Maximum depth of symbolic expression trees before simplification.
    pub max_expr_depth: usize,
    /// Whether to use widening for loop analysis.
    pub use_widening: bool,
}

impl Default for AbstractConfig {
    fn default() -> Self {
        Self {
            domain: AbstractDomain::Symbolic,
            max_expr_depth: 64,
            use_widening: true,
        }
    }
}

/// The abstract interpretation engine.
///
/// Wraps an `ExecutionState` and lifts all operations to work over
/// symbolic values instead of concrete ones.
#[derive(Debug, Clone)]
pub struct AbstractInterpreter {
    /// Current symbolic state.
    pub state: ExecutionState,
    /// Configuration.
    config: AbstractConfig,
    /// Fresh symbolic variable counter.
    sym_counter: u64,
}

impl AbstractInterpreter {
    /// Create a new abstract interpreter with the given config.
    pub fn new(config: AbstractConfig) -> Self {
        Self {
            state: ExecutionState::default(),
            config,
            sym_counter: 0,
        }
    }

    /// Create a new fresh symbolic variable.
    pub fn fresh_variable(&mut self) -> SymbolicValue {
        let id = SymVarId(self.sym_counter);
        self.sym_counter += 1;
        SymbolicValue::fresh(id)
    }

    /// Initialize the interpreter state from calldata bytes.
    /// Marks each 32-byte word of calldata as a symbolic variable.
    pub fn initialize_from_calldata(&mut self, calldata_sym_count: usize) {
        for _ in 0..calldata_sym_count {
            let sym = self.fresh_variable();
            self.state.calldata.push(sym);
        }
        // Also symbolize the callvalue
        self.state.callvalue = self.fresh_variable();
    }

    /// Get the current execution state.
    pub fn get_state(&self) -> &ExecutionState {
        &self.state
    }

    /// Get mutable access to the execution state.
    pub fn get_state_mut(&mut self) -> &mut ExecutionState {
        &mut self.state
    }

    /// Reset the interpreter to a fresh state.
    pub fn reset(&mut self) {
        self.state = ExecutionState::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh_variable() {
        let mut ai = AbstractInterpreter::new(AbstractConfig::default());
        let v1 = ai.fresh_variable();
        let v2 = ai.fresh_variable();
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_initialize_calldata() {
        let mut ai = AbstractInterpreter::new(AbstractConfig::default());
        ai.initialize_from_calldata(4);
        assert_eq!(ai.state.calldata.len(), 4);
    }
}