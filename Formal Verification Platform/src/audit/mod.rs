//! Audit-only verification mode.
//!
//! In audit mode, the verifier analyzes code and produces reports
//! but NEVER mutates or heals the source. This is the default mode
//! for benchmark repository verification.

pub mod report;

use std::path::{Path, PathBuf};
use std::fs;

/// Configuration for an audit run.
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Path to the benchmark repository or source directory.
    pub target_path: PathBuf,
    /// Output path for the audit report (JSON or text).
    pub report_output: PathBuf,
    /// Whether to include STL invariant checks.
    pub include_stl_checks: bool,
    /// Whether to include pure virtual call checks.
    pub include_pure_virtual_checks: bool,
    /// Maximum number of files to analyze (0 = unlimited).
    pub max_files: usize,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            target_path: PathBuf::from("."),
            report_output: PathBuf::from("audit_report.txt"),
            include_stl_checks: true,
            include_pure_virtual_checks: true,
            max_files: 0,
        }
    }
}

/// The result of an audit run.
#[derive(Debug, Clone)]
pub struct AuditResult {
    /// Whether the audit passed (no errors found).
    pub passed: bool,
    /// Total files analyzed.
    pub files_analyzed: usize,
    /// Total violations found.
    pub total_violations: usize,
    /// Errors (hard failures).
    pub errors: usize,
    /// Warnings.
    pub warnings: usize,
    /// Informational notes.
    pub infos: usize,
    /// The rendered report text.
    pub report_text: String,
}

/// Run an audit-only verification pass on the target path.
///
/// # Guarantees
/// - Source files are NEVER modified.
/// - No healing or auto-fix is applied.
/// - Only a report file is written (to `config.report_output`).
pub fn run_audit(config: &AuditConfig) -> Result<AuditResult, AuditError> {
    // Verify target exists.
    if !config.target_path.exists() {
        return Err(AuditError::TargetNotFound(config.target_path.clone()));
    }

    // Collect C++ source files.
    let source_files = collect_cpp_sources(&config.target_path, config.max_files);

    if source_files.is_empty() {
        return Err(AuditError::NoSourceFiles(config.target_path.clone()));
    }

    // Analyze each file (read-only).
    let mut all_violations: Vec<report::ViolationEntry> = Vec::new();
    let mut files_analyzed = 0;

    for file in &source_files {
        let content = fs::read_to_string(file)
            .map_err(|e| AuditError::ReadError {
                path: file.clone(),
                source: e.to_string(),
            })?;

        // Parse and analyze (simplified: pattern-based for audit mode).
        let violations = report::analyze_file_content(
            file,
            &content,
            config.include_stl_checks,
            config.include_pure_virtual_checks,
        );
        all_violations.extend(violations);
        files_analyzed += 1;
    }

    // Tally violations.
    let errors = all_violations.iter().filter(|v| v.severity == report::Severity::Error).count();
    let warnings = all_violations.iter().filter(|v| v.severity == report::Severity::Warning).count();
    let infos = all_violations.iter().filter(|v| v.severity == report::Severity::Info).count();

    // Generate report.
    let report_text = report::render_report(
        &config.target_path,
        files_analyzed,
        &all_violations,
    );

    // Write report (this is the ONLY file write in audit mode).
    if let Some(parent) = config.report_output.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AuditError::WriteError {
                path: parent.to_path_buf(),
                source: e.to_string(),
            })?;
    }
    fs::write(&config.report_output, &report_text)
        .map_err(|e| AuditError::WriteError {
            path: config.report_output.clone(),
            source: e.to_string(),
        })?;

    Ok(AuditResult {
        passed: errors == 0,
        files_analyzed,
        total_violations: all_violations.len(),
        errors,
        warnings,
        infos,
        report_text,
    })
}

/// Collect C++ source files from a directory (non-recursive for simplicity;
/// in production this would be recursive with proper filtering).
fn collect_cpp_sources(root: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut sources = Vec::new();

    if root.is_file() {
        if is_cpp_file(root) {
            sources.push(root.to_path_buf());
        }
        return sources;
    }

    // Walk directory (simplified: one level).
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_cpp_file(&path) {
                sources.push(path);
                if max_files > 0 && sources.len() >= max_files {
                    break;
                }
            }
        }
    }

    sources
}

/// Check if a file has a C++ source extension.
fn is_cpp_file(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => matches!(ext, "cpp" | "cc" | "cxx" | "c++" | "h" | "hpp" | "hxx"),
        None => false,
    }
}

/// Errors from the audit process.
#[derive(Debug)]
pub enum AuditError {
    /// The target path does not exist.
    TargetNotFound(PathBuf),
    /// No C++ source files found in the target.
    NoSourceFiles(PathBuf),
    /// Failed to read a source file.
    ReadError { path: PathBuf, source: String },
    /// Failed to write the report.
    WriteError { path: PathBuf, source: String },
}

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TargetNotFound(p) => write!(f, "Target path not found: {}", p.display()),
            Self::NoSourceFiles(p) => write!(f, "No C++ source files found in: {}", p.display()),
            Self::ReadError { path, source } => {
                write!(f, "Failed to read {}: {}", path.display(), source)
            }
            Self::WriteError { path, source } => {
                write!(f, "Failed to write report to {}: {}", path.display(), source)
            }
        }
    }
}
