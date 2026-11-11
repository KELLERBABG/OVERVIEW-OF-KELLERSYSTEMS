//! Taint Analysis
//!
//! Tracks the flow of data from untrusted inputs (CALLDATA) through the
//! contract to critical state changes (SSTORE). Detects when unvalidated
//! user input reaches sensitive operations, helping identify injection,
//! authorization bypass, and state manipulation vulnerabilities.

use crate::types::{ExecutionState, SymbolicValue};
use std::collections::{BTreeMap, BTreeSet};

/// The taint status of a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaintStatus {
    /// Value is clean (not influenced by untrusted input).
    Clean,
    /// Value is tainted (influenced by untrusted input).
    Tainted,
    /// Value is partially tainted (some bits/sources are tainted).
    Partial,
}

impl TaintStatus {
    /// Combine two taint statuses (OR semantics).
    pub fn combine(self, other: TaintStatus) -> TaintStatus {
        if self == TaintStatus::Tainted || other == TaintStatus::Tainted {
            TaintStatus::Tainted
        } else if self == TaintStatus::Partial || other == TaintStatus::Partial {
            TaintStatus::Partial
        } else {
            TaintStatus::Clean
        }
    }
}

/// A taint source — where untrusted data enters the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TaintSource {
    /// CALLDATA input (function arguments).
    Calldata,
    /// CALLVALUE (msg.value).
    Callvalue,
    /// Return data from an external call.
    Returndata,
    /// Storage slot (may be set by untrusted sources).
    Storage,
    /// Block environment (TIMESTAMP, BLOCKHASH, etc. — partially trusted).
    BlockEnvironment,
}

/// A taint sink — a sensitive operation that should not receive tainted data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaintSink {
    /// SSTORE — writing to storage.
    Sstore,
    /// CALL / DELEGATECALL — making external calls.
    Call,
    /// SELFDESTRUCT — destroying the contract.
    SelfDestruct,
    /// Suicidal address setting.
    SuicidalAddress,
}

/// A taint flow — a path from source to sink.
#[derive(Debug, Clone)]
pub struct TaintFlow {
    /// The source where taint originated.
    pub source: TaintSource,
    /// The sink where tainted data was used.
    pub sink: TaintSink,
    /// The program counter where the flow was detected.
    pub pc: usize,
    /// What data was tainted.
    pub description: String,
    /// Whether the tainted data was validated before reaching the sink.
    pub validated: bool,
}

/// The taint analysis engine.
#[derive(Debug, Clone)]
pub struct TaintAnalyzer {
    /// Mapping from symbolic variable IDs to their taint status.
    pub taint_map: BTreeMap<u64, TaintStatus>,
    /// All detected taint flows.
    pub flows: Vec<TaintFlow>,
    /// The set of taint sources configured.
    pub sources: BTreeSet<TaintSource>,
    /// Whether to track partial taint (bit-level).
    pub track_partial: bool,
}

impl TaintAnalyzer {
    /// Create a new taint analyzer.
    pub fn new() -> Self {
        let mut sources = BTreeSet::new();
        sources.insert(TaintSource::Calldata);
        sources.insert(TaintSource::Callvalue);
        sources.insert(TaintSource::Returndata);

        Self {
            taint_map: BTreeMap::new(),
            flows: Vec::new(),
            sources,
            track_partial: true,
        }
    }

    /// Mark a symbolic value as tainted.
    pub fn mark_tainted(&mut self, var_id: u64) {
        self.taint_map.insert(var_id, TaintStatus::Tainted);
    }

    /// Check if a symbolic value is tainted.
    pub fn is_tainted(&self, value: &SymbolicValue) -> TaintStatus {
        match value {
            SymbolicValue::Symbolic(id) => {
                self.taint_map
                    .get(&id.0)
                    .copied()
                    .unwrap_or(TaintStatus::Clean)
            }
            SymbolicValue::Expression(expr) => {
                // Recursively check expression operands
                match expr.as_ref() {
                    crate::types::SymExpr::Add(a, b)
                    | crate::types::SymExpr::Sub(a, b)
                    | crate::types::SymExpr::Mul(a, b)
                    | crate::types::SymExpr::Div(a, b)
                    | crate::types::SymExpr::Mod(a, b)
                    | crate::types::SymExpr::And(a, b)
                    | crate::types::SymExpr::Or(a, b)
                    | crate::types::SymExpr::Xor(a, b)
                    | crate::types::SymExpr::Shl(a, b)
                    | crate::types::SymExpr::Shr(a, b) => {
                        self.is_tainted(a).combine(self.is_tainted(b))
                    }
                    crate::types::SymExpr::Not(a)
                    | crate::types::SymExpr::Keccak256(a)
                    | crate::types::SymExpr::Sload(a)
                    | crate::types::SymExpr::CalldataLoad(a) => self.is_tainted(a),
                    crate::types::SymExpr::Byte(_, a) => self.is_tainted(a),
                    crate::types::SymExpr::Eq(a, b)
                    | crate::types::SymExpr::Lt(a, b)
                    | crate::types::SymExpr::Gt(a, b) => {
                        self.is_tainted(a).combine(self.is_tainted(b))
                    }
                }
            }
            SymbolicValue::Concrete(_) => TaintStatus::Clean,
        }
    }

    /// Record a taint flow from source to sink.
    pub fn record_flow(
        &mut self,
        source: TaintSource,
        sink: TaintSink,
        pc: usize,
        description: String,
    ) {
        self.flows.push(TaintFlow {
            source,
            sink,
            pc,
            description,
            validated: false,
        });
    }

    /// Run taint analysis over a set of execution states.
    ///
    /// In the full implementation, this walks all states and checks
    /// if any tainted value reaches a sink.
    pub fn analyze(&mut self, states: &[ExecutionState]) -> Vec<TaintFlow> {
        // Stub: returns any flows already recorded.
        self.flows.clone()
    }
}

impl Default for TaintAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taint_status_combine() {
        assert_eq!(
            TaintStatus::Clean.combine(TaintStatus::Clean),
            TaintStatus::Clean
        );
        assert_eq!(
            TaintStatus::Clean.combine(TaintStatus::Tainted),
            TaintStatus::Tainted
        );
        assert_eq!(
            TaintStatus::Tainted.combine(TaintStatus::Partial),
            TaintStatus::Tainted
        );
    }

    #[test]
    fn test_mark_and_check() {
        let mut analyzer = TaintAnalyzer::new();
        analyzer.mark_tainted(0);
        let val = SymbolicValue::fresh(crate::types::SymVarId(0));
        assert_eq!(analyzer.is_tainted(&val), TaintStatus::Tainted);
    }

    #[test]
    fn test_concrete_not_tainted() {
        let analyzer = TaintAnalyzer::new();
        let val = SymbolicValue::concrete(42);
        assert_eq!(analyzer.is_tainted(&val), TaintStatus::Clean);
    }

    #[test]
    fn test_record_flow() {
        let mut analyzer = TaintAnalyzer::new();
        analyzer.record_flow(
            TaintSource::Calldata,
            TaintSink::Sstore,
            42,
            "unvalidated calldata written to storage".to_string(),
        );
        assert_eq!(analyzer.flows.len(), 1);
    }
}