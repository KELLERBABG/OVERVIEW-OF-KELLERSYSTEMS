//! C# Roslyn CIL Engine — Ingests C# syntax trees and CIL bytecode into the
//! Keller-VIR formal verification foundation.
//!
//! Provides:
//! - Nullable Reference Type (NRT) checking
//! - `Span<T>` bounds-safety verification
//! - `IndexOutOfRangeException` detection
//! - xUnit test-fixture synthesis
//! - Audit-only mode (report generation without source mutation)

pub mod ast_ingest;
pub mod cil_analyzer;
pub mod nullable_check;
pub mod span_bounds;
pub mod index_oob;
pub mod test_fixture_gen;
pub mod audit_report;

use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Top-level configuration for the C# Roslyn CIL Engine.
#[derive(Debug, Clone)]
pub struct CSharpEngineConfig {
    /// Path to the C# project or solution under analysis.
    pub target_path: PathBuf,
    /// Enable Nullable Reference Type checking.
    pub check_nullable: bool,
    /// Enable Span<T> bounds-safety verification.
    pub check_span_bounds: bool,
    /// Enable IndexOutOfRangeException detection.
    pub check_index_oob: bool,
    /// Synthesize xUnit test fixtures for discovered violations.
    pub synthesize_fixtures: bool,
    /// Audit-only mode: save report, never mutate source.
    pub audit_only: bool,
    /// Output directory for reports and generated fixtures.
    pub output_dir: PathBuf,
}

impl Default for CSharpEngineConfig {
    fn default() -> Self {
        Self {
            target_path: PathBuf::from("."),
            check_nullable: true,
            check_span_bounds: true,
            check_index_oob: true,
            synthesize_fixtures: true,
            audit_only: true,
            output_dir: PathBuf::from("./verification_output"),
        }
    }
}

/// The result of a full C# verification pass.
#[derive(Debug)]
pub struct VerificationResult {
    /// All nullable reference type violations found.
    pub nullable_violations: Vec<nullable_check::NullableViolation>,
    /// All Span<T> bounds-safety violations found.
    pub span_violations: Vec<span_bounds::SpanBoundsViolation>,
    /// All IndexOutOfRangeException risks found.
    pub index_oob_violations: Vec<index_oob::IndexOOBViolation>,
    /// Generated xUnit test fixtures (as source strings).
    pub generated_fixtures: Vec<test_fixture_gen::TestFixture>,
    /// The audit report (always produced in audit-only mode).
    pub audit_report: audit_report::AuditReport,
}

/// Entry point: run the full C# Roslyn CIL verification pipeline.
pub fn run_verification(config: &CSharpEngineConfig) -> Result<VerificationResult, EngineError> {
    // Phase 1: Ingest C# source / CIL
    let ast = ast_ingest::ingest_project(&config.target_path)?;

    // Phase 2: Analyze CIL bytecode for control-flow and data-flow facts
    let cil_facts = cil_analyzer::analyze_cil(&ast)?;

    // Phase 3: Run verification passes
    let mut nullable_violations = Vec::new();
    let mut span_violations = Vec::new();
    let mut index_oob_violations = Vec::new();

    if config.check_nullable {
        nullable_violations = nullable_check::check_nullable(&ast, &cil_facts);
    }

    if config.check_span_bounds {
        span_violations = span_bounds::check_span_bounds(&ast, &cil_facts);
    }

    if config.check_index_oob {
        index_oob_violations = index_oob::check_index_oob(&ast, &cil_facts);
    }

    // Phase 4: Synthesize xUnit test fixtures
    let mut generated_fixtures = Vec::new();
    if config.synthesize_fixtures {
        generated_fixtures = test_fixture_gen::synthesize_fixtures(
            &nullable_violations,
            &span_violations,
            &index_oob_violations,
        );
    }

    // Phase 5: Generate audit report (always in audit-only mode)
    let audit_report = audit_report::generate_report(
        config,
        &nullable_violations,
        &span_violations,
        &index_oob_violations,
        &generated_fixtures,
    );

    // Audit-only: write report to disk, never mutate source.
    if config.audit_only {
        audit_report::write_report(&config.output_dir, &audit_report)?;
    }

    Ok(VerificationResult {
        nullable_violations,
        span_violations,
        index_oob_violations,
        generated_fixtures,
        audit_report,
    })
}

/// Errors that can occur during C# engine execution.
#[derive(Debug)]
pub enum EngineError {
    /// File I/O error.
    Io(std::io::Error),
    /// Parsing error for C# source or CIL.
    Parse(String),
    /// Analysis error (e.g., malformed CIL).
    Analysis(String),
    /// Configuration error.
    Config(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Io(e) => write!(f, "I/O error: {e}"),
            EngineError::Parse(msg) => write!(f, "Parse error: {msg}"),
            EngineError::Analysis(msg) => write!(f, "Analysis error: {msg}"),
            EngineError::Config(msg) => write!(f, "Configuration error: {msg}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<std::io::Error> for EngineError {
    fn from(e: std::io::Error) -> Self {
        EngineError::Io(e)
    }
}
