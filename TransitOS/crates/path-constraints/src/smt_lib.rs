use crate::constraint::Constraint;

pub fn constraint_to_smt2(c: &Constraint) -> String {
    match c {
        Constraint::Eq(var, val) => format!("(= {} {})", var, val),
        Constraint::Lt(var, val) => format!("(< {} {})", var, val),
        Constraint::Le(var, val) => format!("(<= {} {})", var, val),
        Constraint::Gt(var, val) => format!("(> {} {})", var, val),
        Constraint::Ge(var, val) => format!("(>= {} {})", var, val),
        Constraint::Within(var, lo, hi) => {
            format!("(and (<= {} {}) (<= {} {}))", lo, var, var, hi)
        }
        Constraint::And(l, r) => {
            format!("(and {} {})", constraint_to_smt2(l), constraint_to_smt2(r))
        }
        Constraint::Or(l, r) => {
            format!("(or {} {})", constraint_to_smt2(l), constraint_to_smt2(r))
        }
        Constraint::Implies(l, r) => {
            format!("(=> {} {})", constraint_to_smt2(l), constraint_to_smt2(r))
        }
        Constraint::Not(c) => format!("(not {})", constraint_to_smt2(c)),
        _ => "(true)".to_string(),
    }
}

pub fn smt2_header() -> String {
    "(set-logic QF_NRA)\n(set-option :produce-models true)\n".to_string()
}

pub fn declare_vars(vars: &[&str]) -> String {
    vars.iter()
        .map(|v| format!("(declare-fun {} () Real)", v))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn check_sat_query(constraints: &[Constraint]) -> String {
    let body: Vec<String> = constraints.iter().map(constraint_to_smt2).collect();
    format!(
        "{}\n{}\n(assert {})\n(check-sat)\n(get-model)",
        smt2_header(),
        declare_vars(
            &collect_vars(constraints)
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
        ),
        body.join("\n(assert ")
    )
}

fn collect_vars(constraints: &[Constraint]) -> Vec<String> {
    let mut vars = Vec::new();
    for c in constraints {
        collect_vars_inner(c, &mut vars);
    }
    vars.sort();
    vars.dedup();
    vars
}

fn collect_vars_inner(c: &Constraint, acc: &mut Vec<String>) {
    match c {
        Constraint::Eq(v, _)
        | Constraint::Lt(v, _)
        | Constraint::Le(v, _)
        | Constraint::Gt(v, _)
        | Constraint::Ge(v, _)
        | Constraint::Within(v, _, _) => acc.push(v.clone()),
        Constraint::And(l, r) | Constraint::Or(l, r) | Constraint::Implies(l, r) => {
            collect_vars_inner(l, acc);
            collect_vars_inner(r, acc);
        }
        Constraint::Not(c) => collect_vars_inner(c, acc),
        Constraint::ForAll { body, .. } => collect_vars_inner(body, acc),
        Constraint::TemporalAlways { body, .. } => collect_vars_inner(body, acc),
    }
}
