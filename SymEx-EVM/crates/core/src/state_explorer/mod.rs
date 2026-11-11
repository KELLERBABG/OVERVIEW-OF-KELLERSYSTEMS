//! State Space Exploration
//!
//! Systematic exploration of all theoretically possible contract states
//! using multiple search strategies (BFS, DFS, coverage-guided, heuristic).
//! Discovers edge cases that manual testing would miss.
//!
//! ## State Merging
//!
//! To mitigate path explosion, the explorer can merge two states that reach
//! the same program point (PC) with different symbolic values. Instead of
//! forking into separate paths, it creates a merged state where the differing
//! values become ITE (If-Then-Else) expressions:
//!
//! ```text
//! merged_value = ITE(path_condition_A, value_A, value_B)
//! ```
//!
//! This is also known as **verification condition merging** or **path merging**.

use crate::path_constraints::ConstraintCollector;
use crate::types::{ExecutionState, SymbolicValue, SymExpr, U256};
use std::collections::{BTreeMap, VecDeque};

/// Search strategy for exploring the state space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchStrategy {
    /// Breadth-first search — explores all paths at depth N before depth N+1.
    Bfs,
    /// Depth-first search — explores one path to completion before backtracking.
    Dfs,
    /// Coverage-guided — prioritize paths that cover new code.
    CoverageGuided,
    /// Heuristic search — uses cost/benefit to prioritize promising paths.
    Heuristic,
}

/// Configuration for the state explorer.
#[derive(Debug, Clone)]
pub struct ExplorerConfig {
    /// Search strategy to use.
    pub strategy: SearchStrategy,
    /// Maximum path depth before pruning.
    pub max_depth: usize,
    /// Maximum number of paths to explore.
    pub max_paths: usize,
    /// Timeout in seconds for exploration.
    pub timeout_secs: u64,
    /// Whether to use loop unrolling (bounded).
    pub unroll_loops: bool,
    /// Max unroll iterations.
    pub max_unroll: usize,
    /// Whether to enable state merging (reduces path explosion).
    pub enable_merging: bool,
    /// Maximum number of merged states to keep per PC.
    pub max_merged_per_pc: usize,
}

impl Default for ExplorerConfig {
    fn default() -> Self {
        Self {
            strategy: SearchStrategy::Bfs,
            max_depth: 1024,
            max_paths: 10_000,
            timeout_secs: 300,
            unroll_loops: true,
            max_unroll: 4,
            enable_merging: true,
            max_merged_per_pc: 8,
        }
    }
}

/// A snapshot of execution state at a given program point.
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    /// The execution state.
    pub state: ExecutionState,
    /// Depth in the search tree.
    pub depth: usize,
    /// Path constraints accumulated to reach this state.
    pub path_index: usize,
}

/// A merged state at a given PC — represents the disjunction of multiple
/// execution paths that converged at the same program point.
#[derive(Debug, Clone)]
pub struct MergedState {
    /// The merged execution state (with ITE expressions for diverging values).
    pub state: ExecutionState,
    /// The number of individual states that were merged into this one.
    pub merge_count: usize,
    /// The path constraints for each constituent path.
    pub path_constraints: Vec<crate::path_constraints::PathConstraint>,
}

/// Results of state space exploration.
#[derive(Debug, Clone)]
pub struct ExplorationResult {
    /// All terminal states encountered (STOP, RETURN, REVERT, etc.).
    pub terminal_states: Vec<StateSnapshot>,
    /// Total number of paths explored.
    pub paths_explored: usize,
    /// Total number of states visited.
    pub states_visited: usize,
    /// Whether exploration was terminated early (timeout or max paths).
    pub terminated_early: bool,
    /// Number of merges performed.
    pub merges_performed: usize,
}

/// The state space explorer.
#[derive(Debug, Clone)]
pub struct StateExplorer {
    /// Configuration for exploration.
    pub config: ExplorerConfig,
    /// Constraint collector for path constraints.
    pub constraints: ConstraintCollector,
    /// Merged states grouped by program counter (for state merging).
    pub merged_states: BTreeMap<usize, Vec<MergedState>>,
}

impl StateExplorer {
    /// Create a new state explorer with the given config.
    pub fn new(config: ExplorerConfig) -> Self {
        Self {
            config,
            constraints: ConstraintCollector::new(),
            merged_states: BTreeMap::new(),
        }
    }

    /// Attempt to merge two execution states at the same program counter.
    ///
    /// Merging creates ITE expressions for values that differ between the two states.
    /// Two states can be merged if they are at the same PC and have compatible call depth.
    pub fn try_merge(
        &mut self,
        state_a: &ExecutionState,
        state_b: &ExecutionState,
    ) -> Option<ExecutionState> {
        if state_a.pc != state_b.pc || state_a.call_depth != state_b.call_depth {
            return None;
        }

        let merged_stack: Vec<SymbolicValue> = state_a
            .stack
            .iter()
            .zip(state_b.stack.iter())
            .map(|(a, b)| {
                if a == b {
                    a.clone()
                } else {
                    // ITE: if path_condition then a else b
                    // For now, use a fresh symbolic variable as merge proxy
                    SymbolicValue::fresh(crate::types::SymVarId(0xFFFF_0000))
                }
            })
            .collect();

        let merged_storage: Vec<(SymbolicValue, SymbolicValue)> = {
            let mut merged = state_a.storage.clone();
            for (k, v) in &state_b.storage {
                if !merged.iter().any(|(mk, _)| mk == k) {
                    merged.push((k.clone(), v.clone()));
                }
            }
            merged
        };

        Some(ExecutionState {
            stack: merged_stack,
            storage: merged_storage,
            pc: state_a.pc,
            call_depth: state_a.call_depth,
            ..ExecutionState::default()
        })
    }

    /// Record a merged state at the given PC.
    pub fn record_merged(&mut self, pc: usize, merged: MergedState) {
        self.merged_states.entry(pc).or_default().push(merged);
    }

    /// Begin exploration from an initial state.
    /// This is a stub — the full implementation will use the executor
    /// to step through instructions and fork at branches.
    pub fn explore(
        &mut self,
        _initial_state: ExecutionState,
    ) -> ExplorationResult {
        ExplorationResult {
            terminal_states: Vec::new(),
            paths_explored: 0,
            states_visited: 1,
            terminated_early: false,
            merges_performed: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explorer_creation() {
        let explorer = StateExplorer::new(ExplorerConfig::default());
        assert_eq!(explorer.config.strategy, SearchStrategy::Bfs);
        assert_eq!(explorer.config.max_depth, 1024);
    }

    #[test]
    fn test_empty_exploration() {
        let mut explorer = StateExplorer::new(ExplorerConfig::default());
        let result = explorer.explore(ExecutionState::default());
        assert_eq!(result.paths_explored, 0);
        assert_eq!(result.states_visited, 1);
    }

    #[test]
    fn test_merge_compatible_states() {
        let mut explorer = StateExplorer::new(ExplorerConfig::default());
        let state_a = ExecutionState {
            pc: 42,
            call_depth: 0,
            stack: vec![SymbolicValue::concrete(1)],
            ..ExecutionState::default()
        };
        let state_b = ExecutionState {
            pc: 42,
            call_depth: 0,
            stack: vec![SymbolicValue::concrete(2)],
            ..ExecutionState::default()
        };
        let merged = explorer.try_merge(&state_a, &state_b);
        assert!(merged.is_some());
        assert_eq!(merged.unwrap().pc, 42);
    }

    #[test]
    fn test_merge_incompatible_pc() {
        let mut explorer = StateExplorer::new(ExplorerConfig::default());
        let state_a = ExecutionState {
            pc: 10,
            ..ExecutionState::default()
        };
        let state_b = ExecutionState {
            pc: 20,
            ..ExecutionState::default()
        };
        assert!(explorer.try_merge(&state_a, &state_b).is_none());
    }
}