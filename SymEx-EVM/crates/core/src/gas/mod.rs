//! Gas Consumption Analysis
//!
//! Symbolic computation of worst-case gas usage per function. Detects
//! unbounded loops, excessive storage operations, and potential
//! out-of-gas vulnerabilities before deployment.

use crate::types::{ExecutionState, SymbolicValue};

/// Result of analyzing gas consumption for a function.
#[derive(Debug, Clone)]
pub struct GasAnalysisResult {
    /// Name of the function analyzed.
    pub function_name: String,
    /// Minimum gas cost (best-case path).
    pub min_gas: u64,
    /// Maximum gas cost (worst-case path, bounded).
    pub max_gas: u64,
    /// Whether the function has potentially unbounded gas usage.
    pub has_unbounded_loop: bool,
    /// Paths with the highest gas consumption.
    pub worst_case_paths: Vec<GasPath>,
    /// Gas distribution across explored paths.
    pub path_gas_distribution: Vec<(u64, usize)>, // (gas_cost, count)
}

/// A single execution path's gas consumption.
#[derive(Debug, Clone)]
pub struct GasPath {
    /// Total gas consumed on this path.
    pub total_gas: u64,
    /// Breakdown of gas by category.
    pub breakdown: GasBreakdown,
    /// Number of instructions executed.
    pub instruction_count: usize,
}

/// Breakdown of gas costs by category.
#[derive(Debug, Clone, Default)]
pub struct GasBreakdown {
    /// Gas spent on arithmetic operations.
    pub arithmetic: u64,
    /// Gas spent on storage operations (SLOAD/SSTORE).
    pub storage: u64,
    /// Gas spent on memory operations.
    pub memory: u64,
    /// Gas spent on external calls.
    pub calls: u64,
    /// Gas spent on logging.
    pub logs: u64,
    /// Gas spent on other operations.
    pub other: u64,
}

/// Configuration for gas analysis.
#[derive(Debug, Clone)]
pub struct GasAnalyzerConfig {
    /// Whether to perform detailed per-opcode breakdown.
    pub detailed_breakdown: bool,
    /// Whether to detect unbounded loops via symbolic analysis.
    pub detect_unbounded_loops: bool,
    /// Block gas limit to compare against.
    pub block_gas_limit: u64,
    /// Whether to include intrinsic gas (21,000 + calldata costs).
    pub include_intrinsic_gas: bool,
}

impl Default for GasAnalyzerConfig {
    fn default() -> Self {
        Self {
            detailed_breakdown: true,
            detect_unbounded_loops: true,
            block_gas_limit: 30_000_000, // ~30M Ethereum block gas limit
            include_intrinsic_gas: true,
        }
    }
}

/// The gas consumption analyzer.
#[derive(Debug, Clone)]
pub struct GasAnalyzer {
    /// Analyzer configuration.
    pub config: GasAnalyzerConfig,
}

impl GasAnalyzer {
    /// Create a new gas analyzer.
    pub fn new(config: GasAnalyzerConfig) -> Self {
        Self { config }
    }

    /// Analyze gas consumption for all functions in a contract.
    ///
    /// In the full implementation, this will symbolically execute each
    /// function and compute the gas cost for every path.
    pub fn analyze(
        &self,
        states: &[ExecutionState],
    ) -> Vec<GasAnalysisResult> {
        let mut results = Vec::new();

        // Stub: create a placeholder result for each analysis
        for (i, _state) in states.iter().enumerate() {
            results.push(GasAnalysisResult {
                function_name: format!("function_{}", i),
                min_gas: 0,
                max_gas: 0,
                has_unbounded_loop: false,
                worst_case_paths: Vec::new(),
                path_gas_distribution: Vec::new(),
            });
        }

        results
    }

    /// Check if any function exceeds the block gas limit.
    pub fn check_block_gas_limit<'a>(&self, results: &'a [GasAnalysisResult]) -> Vec<&'a GasAnalysisResult> {
        results
            .iter()
            .filter(|r| r.max_gas > self.config.block_gas_limit)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let analyzer = GasAnalyzer::new(GasAnalyzerConfig::default());
        assert_eq!(analyzer.config.block_gas_limit, 30_000_000);
    }

    #[test]
    fn test_analyze_empty() {
        let analyzer = GasAnalyzer::new(GasAnalyzerConfig::default());
        let results = analyzer.analyze(&[]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_block_gas_limit_check() {
        let analyzer = GasAnalyzer::new(GasAnalyzerConfig::default());
        let results = vec![GasAnalysisResult {
            function_name: "risky".to_string(),
            min_gas: 100,
            max_gas: 35_000_000,
            has_unbounded_loop: false,
            worst_case_paths: Vec::new(),
            path_gas_distribution: Vec::new(),
        }];
        let over_limit = analyzer.check_block_gas_limit(&results);
        assert_eq!(over_limit.len(), 1);
    }
}