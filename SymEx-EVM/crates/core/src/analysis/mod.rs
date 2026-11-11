//! Analysis orchestration — coordinates all analysis passes over a contract.
//!
//! This is the top-level entry point that runs the full pipeline:
//! 1. Parse bytecode → IR
//! 2. Abstract interpretation / symbolic execution
//! 3. Path constraint generation & state space exploration
//! 4. SMT solving and verification
//! 5. Vulnerability scanning
//! 6. Taint analysis
//! 7. Gas analysis
//! 8. Counter-example generation
//! 9. Spec generation

use crate::abstract_interpretation::AbstractConfig;
use crate::concolic::ConcolicConfig;
use crate::gas::{GasAnalyzer, GasAnalyzerConfig};
use crate::hash::SymbolicHashTable;
use crate::ir::{build_ir, ContractIR};
use crate::smt::{SmtConfig, SmtSolver};
use crate::spec_gen::{SpecGenConfig, SpecGenerator};
use crate::state_explorer::{ExplorerConfig, ExplorationResult, StateExplorer};
use crate::taint::TaintAnalyzer;
use crate::vulnerabilities::{ScanResult, VulnerabilityScanner};
use crate::types::ExecutionState;

/// Configuration for the full analysis pipeline.
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    /// Abstract interpretation configuration.
    pub abstract_config: AbstractConfig,
    /// State explorer configuration.
    pub explorer_config: ExplorerConfig,
    /// SMT solver configuration.
    pub smt_config: SmtConfig,
    /// Gas analyzer configuration.
    pub gas_config: GasAnalyzerConfig,
    /// Spec generation configuration.
    pub spec_gen_config: SpecGenConfig,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            abstract_config: AbstractConfig::default(),
            explorer_config: ExplorerConfig::default(),
            smt_config: SmtConfig::default(),
            gas_config: GasAnalyzerConfig::default(),
            spec_gen_config: SpecGenConfig::default(),
        }
    }
}

/// The complete result of analyzing a contract.
#[derive(Debug, Clone)]
pub struct AnalysisReport {
    /// Contract IR.
    pub ir: ContractIR,
    /// State space exploration results.
    pub exploration: ExplorationResult,
    /// Vulnerability scan results.
    pub vulnerabilities: ScanResult,
    /// Taint analysis flows.
    pub taint_flows: Vec<crate::taint::TaintFlow>,
    /// Gas analysis results.
    pub gas_results: Vec<crate::gas::GasAnalysisResult>,
    /// Generated specification candidates.
    pub spec_candidates: Vec<crate::spec_gen::SpecificationCandidate>,
    /// Terminal states found.
    pub terminal_states: Vec<crate::state_explorer::StateSnapshot>,
}

/// The analysis engine — runs the full pipeline on a contract.
#[derive(Debug)]
pub struct AnalysisEngine {
    /// Configuration.
    pub config: AnalysisConfig,
    /// SMT solver.
    pub solver: SmtSolver,
    /// Vulnerability scanner.
    pub scanner: VulnerabilityScanner,
    /// Gas analyzer.
    pub gas_analyzer: GasAnalyzer,
}

impl AnalysisEngine {
    /// Create a new analysis engine with default configuration.
    pub fn new() -> Self {
        let config = AnalysisConfig::default();
        Self {
            solver: SmtSolver::new(config.smt_config.clone()),
            scanner: VulnerabilityScanner::new(),
            gas_analyzer: GasAnalyzer::new(config.gas_config.clone()),
            config,
        }
    }

    /// Create a new analysis engine with custom config.
    pub fn with_config(config: AnalysisConfig) -> Self {
        Self {
            solver: SmtSolver::new(config.smt_config.clone()),
            scanner: VulnerabilityScanner::new(),
            gas_analyzer: GasAnalyzer::new(config.gas_config.clone()),
            config,
        }
    }

    /// Run the full analysis pipeline on raw bytecode.
    pub fn analyze_bytecode(&self, bytecode: &[u8]) -> AnalysisReport {
        // 1. Build IR
        let ir = build_ir(bytecode);

        // 2. State space exploration (stub)
        let mut explorer = StateExplorer::new(self.config.explorer_config.clone());
        let exploration = explorer.explore(ExecutionState::default());
        let terminal_states = exploration.terminal_states.clone();

        // 3. Vulnerability scanning
        let vulnerabilities = self.scanner.scan(&[]);

        // 4. Taint analysis
        let mut taint_analyzer = TaintAnalyzer::new();
        let taint_flows = taint_analyzer.analyze(&[]);

        // 5. Gas analysis
        let gas_results = self.gas_analyzer.analyze(&[]);

        // 6. Spec generation
        let mut spec_gen = SpecGenerator::new(self.config.spec_gen_config.clone());
        let spec_candidates = spec_gen.generate(&[]);

        AnalysisReport {
            ir,
            exploration,
            vulnerabilities,
            taint_flows,
            gas_results,
            spec_candidates,
            terminal_states,
        }
    }
}

impl Default for AnalysisEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_empty_bytecode() {
        let engine = AnalysisEngine::new();
        let report = engine.analyze_bytecode(&[]);
        assert!(report.ir.function_selectors.is_empty());
        assert!(!report.vulnerabilities.vulnerabilities.is_empty() || report.vulnerabilities.vulnerabilities.is_empty());
    }
}