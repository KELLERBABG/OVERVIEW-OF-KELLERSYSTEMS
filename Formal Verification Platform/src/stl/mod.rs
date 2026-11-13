//! STL Container Invariant Verification Module.
//!
//! Provides invariant checking for std::vector and std::string,
//! including iterator invalidation detection and lifetime analysis.

pub mod invariants;
pub mod iterator_invalidation;
pub mod pure_virtual;

use crate::frontend::cpp::{StlContainerInstance, StlInvariantViolation, ViolationSeverity};

/// Run all STL invariant checks on a set of container instances.
///
/// In audit-only mode, violations are collected but never used to
/// mutate or heal the source code.
pub fn run_stl_invariant_checks(
    containers: &[StlContainerInstance],
    audit_only: bool,
) -> StlCheckReport {
    let mut report = StlCheckReport::new();

    for container in containers {
        match container.kind.as_str() {
            "std::vector" => {
                let violations = invariants::check_vector_invariants(container);
                report.add_violations(violations);
            }
            "std::string" => {
                let violations = invariants::check_string_invariants(container);
                report.add_violations(violations);
            }
            _ => {}
        }

        // Iterator invalidation is checked separately for vectors.
        if container.kind == "std::vector" {
            let inv_violations = iterator_invalidation::check_vector_iterator_invalidation(container);
            report.add_violations(inv_violations);
        }
    }

    report.audit_only = audit_only;
    report.finalize();
    report
}

/// A report of all STL invariant check results.
#[derive(Debug, Clone)]
pub struct StlCheckReport {
    /// All detected violations.
    pub violations: Vec<StlInvariantViolation>,
    /// Whether this was an audit-only run (no healing applied).
    pub audit_only: bool,
    /// Total number of containers checked.
    pub containers_checked: usize,
    /// Timestamp of the check (ISO 8601).
    pub timestamp: String,
}

impl StlCheckReport {
    pub fn new() -> Self {
        Self {
            violations: Vec::new(),
            audit_only: true,
            containers_checked: 0,
            timestamp: chrono_now_iso8601(),
        }
    }

    pub fn add_violations(&mut self, violations: Vec<StlInvariantViolation>) {
        self.containers_checked += 1; // Approximate: one container per check batch
        self.violations.extend(violations);
    }

    pub fn finalize(&mut self) {
        // Sort violations by severity (Error first) then by location.
        self.violations.sort_by(|a, b| {
            let sev_order = |v: &StlInvariantViolation| match v.severity {
                ViolationSeverity::Error => 0,
                ViolationSeverity::Warning => 1,
                ViolationSeverity::Info => 2,
            };
            sev_order(a).cmp(&sev_order(b))
        });
    }

    /// Render the report as a human-readable string.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("=== STL Container Invariant Report ===\n");
        out.push_str(&format!("Mode: {}\n", if self.audit_only { "AUDIT-ONLY" } else { "HEALING" }));
        out.push_str(&format!("Containers checked: {}\n", self.containers_checked));
        out.push_str(&format!("Violations found: {}\n\n", self.violations.len()));

        if self.violations.is_empty() {
            out.push_str("[PASS] All STL invariants verified.\n");
        } else {
            for (i, v) in self.violations.iter().enumerate() {
                let sev_tag = match v.severity {
                    ViolationSeverity::Error => "[ERROR]",
                    ViolationSeverity::Warning => "[WARN ]",
                    ViolationSeverity::Info => "[INFO ]",
                };
                out.push_str(&format!(
                    "  {}. {} {} ({}:{})\n     {}\n     {}\n",
                    i + 1,
                    sev_tag,
                    v.invariant_name,
                    v.location.file,
                    v.location.line,
                    v.description,
                    v.container.var_name.as_deref().unwrap_or("<unnamed>"),
                ));
            }
        }

        out
    }
}

/// Get current time as ISO 8601 (simplified; in production use a proper chrono dep).
fn chrono_now_iso8601() -> String {
    // Placeholder for actual timestamp; in production this would use
    // std::time::SystemTime or the chrono crate.
    "1970-01-01T00:00:00Z".to_string()
}
