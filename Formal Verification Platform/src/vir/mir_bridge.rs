//! Keller-VIR Bridge for Rust MIR.
//!
//! Translates Rust MIR structures (`rustc_middle::mir`) into canonical Keller-VIR:
//! - Maps `AssertKind` to `DiagnosticKind`
//! - Constructs SSA basic blocks and transitions
//! - Emits VIR functions for full-body verification and cross-language IR export.

use rustc_middle::mir::{AssertKind, BasicBlock, BinOp, Body, Operand, TerminatorKind};
use rustc_middle::ty::TyCtxt;

use crate::vir::ast::*;
use crate::vir::effect::{Effect, EffectSet};

/// Map Rust MIR `AssertKind` to Keller-VIR `DiagnosticKind`.
pub fn lower_mir_assert_kind<'tcx>(kind: &AssertKind<Operand<'tcx>>) -> DiagnosticKind {
    match kind {
        AssertKind::Overflow(BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Shl | BinOp::Shr, ..) => {
            DiagnosticKind::Overflow
        }
        AssertKind::OverflowNeg(..) => DiagnosticKind::Overflow,
        AssertKind::DivisionByZero(..) | AssertKind::RemainderByZero(..) => {
            DiagnosticKind::DivisionByZero
        }
        AssertKind::BoundsCheck { .. } => DiagnosticKind::OutOfBounds,
        AssertKind::MisalignedPointerDereference { .. } => DiagnosticKind::MemorySafety,
        _ => DiagnosticKind::UserAssertion,
    }
}

/// Lower a Rust MIR BasicBlock label into Keller-VIR block identifier.
pub fn lower_mir_block_label(bb: BasicBlock) -> String {
    format!("bb{}", bb.as_u32())
}

/// Lower a Rust MIR function body into a Keller-VIR function.
pub fn mir_to_vir_function<'tcx>(
    _tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
    fn_name: &str,
) -> VirFunction {
    let mut blocks = Vec::new();
    let mut effects = EffectSet::default();

    for (bb, data) in body.basic_blocks.iter_enumerated() {
        let label = lower_mir_block_label(bb);
        let mut statements = Vec::new();

        if let Some(ref term) = data.terminator {
            match &term.kind {
                TerminatorKind::Assert {
                    cond,
                    expected,
                    msg,
                    target,
                    ..
                } => {
                    effects.add(Effect::Panic);
                    let diag = lower_mir_assert_kind(msg);
                    let cond_name = format!("{:?}", cond);
                    let guard_expr = if *expected {
                        VirExpr::Var(cond_name)
                    } else {
                        VirExpr::Unary(VirUnaryOp::Not, Box::new(VirExpr::Var(cond_name)))
                    };
                    statements.push(VirStatement::Assert(guard_expr, diag));

                    blocks.push(VirBasicBlock {
                        label,
                        statements,
                        terminator: VirTerminator::Goto(lower_mir_block_label(*target)),
                    });
                    continue;
                }
                TerminatorKind::Goto { target } => {
                    blocks.push(VirBasicBlock {
                        label,
                        statements,
                        terminator: VirTerminator::Goto(lower_mir_block_label(*target)),
                    });
                    continue;
                }
                TerminatorKind::SwitchInt { discr, targets } => {
                    let discr_name = format!("{:?}", discr);
                    let otherwise = lower_mir_block_label(targets.otherwise());
                    if targets.all_targets().len() == 2 {
                        let then_bb = lower_mir_block_label(targets.all_targets()[0]);
                        blocks.push(VirBasicBlock {
                            label,
                            statements,
                            terminator: VirTerminator::Branch {
                                guard: VirExpr::Var(discr_name),
                                then_block: then_bb,
                                else_block: otherwise,
                            },
                        });
                    } else {
                        blocks.push(VirBasicBlock {
                            label,
                            statements,
                            terminator: VirTerminator::Goto(otherwise),
                        });
                    }
                    continue;
                }
                TerminatorKind::Return => {
                    blocks.push(VirBasicBlock {
                        label,
                        statements,
                        terminator: VirTerminator::Return(None),
                    });
                    continue;
                }
                TerminatorKind::Unreachable => {
                    blocks.push(VirBasicBlock {
                        label,
                        statements,
                        terminator: VirTerminator::Unreachable,
                    });
                    continue;
                }
                _ => {
                    blocks.push(VirBasicBlock {
                        label,
                        statements,
                        terminator: VirTerminator::Return(None),
                    });
                    continue;
                }
            }
        } else {
            blocks.push(VirBasicBlock {
                label,
                statements,
                terminator: VirTerminator::Return(None),
            });
        }
    }

    let param_count = body.arg_count;
    let mut params = Vec::new();
    for i in 1..=param_count {
        params.push(VirParam {
            name: format!("_{i}"),
            ty: VirType::Int(64),
        });
    }

    VirFunction {
        name: fn_name.to_string(),
        params,
        return_type: VirType::Bool,
        effects,
        blocks,
    }
}
