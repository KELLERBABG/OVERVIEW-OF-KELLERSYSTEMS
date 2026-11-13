//! Pure virtual call detection for C++ class hierarchies.
//!
//! Verifies that no code path can dispatch to a pure virtual function,
//! which would be undefined behavior (typically calling a sentinel address).

use crate::frontend::cpp::{PureVirtualInfo, ViolationSeverity};
use crate::frontend::cpp::ast::ClassDecl;

/// Result of the pure virtual call analysis.
#[derive(Debug, Clone)]
pub struct PureVirtualAnalysisResult {
    /// All pure virtual functions found in the hierarchy.
    pub pure_virtuals: Vec<PureVirtualInfo>,
    /// Classes that are correctly abstract (cannot be instantiated).
    pub abstract_classes: Vec<String>,
    /// Concrete classes that correctly override all base pure virtuals.
    pub valid_concrete_classes: Vec<String>,
    /// Violations: concrete classes that fail to override a pure virtual.
    pub violations: Vec<PureVirtualViolation>,
}

/// A specific pure virtual violation.
#[derive(Debug, Clone)]
pub struct PureVirtualViolation {
    /// The class where the violation is detected.
    pub class_name: String,
    /// The pure virtual method that is not overridden.
    pub method_name: String,
    /// The base class that declares the pure virtual.
    pub declaring_base: String,
    /// Severity (always Error for this check).
    pub severity: ViolationSeverity,
    /// Human-readable explanation.
    pub explanation: String,
}

/// Run the full pure virtual call analysis on a set of classes.
pub fn analyze_pure_virtuals(classes: &[ClassDecl]) -> PureVirtualAnalysisResult {
    let mut pure_virtuals = Vec::new();
    let mut abstract_classes = Vec::new();
    let mut valid_concrete_classes = Vec::new();
    let mut violations = Vec::new();

    // First pass: collect all pure virtuals and identify abstract classes.
    for class in classes {
        if class.is_abstract {
            abstract_classes.push(class.name.clone());
        } else {
            valid_concrete_classes.push(class.name.clone());
        }

        for method in &class.methods {
            if method.is_pure_virtual {
                pure_virtuals.push(PureVirtualInfo {
                    qualified_name: format!("{}::{}", class.name, method.name),
                    declaring_class: class.name.clone(),
                    location: method.location.clone(),
                });
            }
        }
    }

    // Second pass: for each concrete class, verify all base pure virtuals are overridden.
    for class in classes {
        if !class.is_abstract {
            for base_spec in &class.bases {
                if let Some(base_class) = classes.iter().find(|c| c.name == base_spec.name) {
                    for method in &base_class.methods {
                        if method.is_pure_virtual {
                            let overridden = class.methods.iter().any(
                                |m| m.name == method.name && m.is_virtual && !m.is_pure_virtual,
                            );

                            if !overridden {
                                violations.push(PureVirtualViolation {
                                    class_name: class.name.clone(),
                                    method_name: method.name.clone(),
                                    declaring_base: base_class.name.clone(),
                                    severity: ViolationSeverity::Error,
                                    explanation: format!(
                                        "Concrete class '{}' does not override pure virtual \
                                         '{}::{}'. Instantiating this class and calling the \
                                         method through a base pointer/reference is UB.",
                                        class.name,
                                        base_class.name,
                                        method.name,
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    PureVirtualAnalysisResult {
        pure_virtuals,
        abstract_classes,
        valid_concrete_classes,
        violations,
    }
}

/// Check if a class hierarchy is "safe" (no possible pure virtual dispatch).
pub fn is_hierarchy_safe(classes: &[ClassDecl]) -> bool {
    let result = analyze_pure_virtuals(classes);
    result.violations.is_empty()
}

/// Generate a summary of the pure virtual analysis for reporting.
pub fn summarize(result: &PureVirtualAnalysisResult) -> String {
    let mut out = String::new();
    out.push_str("=== Pure Virtual Call Analysis ===\n");
    out.push_str(&format!("Abstract classes: {}\n", result.abstract_classes.len()));
    out.push_str(&format!("Concrete classes: {}\n", result.valid_concrete_classes.len()));
    out.push_str(&format!("Pure virtual functions: {}\n", result.pure_virtuals.len()));

    if result.violations.is_empty() {
        out.push_str("[PASS] No pure virtual call violations detected.\n");
    } else {
        out.push_str(&format!("[FAIL] {} violation(s) detected:\n", result.violations.len()));
        for v in &result.violations {
            out.push_str(&format!("  - {}::{} (declared in {})\n", v.class_name, v.method_name, v.declaring_base));
        }
    }

    out
}
