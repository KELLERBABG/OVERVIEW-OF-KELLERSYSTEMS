//! Keller-VIR Optimization Passes.
//!
//! Implements canonical IR optimizations:
//! 1. Algebraic identity reduction & constant folding on `VirExpr`
//! 2. Trivial assertion pruning (`assert true`)
//! 3. Reachability analysis & dead basic block elimination

use std::collections::{HashSet, VecDeque};

use crate::vir::ast::*;

/// Recursively constant-fold and algebraically simplify a VIR expression.
pub fn fold_expr(expr: VirExpr) -> VirExpr {
    match expr {
        VirExpr::Binary(op, left, right) => {
            let l = fold_expr(*left);
            let r = fold_expr(*right);

            // Constant folding on two integer constants
            if let (VirExpr::ConstInt(lv, l_bits), VirExpr::ConstInt(rv, r_bits)) = (&l, &r) {
                let bits = if *l_bits > 0 { *l_bits } else { *r_bits };
                match op {
                    VirBinaryOp::Add => return VirExpr::ConstInt(lv.wrapping_add(*rv), bits),
                    VirBinaryOp::Sub => return VirExpr::ConstInt(lv.wrapping_sub(*rv), bits),
                    VirBinaryOp::Mul => return VirExpr::ConstInt(lv.wrapping_mul(*rv), bits),
                    VirBinaryOp::Div if *rv != 0 => return VirExpr::ConstInt(lv / rv, bits),
                    VirBinaryOp::Rem if *rv != 0 => return VirExpr::ConstInt(lv % rv, bits),
                    VirBinaryOp::Eq => return VirExpr::ConstBool(lv == rv),
                    VirBinaryOp::Ne => return VirExpr::ConstBool(lv != rv),
                    VirBinaryOp::Lt => return VirExpr::ConstBool(lv < rv),
                    VirBinaryOp::Le => return VirExpr::ConstBool(lv <= rv),
                    VirBinaryOp::Gt => return VirExpr::ConstBool(lv > rv),
                    VirBinaryOp::Ge => return VirExpr::ConstBool(lv >= rv),
                    _ => {}
                }
            }

            // Constant folding on boolean constants
            if let (VirExpr::ConstBool(lb), VirExpr::ConstBool(rb)) = (&l, &r) {
                match op {
                    VirBinaryOp::And => return VirExpr::ConstBool(*lb && *rb),
                    VirBinaryOp::Or => return VirExpr::ConstBool(*lb || *rb),
                    VirBinaryOp::Eq => return VirExpr::ConstBool(lb == rb),
                    VirBinaryOp::Ne => return VirExpr::ConstBool(lb != rb),
                    _ => {}
                }
            }

            // Algebraic identities
            match op {
                VirBinaryOp::Add => {
                    if let VirExpr::ConstInt(0, _) = r {
                        return l;
                    }
                    if let VirExpr::ConstInt(0, _) = l {
                        return r;
                    }
                }
                VirBinaryOp::Sub => {
                    if let VirExpr::ConstInt(0, _) = r {
                        return l;
                    }
                }
                VirBinaryOp::Mul => {
                    if let VirExpr::ConstInt(1, _) = r {
                        return l;
                    }
                    if let VirExpr::ConstInt(1, _) = l {
                        return r;
                    }
                    if let VirExpr::ConstInt(0, bits) = r {
                        return VirExpr::ConstInt(0, bits);
                    }
                    if let VirExpr::ConstInt(0, bits) = l {
                        return VirExpr::ConstInt(0, bits);
                    }
                }
                VirBinaryOp::And => {
                    if let VirExpr::ConstBool(true) = r {
                        return l;
                    }
                    if let VirExpr::ConstBool(true) = l {
                        return r;
                    }
                    if let VirExpr::ConstBool(false) = r {
                        return VirExpr::ConstBool(false);
                    }
                    if let VirExpr::ConstBool(false) = l {
                        return VirExpr::ConstBool(false);
                    }
                }
                VirBinaryOp::Or => {
                    if let VirExpr::ConstBool(false) = r {
                        return l;
                    }
                    if let VirExpr::ConstBool(false) = l {
                        return r;
                    }
                    if let VirExpr::ConstBool(true) = r {
                        return VirExpr::ConstBool(true);
                    }
                    if let VirExpr::ConstBool(true) = l {
                        return VirExpr::ConstBool(true);
                    }
                }
                _ => {}
            }

            VirExpr::Binary(op, Box::new(l), Box::new(r))
        }
        VirExpr::Unary(op, inner) => {
            let folded = fold_expr(*inner);
            match op {
                VirUnaryOp::Neg => {
                    if let VirExpr::ConstInt(v, bits) = folded {
                        VirExpr::ConstInt(-v, bits)
                    } else {
                        VirExpr::Unary(op, Box::new(folded))
                    }
                }
                VirUnaryOp::Not => {
                    if let VirExpr::ConstBool(b) = folded {
                        VirExpr::ConstBool(!b)
                    } else {
                        VirExpr::Unary(op, Box::new(folded))
                    }
                }
            }
        }
        VirExpr::Ite(cond, then_e, else_e) => {
            let c = fold_expr(*cond);
            match c {
                VirExpr::ConstBool(true) => fold_expr(*then_e),
                VirExpr::ConstBool(false) => fold_expr(*else_e),
                _ => VirExpr::Ite(
                    Box::new(c),
                    Box::new(fold_expr(*then_e)),
                    Box::new(fold_expr(*else_e)),
                ),
            }
        }
        other => other,
    }
}

/// Optimizes a single VIR function: folds expressions, prunes trivial assertions,
/// and eliminates unreachable dead basic blocks.
pub fn optimize_function(mut func: VirFunction) -> VirFunction {
    for block in &mut func.blocks {
        let mut optimized_stmts = Vec::new();
        for stmt in block.statements.drain(..) {
            match stmt {
                VirStatement::Assign(var, expr) => {
                    optimized_stmts.push(VirStatement::Assign(var, fold_expr(expr)));
                }
                VirStatement::Store {
                    mem_token,
                    address,
                    value,
                } => {
                    optimized_stmts.push(VirStatement::Store {
                        mem_token,
                        address: fold_expr(address),
                        value: fold_expr(value),
                    });
                }
                VirStatement::Assert(expr, diag) => {
                    let folded = fold_expr(expr);
                    // Prune assertions that are mathematically proved statically
                    if let VirExpr::ConstBool(true) = folded {
                        continue;
                    }
                    optimized_stmts.push(VirStatement::Assert(folded, diag));
                }
                other => optimized_stmts.push(other),
            }
        }
        block.statements = optimized_stmts;

        // Simplify branch terminators with constant guards
        block.terminator = match block.terminator.clone() {
            VirTerminator::Branch {
                guard,
                then_block,
                else_block,
            } => {
                let folded_guard = fold_expr(guard);
                match folded_guard {
                    VirExpr::ConstBool(true) => VirTerminator::Goto(then_block),
                    VirExpr::ConstBool(false) => VirTerminator::Goto(else_block),
                    _ => VirTerminator::Branch {
                        guard: folded_guard,
                        then_block,
                        else_block,
                    },
                }
            }
            other => other,
        };
    }

    // Eliminate unreachable blocks if there are multiple blocks
    if func.blocks.len() > 1 {
        let mut reachable = HashSet::new();
        let mut queue = VecDeque::new();

        let entry_label = func.blocks[0].label.clone();
        reachable.insert(entry_label.clone());
        queue.push_back(entry_label);

        while let Some(current) = queue.pop_front() {
            if let Some(blk) = func.blocks.iter().find(|b| b.label == current) {
                match &blk.terminator {
                    VirTerminator::Goto(target) => {
                        if reachable.insert(target.clone()) {
                            queue.push_back(target.clone());
                        }
                    }
                    VirTerminator::Branch {
                        then_block,
                        else_block,
                        ..
                    } => {
                        if reachable.insert(then_block.clone()) {
                            queue.push_back(then_block.clone());
                        }
                        if reachable.insert(else_block.clone()) {
                            queue.push_back(else_block.clone());
                        }
                    }
                    _ => {}
                }
            }
        }

        func.blocks.retain(|b| reachable.contains(&b.label));
    }

    func
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vir_constant_folding() {
        let expr = VirExpr::Binary(
            VirBinaryOp::Add,
            Box::new(VirExpr::ConstInt(10, 32)),
            Box::new(VirExpr::ConstInt(32, 32)),
        );
        assert_eq!(fold_expr(expr), VirExpr::ConstInt(42, 32));
    }

    #[test]
    fn test_vir_algebraic_identities() {
        let expr = VirExpr::Binary(
            VirBinaryOp::Mul,
            Box::new(VirExpr::Var("x".to_string())),
            Box::new(VirExpr::ConstInt(1, 32)),
        );
        assert_eq!(fold_expr(expr), VirExpr::Var("x".to_string()));

        let expr_zero = VirExpr::Binary(
            VirBinaryOp::Mul,
            Box::new(VirExpr::Var("y".to_string())),
            Box::new(VirExpr::ConstInt(0, 32)),
        );
        assert_eq!(fold_expr(expr_zero), VirExpr::ConstInt(0, 32));
    }

    #[test]
    fn test_vir_dead_assertion_and_branch_pruning() {
        let func = VirFunction {
            name: "test_fn".to_string(),
            params: vec![],
            return_type: VirType::Bool,
            effects: Default::default(),
            blocks: vec![
                VirBasicBlock {
                    label: "entry".to_string(),
                    statements: vec![
                        VirStatement::Assert(VirExpr::ConstBool(true), DiagnosticKind::UserAssertion),
                        VirStatement::Assert(VirExpr::Var("x".to_string()), DiagnosticKind::Overflow),
                    ],
                    terminator: VirTerminator::Branch {
                        guard: VirExpr::ConstBool(true),
                        then_block: "live_bb".to_string(),
                        else_block: "dead_bb".to_string(),
                    },
                },
                VirBasicBlock {
                    label: "live_bb".to_string(),
                    statements: vec![],
                    terminator: VirTerminator::Return(None),
                },
                VirBasicBlock {
                    label: "dead_bb".to_string(),
                    statements: vec![],
                    terminator: VirTerminator::Unreachable,
                },
            ],
        };

        let opt = optimize_function(func);
        assert_eq!(opt.blocks.len(), 2); // dead_bb eliminated
        assert_eq!(opt.blocks[0].statements.len(), 1); // trivial assert pruned
        assert_eq!(
            opt.blocks[0].terminator,
            VirTerminator::Goto("live_bb".to_string())
        );
    }
}
