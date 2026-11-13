//! Keller-VIR Lowering Bridge: Translates ASTs from language frontends into Keller-VIR.
//!
//! Provides bidirectional lowering from Python AST expressions (`PyExpr`) and
//! verification obligations (`RawObligation`) into unified Keller-VIR AST (`VirExpr`,
//! `VirStatement`, `VirFunction`), enabling unified formal reasoning and SMT-LIB 2.6 emission.

use std::collections::HashSet;

use crate::python::solver::{sanitize_ident, PyExpr, RawObligation};
use crate::vir::ast::*;
use crate::vir::effect::EffectSet;
use crate::vir::smt::SmtEmitter;

/// Lower a Python AST expression (`PyExpr`) into Keller-VIR intermediate expression (`VirExpr`).
pub fn lower_py_expr_to_vir(expr: &PyExpr) -> VirExpr {
    match expr {
        PyExpr::Int { val } => {
            if let Ok(i) = val.parse::<i128>() {
                VirExpr::ConstInt(i, 0)
            } else {
                VirExpr::Var(val.clone())
            }
        }
        PyExpr::Float { val } => VirExpr::ConstFloat(val.to_string()),
        PyExpr::Bool { val } => VirExpr::ConstBool(*val),
        PyExpr::Str { val } => VirExpr::Var(format!("str_{}", sanitize_ident(val))),
        PyExpr::NoneVal => VirExpr::ConstInt(0, 0),
        PyExpr::Var { name } => VirExpr::Var(sanitize_ident(name)),
        PyExpr::Unary { op, arg } => {
            let inner = Box::new(lower_py_expr_to_vir(arg));
            match op.as_str() {
                "-" => VirExpr::Unary(VirUnaryOp::Neg, inner),
                "not" => VirExpr::Unary(VirUnaryOp::Not, inner),
                _ => VirExpr::Unary(VirUnaryOp::Not, inner),
            }
        }
        PyExpr::BinOp { op, left, right } => {
            let l = Box::new(lower_py_expr_to_vir(left));
            let r = Box::new(lower_py_expr_to_vir(right));
            let vir_op = match op.as_str() {
                "+" => VirBinaryOp::Add,
                "-" => VirBinaryOp::Sub,
                "*" => VirBinaryOp::Mul,
                "/" | "//" => VirBinaryOp::Div,
                "%" => VirBinaryOp::Rem,
                "&" => VirBinaryOp::BitAnd,
                "|" => VirBinaryOp::BitOr,
                "^" => VirBinaryOp::BitXor,
                "<<" => VirBinaryOp::Shl,
                ">>" => VirBinaryOp::Shr,
                _ => VirBinaryOp::Add,
            };
            VirExpr::Binary(vir_op, l, r)
        }
        PyExpr::Compare { op, left, right } => {
            let l = Box::new(lower_py_expr_to_vir(left));
            let r = Box::new(lower_py_expr_to_vir(right));
            let vir_op = match op.as_str() {
                "==" => VirBinaryOp::Eq,
                "!=" => VirBinaryOp::Ne,
                "<" => VirBinaryOp::Lt,
                "<=" => VirBinaryOp::Le,
                ">" => VirBinaryOp::Gt,
                ">=" => VirBinaryOp::Ge,
                _ => VirBinaryOp::Eq,
            };
            VirExpr::Binary(vir_op, l, r)
        }
        PyExpr::And { items } => {
            if items.is_empty() {
                VirExpr::ConstBool(true)
            } else if items.len() == 1 {
                lower_py_expr_to_vir(&items[0])
            } else {
                let mut iter = items.iter();
                let first = lower_py_expr_to_vir(iter.next().unwrap());
                iter.fold(first, |acc, next| {
                    VirExpr::Binary(
                        VirBinaryOp::And,
                        Box::new(acc),
                        Box::new(lower_py_expr_to_vir(next)),
                    )
                })
            }
        }
        PyExpr::Or { items } => {
            if items.is_empty() {
                VirExpr::ConstBool(false)
            } else if items.len() == 1 {
                lower_py_expr_to_vir(&items[0])
            } else {
                let mut iter = items.iter();
                let first = lower_py_expr_to_vir(iter.next().unwrap());
                iter.fold(first, |acc, next| {
                    VirExpr::Binary(
                        VirBinaryOp::Or,
                        Box::new(acc),
                        Box::new(lower_py_expr_to_vir(next)),
                    )
                })
            }
        }
        PyExpr::Call { fn_, args } => {
            let name = fn_.as_deref().unwrap_or("");
            match name {
                "len" if !args.is_empty() => {
                    let mut vars = HashSet::new();
                    args[0].collect_vars(&mut vars);
                    let target_name = vars.into_iter().next().unwrap_or_else(|| "seq".to_string());
                    VirExpr::Var(format!("len_{}", sanitize_ident(&target_name)))
                }
                "abs" if !args.is_empty() => {
                    let inner = lower_py_expr_to_vir(&args[0]);
                    VirExpr::Ite(
                        Box::new(VirExpr::Binary(
                            VirBinaryOp::Ge,
                            Box::new(inner.clone()),
                            Box::new(VirExpr::ConstInt(0, 0)),
                        )),
                        Box::new(inner.clone()),
                        Box::new(VirExpr::Unary(VirUnaryOp::Neg, Box::new(inner))),
                    )
                }
                "min" if args.len() >= 2 => {
                    let a = lower_py_expr_to_vir(&args[0]);
                    let b = lower_py_expr_to_vir(&args[1]);
                    VirExpr::Ite(
                        Box::new(VirExpr::Binary(
                            VirBinaryOp::Le,
                            Box::new(a.clone()),
                            Box::new(b.clone()),
                        )),
                        Box::new(a),
                        Box::new(b),
                    )
                }
                "max" if args.len() >= 2 => {
                    let a = lower_py_expr_to_vir(&args[0]);
                    let b = lower_py_expr_to_vir(&args[1]);
                    VirExpr::Ite(
                        Box::new(VirExpr::Binary(
                            VirBinaryOp::Ge,
                            Box::new(a.clone()),
                            Box::new(b.clone()),
                        )),
                        Box::new(a),
                        Box::new(b),
                    )
                }
                _ => VirExpr::Call {
                    target: sanitize_ident(name),
                    args: args.iter().map(lower_py_expr_to_vir).collect(),
                },
            }
        }
        PyExpr::Attr { value, attr } => {
            let base = lower_py_expr_to_vir(value);
            match base {
                VirExpr::Var(v) => VirExpr::Var(format!("{v}_{attr}")),
                _ => VirExpr::Var(format!("val_{attr}")),
            }
        }
        PyExpr::Subscript { value, slice } => {
            let base = lower_py_expr_to_vir(value);
            let s = lower_py_expr_to_vir(slice);
            let b_str = match base {
                VirExpr::Var(v) => v,
                _ => "arr".to_string(),
            };
            let s_str = match s {
                VirExpr::Var(sv) => sv,
                VirExpr::ConstInt(val, _) => val.to_string(),
                _ => "idx".to_string(),
            };
            VirExpr::Var(format!("{b_str}_{s_str}"))
        }
        _ => VirExpr::ConstInt(0, 0),
    }
}

/// Lowers a Python verification obligation into a Keller-VIR Function and BasicBlock.
pub fn lower_obligation_to_vir(
    raw: &RawObligation,
    extra_guards: &[PyExpr],
) -> (VirFunction, SmtEmitter) {
    let mut stmts = Vec::new();
    let mut emitter = SmtEmitter::new("ALL");

    // Lower path guards
    for guard in &raw.guards {
        let vir_guard = lower_py_expr_to_vir(guard);
        stmts.push(VirStatement::Assert(vir_guard, DiagnosticKind::UserAssertion));
    }

    // Lower induction bounds
    for bound in &raw.induction_bounds {
        let vir_bound = lower_py_expr_to_vir(bound);
        stmts.push(VirStatement::Assert(vir_bound, DiagnosticKind::UserAssertion));
    }

    // Lower explicit loop bounds
    for lb in &raw.loop_bounds {
        let var_expr = VirExpr::Var(sanitize_ident(&lb.variable));
        if let Some(ref lower) = lb.lower {
            let l_expr = lower_py_expr_to_vir(lower);
            stmts.push(VirStatement::Assert(
                VirExpr::Binary(VirBinaryOp::Ge, Box::new(var_expr.clone()), Box::new(l_expr)),
                DiagnosticKind::OutOfBounds,
            ));
        }
        if let Some(ref upper) = lb.upper {
            let u_expr = lower_py_expr_to_vir(upper);
            stmts.push(VirStatement::Assert(
                VirExpr::Binary(VirBinaryOp::Lt, Box::new(var_expr.clone()), Box::new(u_expr)),
                DiagnosticKind::OutOfBounds,
            ));
        }
    }

    // Lower helper preconditions
    for extra in extra_guards {
        let vir_extra = lower_py_expr_to_vir(extra);
        stmts.push(VirStatement::Assert(vir_extra, DiagnosticKind::UserAssertion));
    }

    // Target defect condition
    let target_vir = lower_py_expr_to_vir(&raw.target);
    stmts.push(VirStatement::Assert(target_vir.clone(), DiagnosticKind::Overflow));

    // Convert statements to SMT emitter assertions
    for stmt in &stmts {
        if let VirStatement::Assert(cond, _) = stmt {
            let smt_cond = emitter.lower_expr(cond);
            if !smt_cond.is_empty() && smt_cond != "true" {
                emitter.assert_formula(&smt_cond);
            }
        }
    }

    // Diagnostic kind classification
    let _diag_kind = match raw.kind.as_str() {
        "div_zero" => DiagnosticKind::DivisionByZero,
        "index_bounds" => DiagnosticKind::OutOfBounds,
        "none_deref" => DiagnosticKind::NullPointer,
        _ => DiagnosticKind::UserAssertion,
    };

    let block = VirBasicBlock {
        label: format!("{}_entry", raw.function),
        statements: stmts,
        terminator: VirTerminator::Return(None),
    };

    let func = VirFunction {
        name: format!("{}_{}_L{}", raw.module, raw.function, raw.line),
        params: raw
            .param_order
            .iter()
            .map(|p| VirParam {
                name: p.clone(),
                ty: VirType::Int(64),
            })
            .collect(),
        return_type: VirType::Bool,
        effects: EffectSet::default(),
        blocks: vec![block],
    };

    (func, emitter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lower_py_expr_arithmetic_to_vir() {
        let py_expr = PyExpr::BinOp {
            op: "+".to_string(),
            left: Box::new(PyExpr::Var { name: "x".to_string() }),
            right: Box::new(PyExpr::Int { val: "42".to_string() }),
        };

        let vir = lower_py_expr_to_vir(&py_expr);
        assert_eq!(
            vir,
            VirExpr::Binary(
                VirBinaryOp::Add,
                Box::new(VirExpr::Var("x".to_string())),
                Box::new(VirExpr::ConstInt(42, 0)),
            )
        );

        let mut emitter = SmtEmitter::new("ALL");
        let smt = emitter.lower_expr(&vir);
        assert_eq!(smt, "(+ x 42)");
    }

    #[test]
    fn test_lower_py_compare_and_ite_to_vir() {
        let py_call = PyExpr::Call {
            fn_: Some("abs".to_string()),
            args: vec![PyExpr::Var { name: "delta".to_string() }],
        };

        let vir = lower_py_expr_to_vir(&py_call);
        let mut emitter = SmtEmitter::new("ALL");
        let smt = emitter.lower_expr(&vir);
        assert_eq!(smt, "(ite (>= delta 0) delta (- delta))");
    }
}
