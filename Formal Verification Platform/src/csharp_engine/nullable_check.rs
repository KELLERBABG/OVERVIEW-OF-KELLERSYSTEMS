//! Nullable Reference Type (NRT) Verification Pass.
//!
//! Detects potential null-reference violations in C# code that uses
//! the `#nullable` context. Checks include:
//! - Dereferencing a nullable reference without null-check
//! - Assigning `null` to a non-nullable reference
//! - Returning `null` from a method with non-nullable return type
//! - Passing a nullable value where non-nullable is expected

use std::collections::HashMap;

use crate::csharp_engine::ast_ingest::{
    CSharpProject, Expression, MemberDecl, MethodBody, NullabilityAnnotation, Statement,
};
use crate::csharp_engine::cil_analyzer::CilFacts;

/// A single nullable reference type violation.
#[derive(Debug)]
pub struct NullableViolation {
    /// Fully qualified method name where the violation occurs.
    pub method_fqn: String,
    /// File path.
    pub file_path: String,
    /// Line number (approximate).
    pub line: usize,
    /// The variable or expression involved.
    pub variable: String,
    /// Severity level.
    pub severity: Severity,
    /// Human-readable description.
    pub message: String,
    /// Suggested fix (for non-audit modes).
    pub suggested_fix: Option<String>,
}

/// Severity of a violation.
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    /// Will definitely cause a NullReferenceException at runtime.
    Definite,
    /// May cause a NullReferenceException depending on input.
    Possible,
    /// Style issue — nullable annotation is inconsistent.
    Warning,
}

/// Run the nullable reference type checking pass.
pub fn check_nullable(project: &CSharpProject, _cil_facts: &CilFacts) -> Vec<NullableViolation> {
    let mut violations = Vec::new();

    for unit in &project.compilation_units {
        for ns in &unit.namespaces {
            for ty in &ns.types {
                for member in &ty.members {
                    if let Some(body) = &member.body {
                        let method_fqn = format!("{}.{}::{}", ns.name, ty.name, member.name);
                        check_method_nullable(body, &method_fqn, &unit.file_path.to_string_lossy(), &mut violations);
                    }
                }
            }
        }
    }

    violations
}

/// Check a single method body for nullable violations.
fn check_method_nullable(
    body: &MethodBody,
    method_fqn: &str,
    file_path: &str,
    violations: &mut Vec<NullableViolation>,
) {
    // Track nullability state of each variable
    let mut var_null_state: HashMap<String, bool> = HashMap::new();

    // Initialize from parameters
    for param in &body.parameters {
        if param.nullability == NullabilityAnnotation::Nullable {
            var_null_state.insert(param.name.clone(), true);
        } else if param.nullability == NullabilityAnnotation::NonNull {
            var_null_state.insert(param.name.clone(), false);
        }
    }

    // Initialize from locals
    for local in &body.locals {
        if local.nullability == NullabilityAnnotation::Nullable && !local.has_initializer {
            var_null_state.insert(local.name.clone(), true);
        } else if local.nullability == NullabilityAnnotation::NonNull {
            var_null_state.insert(local.name.clone(), false);
        }
    }

    // Walk statements
    let mut line = 0;
    for stmt in &body.statements {
        match stmt {
            Statement::Assignment { target, expr } => {
                // Check: assigning null to non-nullable
                if matches!(expr, Expression::Null) {
                    if let Some(is_null) = var_null_state.get(target) {
                        if !is_null {
                            violations.push(NullableViolation {
                                method_fqn: method_fqn.to_string(),
                                file_path: file_path.to_string(),
                                line,
                                variable: target.clone(),
                                severity: Severity::Definite,
                                message: format!(
                                    "Assignment of `null` to non-nullable reference `{target}`"
                                ),
                                suggested_fix: Some(format!(
                                    "Change type of `{target}` to nullable or add null-check"
                                )),
                            });
                        }
                    }
                }
                // Update null state
                match expr {
                    Expression::Null => {
                        var_null_state.insert(target.clone(), true);
                    }
                    Expression::NewObject { .. } | Expression::Default(_) => {
                        var_null_state.insert(target.clone(), false);
                    }
                    _ => {}
                }
            }

            Statement::Invocation { receiver, method_name, args } => {
                // Check: calling method on potentially-null receiver
                if let Some(recv) = receiver {
                    if let Some(is_null) = var_null_state.get(recv) {
                        if *is_null {
                            violations.push(NullableViolation {
                                method_fqn: method_fqn.to_string(),
                                file_path: file_path.to_string(),
                                line,
                                variable: recv.clone(),
                                severity: Severity::Definite,
                                message: format!(
                                    "Dereference of potentially-null reference `{recv}` in call to `{method_name}`"
                                ),
                                suggested_fix: Some(format!(
                                    "Add null-check: `if ({recv} is not null) {{ {recv}.{method_name}(...); }}`"
                                )),
                            });
                        }
                    }
                }

                // Check: passing null where non-nullable expected
                for (i, arg) in args.iter().enumerate() {
                    if matches!(arg, Expression::Null) {
                        violations.push(NullableViolation {
                            method_fqn: method_fqn.to_string(),
                            file_path: file_path.to_string(),
                            line,
                            variable: format!("arg[{i}]"),
                            severity: Severity::Possible,
                            message: format!(
                                "Passing `null` as argument {i} to `{method_name}` — parameter may be non-nullable"
                            ),
                            suggested_fix: None,
                        });
                    }
                }
            }

            Statement::Return(Some(expr)) => {
                // Check: returning null from non-nullable method
                if matches!(expr.as_ref(), Expression::Null) {
                    if !body.return_type.is_reference || body.return_type.type_name != "object" {
                        violations.push(NullableViolation {
                            method_fqn: method_fqn.to_string(),
                            file_path: file_path.to_string(),
                            line,
                            variable: "return".to_string(),
                            severity: Severity::Definite,
                            message: format!(
                                "Returning `null` from method with non-nullable return type `{}`",
                                body.return_type.type_name
                            ),
                            suggested_fix: Some(format!(
                                "Change return type to nullable or return a valid instance"
                            )),
                        });
                    }
                }
            }

            Statement::If { condition, then_branch, .. } => {
                // Null-check pattern: `if (x is not null)` or `if (x != null)`
                // narrows the type in the then-branch
                let narrowed_var = extract_null_check_var(condition);
                if let Some(var) = narrowed_var {
                    // In then-branch, var is non-null
                    for s in then_branch {
                        apply_null_narrowing(s, &var, false, &mut var_null_state);
                    }
                    // In else-branch (if any), var is null
                    // (simplified: we don't track else here)
                }
            }

            _ => {}
        }
        line += 1;
    }
}

/// Try to extract the variable being null-checked from a condition expression.
fn extract_null_check_var(cond: &Expression) -> Option<String> {
    match cond {
        Expression::BinaryOp { left, right, .. } => {
            // `x != null` or `x == null`
            if matches!(right.as_ref(), Expression::Null) || matches!(left.as_ref(), Expression::Null) {
                let var_expr = if matches!(right.as_ref(), Expression::Null) {
                    left.as_ref()
                } else {
                    left.as_ref()
                };
                match var_expr {
                    Expression::Variable(name) => Some(name.clone()),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Apply null-narrowing to statements (simplified).
fn apply_null_narrowing(
    stmt: &Statement,
    var: &str,
    is_null: bool,
    state: &mut HashMap<String, bool>,
) {
    if let Statement::Invocation { receiver, .. } = stmt {
        if let Some(recv) = receiver {
            if recv == var {
                state.insert(var.to_string(), is_null);
            }
        }
    }
}
