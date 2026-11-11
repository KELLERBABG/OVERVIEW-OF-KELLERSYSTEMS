//! SMT-LIB 2.0 expression generation.
//!
//! Converts our internal `ConstraintExpr` into SMT-LIB 2.0 format
//! strings that can be fed to Z3 (or other SMT solvers) via stdin.
//!
//! The SMT-LIB format is the standard input language for SMT solvers.
//! This module enables the Z3 backend in the future.

use sca_types::{ComparisonOp, ConstraintExpr, ConstraintVariable};

/// Convert a `ConstraintVariable` to an SMT-LIB identifier.
fn variable_to_smt(var: &ConstraintVariable) -> String {
    match var {
        ConstraintVariable::Capacity => "capacity".to_string(),
        ConstraintVariable::Cost => "cost".to_string(),
        ConstraintVariable::Time => "time".to_string(),
        ConstraintVariable::Carbon => "carbon".to_string(),
        ConstraintVariable::Reliability => "reliability".to_string(),
        ConstraintVariable::FuelEfficiency => "fuel_efficiency".to_string(),
        ConstraintVariable::BorderTime => "border_time".to_string(),
        ConstraintVariable::Temperature => "temperature".to_string(),
        ConstraintVariable::Custom(name) => name.clone(),
    }
}

/// Convert a `ComparisonOp` to an SMT-LIB operator string.
fn op_to_smt(op: &ComparisonOp) -> &'static str {
    match op {
        ComparisonOp::Gt => ">",
        ComparisonOp::Ge => ">=",
        ComparisonOp::Lt => "<",
        ComparisonOp::Le => "<=",
        ComparisonOp::Eq => "=",
        ComparisonOp::Ne => "!=",
    }
}

/// Convert a `ConstraintExpr` to SMT-LIB format.
pub fn expr_to_smt(expr: &ConstraintExpr, indent: usize) -> String {
    let pad = " ".repeat(indent);
    match expr {
        ConstraintExpr::Literal(true) => format!("{pad}true"),
        ConstraintExpr::Literal(false) => format!("{pad}false"),
        ConstraintExpr::Comparison { variable, op, value } => {
            let var_name = variable_to_smt(variable);
            let op_str = op_to_smt(op);
            format!("{pad}({op_str} {var_name} {value})")
        }
        ConstraintExpr::And(a, b) => {
            format!("{pad}(and\n{}\n{}\n{pad})", expr_to_smt(a, indent + 2), expr_to_smt(b, indent + 2),)
        }
        ConstraintExpr::Or(a, b) => {
            format!("{pad}(or\n{}\n{}\n{pad})", expr_to_smt(a, indent + 2), expr_to_smt(b, indent + 2),)
        }
        ConstraintExpr::Not(a) => {
            format!("{pad}(not\n{}\n{pad})", expr_to_smt(a, indent + 2))
        }
    }
}

/// Generate a complete SMT-LIB 2.0 query from a set of constraints.
/// The query checks if all constraints are simultaneously satisfiable.
pub fn generate_smt_query(constraints: &[ConstraintExpr]) -> String {
    let mut sb = String::new();

    // Header
    sb.push_str("(set-logic QF_LRA)\n");
    sb.push_str("(set-info :source \"Algorithmic Supply Chain Arbitrage\")\n\n");

    // Declare variables
    let mut declared = std::collections::HashSet::new();
    for expr in constraints {
        collect_and_declare(expr, &mut declared, &mut sb);
    }

    sb.push('\n');

    // Assert constraints
    for expr in constraints {
        sb.push_str("(assert\n");
        sb.push_str(&expr_to_smt(expr, 2));
        sb.push_str(")\n\n");
    }

    // Check
    sb.push_str("(check-sat)\n");
    sb.push_str("(get-model)\n");

    sb
}

fn collect_and_declare(expr: &ConstraintExpr, declared: &mut std::collections::HashSet<String>, sb: &mut String) {
    match expr {
        ConstraintExpr::Literal(_) => {}
        ConstraintExpr::Comparison { variable, .. } => {
            let name = variable_to_smt(variable);
            if declared.insert(name.clone()) {
                sb.push_str(&format!("(declare-fun {name} () Real)\n"));
            }
        }
        ConstraintExpr::And(a, b) | ConstraintExpr::Or(a, b) => {
            collect_and_declare(a, declared, sb);
            collect_and_declare(b, declared, sb);
        }
        ConstraintExpr::Not(a) => collect_and_declare(a, declared, sb),
    }
}

/// Generate an SMT-LIB optimization query for minimize/maximize.
pub fn generate_optimize_query(
    constraints: &[ConstraintExpr],
    variable: &ConstraintVariable,
    maximize: bool,
) -> String {
    let objective = if maximize { "maximize" } else { "minimize" };
    let opt = format!("({objective} {})", variable_to_smt(variable));

    // Rebuild with optimization
    let mut sb = String::new();
    sb.push_str("(set-logic QF_LRA)\n");
    sb.push_str("(set-option :opt-strategy pareto)\n");
    sb.push_str("(set-info :source \"Algorithmic Supply Chain Arbitrage\")\n\n");

    let mut declared = std::collections::HashSet::new();
    for expr in constraints {
        collect_and_declare(expr, &mut declared, &mut sb);
    }

    sb.push('\n');
    for expr in constraints {
        sb.push_str("(assert\n");
        sb.push_str(&expr_to_smt(expr, 2));
        sb.push_str(")\n\n");
    }

    sb.push_str("(minimize transit_time)\n");
    sb.push_str(&opt);
    sb.push_str("\n(check-sat)\n");
    sb.push_str("(get-objectives)\n");

    sb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_true() {
        let result = expr_to_smt(&ConstraintExpr::always_true(), 0);
        assert_eq!(result, "true");
    }

    #[test]
    fn test_literal_false() {
        let result = expr_to_smt(&ConstraintExpr::always_false(), 0);
        assert_eq!(result, "false");
    }

    #[test]
    fn test_comparison() {
        let expr = ConstraintExpr::capacity(ComparisonOp::Ge, 100.0);
        let result = expr_to_smt(&expr, 0);
        assert_eq!(result, "(>= capacity 100)");
    }

    #[test]
    fn test_generate_query() {
        let constraints = vec![
            ConstraintExpr::capacity(ComparisonOp::Ge, 100.0),
            ConstraintExpr::Comparison { variable: ConstraintVariable::Temperature, op: ComparisonOp::Le, value: 8.0 },
        ];
        let query = generate_smt_query(&constraints);
        assert!(query.contains("declare-fun capacity () Real"));
        assert!(query.contains("declare-fun temperature () Real"));
        assert!(query.contains("(>= capacity 100)"));
        assert!(query.contains("(<= temperature 8)"));
        assert!(query.contains("(check-sat)"));
        assert!(query.contains("(get-model)"));
    }

    #[test]
    fn test_nested_expression() {
        let expr = ConstraintExpr::And(
            Box::new(ConstraintExpr::capacity(ComparisonOp::Ge, 100.0)),
            Box::new(ConstraintExpr::always_true()),
        );
        let result = expr_to_smt(&expr, 0);
        assert!(result.contains("(and"));
        assert!(result.contains("(>= capacity 100)"));
        assert!(result.contains("true"));
    }
}
