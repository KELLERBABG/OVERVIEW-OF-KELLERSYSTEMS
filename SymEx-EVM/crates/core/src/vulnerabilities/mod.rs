//! Vulnerability Pattern Matching
//!
//! Automated detection of known attack vectors using predefined logical
//! patterns. Includes detectors for reentrancy, integer overflow,
//! tx.origin misuse, unchecked call returns, access control bypass,
//! and more.

use crate::types::{ExecutionState, SymbolicValue};

/// Severity of a detected vulnerability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Critical — immediate risk of fund loss or unauthorized access.
    Critical,
    /// High — likely exploitable under common conditions.
    High,
    /// Medium — exploitable in specific edge cases.
    Medium,
    /// Low — informational, best-practice violation.
    Low,
    /// Informational — no direct security impact but worth noting.
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Critical => write!(f, "CRITICAL"),
            Severity::High => write!(f, "HIGH"),
            Severity::Medium => write!(f, "MEDIUM"),
            Severity::Low => write!(f, "LOW"),
            Severity::Info => write!(f, "INFO"),
        }
    }
}

/// A detected vulnerability instance.
#[derive(Debug, Clone)]
pub struct Vulnerability {
    /// Name of the vulnerability pattern.
    pub name: String,
    /// Severity of the issue.
    pub severity: Severity,
    /// Human-readable description.
    pub description: String,
    /// The function or code location where it was found.
    pub location: String,
    /// SWC (Smart Contract Weakness Classification) identifier.
    pub swc_id: Option<u32>,
    /// Suggested remediation.
    pub remediation: Option<String>,
}

/// Result of running all vulnerability detectors on a contract.
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// All detected vulnerabilities.
    pub vulnerabilities: Vec<Vulnerability>,
    /// Number of detectors that ran.
    pub detectors_run: usize,
    /// Time taken in milliseconds.
    pub scan_time_ms: u64,
}

impl ScanResult {
    /// Create an empty scan result.
    pub fn empty() -> Self {
        Self {
            vulnerabilities: Vec::new(),
            detectors_run: 0,
            scan_time_ms: 0,
        }
    }

    /// Get vulnerabilities by severity.
    pub fn by_severity(&self, severity: Severity) -> Vec<&Vulnerability> {
        self.vulnerabilities
            .iter()
            .filter(|v| v.severity == severity)
            .collect()
    }

    /// Get the highest severity vulnerability.
    pub fn highest_severity(&self) -> Option<Severity> {
        self.vulnerabilities
            .iter()
            .map(|v| v.severity)
            .min_by_key(|s| match s {
                Severity::Critical => 0,
                Severity::High => 1,
                Severity::Medium => 2,
                Severity::Low => 3,
                Severity::Info => 4,
            })
    }
}

/// Base trait for vulnerability detectors.
pub trait VulnerabilityDetector: std::fmt::Debug {
    /// Human-readable name of this detector.
    fn name(&self) -> &'static str;

    /// Run detection on the given execution states or contract IR.
    fn detect(&self, states: &[ExecutionState]) -> Vec<Vulnerability>;
}

// ---------------------------------------------------------------------------
// Concrete detectors
// ---------------------------------------------------------------------------

/// Detects reentrancy vulnerabilities (external calls before state updates).
#[derive(Debug)]
pub struct ReentrancyDetector;

impl VulnerabilityDetector for ReentrancyDetector {
    fn name(&self) -> &'static str {
        "Reentrancy"
    }

    fn detect(&self, _states: &[ExecutionState]) -> Vec<Vulnerability> {
        // Stub: in the full implementation, this will analyze CALL/DELEGATECALL
        // patterns and check if state changes occur after the call.
        Vec::new()
    }
}

/// Detects integer overflow/underflow in arithmetic operations.
#[derive(Debug)]
pub struct IntegerOverflowDetector;

impl VulnerabilityDetector for IntegerOverflowDetector {
    fn name(&self) -> &'static str {
        "Integer Overflow/Underflow"
    }

    fn detect(&self, states: &[ExecutionState]) -> Vec<Vulnerability> {
        // Stub: in the full implementation, this will use the SMT solver
        // to check if ADD/SUB/MUL operations can overflow.
        Vec::new()
    }
}

/// Detects use of tx.origin for authentication.
#[derive(Debug)]
pub struct TxOriginDetector;

impl VulnerabilityDetector for TxOriginDetector {
    fn name(&self) -> &'static str {
        "Tx.Origin Authentication"
    }

    fn detect(&self, states: &[ExecutionState]) -> Vec<Vulnerability> {
        // Stub: check for ORIGIN opcode usage in authentication checks.
        Vec::new()
    }
}

/// Detects unchecked return values from external calls.
#[derive(Debug)]
pub struct UncheckedCallDetector;

impl VulnerabilityDetector for UncheckedCallDetector {
    fn name(&self) -> &'static str {
        "Unchecked Call Return"
    }

    fn detect(&self, states: &[ExecutionState]) -> Vec<Vulnerability> {
        // Stub: check if CALL return values are properly checked.
        Vec::new()
    }
}

/// Detects access control vulnerabilities.
#[derive(Debug)]
pub struct AccessControlDetector;

impl VulnerabilityDetector for AccessControlDetector {
    fn name(&self) -> &'static str {
        "Access Control"
    }

    fn detect(&self, states: &[ExecutionState]) -> Vec<Vulnerability> {
        // Stub: analyze function selectors and authorization checks.
        Vec::new()
    }
}

/// Detects timestamp dependency (miners can influence timestamps).
#[derive(Debug)]
pub struct TimestampDependencyDetector;

impl VulnerabilityDetector for TimestampDependencyDetector {
    fn name(&self) -> &'static str {
        "Timestamp Dependency"
    }

    fn detect(&self, states: &[ExecutionState]) -> Vec<Vulnerability> {
        // Stub: detect TIMESTAMP opcode usage in critical logic.
        Vec::new()
    }
}

/// The vulnerability scanning engine.
#[derive(Debug)]
pub struct VulnerabilityScanner {
    /// All registered detectors.
    pub detectors: Vec<Box<dyn VulnerabilityDetector>>,
}

impl VulnerabilityScanner {
    /// Create a new scanner with all default detectors.
    pub fn new() -> Self {
        let detectors: Vec<Box<dyn VulnerabilityDetector>> = vec![
            Box::new(ReentrancyDetector),
            Box::new(IntegerOverflowDetector),
            Box::new(TxOriginDetector),
            Box::new(UncheckedCallDetector),
            Box::new(AccessControlDetector),
            Box::new(TimestampDependencyDetector),
        ];
        Self { detectors }
    }

    /// Register a custom detector.
    pub fn register_detector(&mut self, detector: Box<dyn VulnerabilityDetector>) {
        self.detectors.push(detector);
    }

    /// Run all detectors on the given execution states.
    pub fn scan(&self, states: &[ExecutionState]) -> ScanResult {
        let mut all_vulns = Vec::new();

        for detector in &self.detectors {
            let vulns = detector.detect(states);
            all_vulns.extend(vulns);
        }

        ScanResult {
            vulnerabilities: all_vulns,
            detectors_run: self.detectors.len(),
            scan_time_ms: 0,
        }
    }
}

impl Default for VulnerabilityScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_has_default_detectors() {
        let scanner = VulnerabilityScanner::new();
        assert_eq!(scanner.detectors.len(), 6);
    }

    #[test]
    fn test_scan_empty_states() {
        let scanner = VulnerabilityScanner::new();
        let result = scanner.scan(&[]);
        assert!(result.vulnerabilities.is_empty());
        assert_eq!(result.detectors_run, 6);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical < Severity::High);
        assert!(Severity::High < Severity::Medium);
        assert!(Severity::Medium < Severity::Low);
        assert!(Severity::Low < Severity::Info);
    }
}