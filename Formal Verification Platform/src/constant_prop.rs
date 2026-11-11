use rustc_middle::mir::{BasicBlock, BinOp, Body, Operand, Rvalue, StatementKind};
use rustc_middle::ty::{Ty, TyCtxt, TyKind, TypingEnv, TypingMode};
use std::collections::HashMap;

use crate::types::IntType;

/// Map from MIR local index (as usize) to a known constant value.
pub type ConstMap = HashMap<usize, i128>;

/// Try to evaluate an operand to a concrete integer value using the MIR constant evaluator.
/// Uses `try_eval_scalar_int()` with proper TyCtxt instead of parsing debug strings.
pub fn eval_operand<'tcx>(
    operand: &Operand<'tcx>,
    _mir_body: &Body<'tcx>,
    const_map: &ConstMap,
    tcx: TyCtxt<'tcx>,
) -> Option<i128> {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => {
            if !place.projection.is_empty() {
                return None;
            }
            let local = place.local;
            // Check our const map first (populated by propagate_constants pass)
            if let Some(&v) = const_map.get(&local.index()) {
                return Some(v);
            }
            // If not in map, it's a runtime value we can't determine statically
            None
        }
        Operand::Constant(c) => {
            let param_env = rustc_middle::ty::ParamEnv::empty();
            let typing_env = TypingEnv::new(param_env, TypingMode::PostAnalysis);
            let ty = c.const_.ty();
            // Only integer constants are of interest; `try_eval_scalar_int`
            // handles bools/chars/etc. but they cannot feed integer checks.
            if !matches!(ty.kind(), TyKind::Int(_) | TyKind::Uint(_)) {
                return None;
            }
            let val = c.const_.try_eval_scalar_int(tcx, typing_env)?;
            let size = ty.primitive_size(tcx);
            // `try_to_bits` (unlike `try_into::<i128>()`) does not panic when
            // the scalar is narrower than 16 bytes (e.g. an 8-byte usize const).
            match val.try_to_bits(size) {
                Ok(bits) => {
                    if ty.is_signed() {
                        Some(size.sign_extend(bits))
                    } else {
                        // Do not replace an exact unsigned value with signed MAX.
                        // Large literals are handled by the symbolic decimal path.
                        i128::try_from(bits).ok()
                    }
                }
                Err(_) => None,
            }
        }
        Operand::RuntimeChecks(_) => None,
    }
}

/// Infer the integer type of a MIR local from its type.
pub fn infer_int_type(local_ty: Ty<'_>) -> Option<IntType> {
    match local_ty.kind() {
        TyKind::Int(t) => match t {
            rustc_middle::ty::IntTy::I8 => Some(IntType::I8),
            rustc_middle::ty::IntTy::I16 => Some(IntType::I16),
            rustc_middle::ty::IntTy::I32 => Some(IntType::I32),
            rustc_middle::ty::IntTy::I64 => Some(IntType::I64),
            rustc_middle::ty::IntTy::I128 => Some(IntType::I128),
            rustc_middle::ty::IntTy::Isize => Some(IntType::Isize),
        },
        TyKind::Uint(t) => match t {
            rustc_middle::ty::UintTy::U8 => Some(IntType::U8),
            rustc_middle::ty::UintTy::U16 => Some(IntType::U16),
            rustc_middle::ty::UintTy::U32 => Some(IntType::U32),
            rustc_middle::ty::UintTy::U64 => Some(IntType::U64),
            rustc_middle::ty::UintTy::U128 => Some(IntType::U128),
            rustc_middle::ty::UintTy::Usize => Some(IntType::Usize),
        },
        _ => None,
    }
}

/// Infer the integer type of an operand (local variable or constant).
/// This is what wires the type-aware bounds into the Z3 constraints.
pub fn operand_int_type<'tcx>(
    operand: &Operand<'tcx>,
    mir: &Body<'tcx>,
    tcx: TyCtxt<'tcx>,
) -> Option<IntType> {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => {
            infer_int_type(place.ty(&mir.local_decls, tcx).ty)
        }
        Operand::Constant(c) => infer_int_type(c.const_.ty()),
        Operand::RuntimeChecks(_) => None,
    }
}

/// Fold a single rvalue to a constant, given the constants known so far.
///
/// Returns `None` for anything we cannot evaluate (a runtime operand, an
/// unsupported operator, or an operation that traps such as division by zero).
fn eval_rvalue<'tcx>(
    rvalue: &Rvalue<'tcx>,
    mir: &Body<'tcx>,
    const_values: &ConstMap,
    tcx: TyCtxt<'tcx>,
) -> Option<i128> {
    match rvalue {
        Rvalue::Use(operand, _) => eval_operand(operand, mir, const_values, tcx),
        Rvalue::BinaryOp(op, box (left_op, right_op)) => {
            let l = eval_operand(left_op, mir, const_values, tcx)?;
            let r = eval_operand(right_op, mir, const_values, tcx)?;
            fold_binary(*op, l, r, operand_int_type(left_op, mir, tcx)?)
        }
        _ => None,
    }
}

/// Exact decimal integer literal, including the unsigned half above i128::MAX.
pub fn integer_literal<'tcx>(operand: &Operand<'tcx>, tcx: TyCtxt<'tcx>) -> Option<String> {
    let Operand::Constant(c) = operand else {
        return None;
    };
    let ty = c.const_.ty();
    if !matches!(ty.kind(), TyKind::Int(_) | TyKind::Uint(_)) {
        return None;
    }
    let env = TypingEnv::new(
        rustc_middle::ty::ParamEnv::empty(),
        TypingMode::PostAnalysis,
    );
    let bits = c
        .const_
        .try_eval_scalar_int(tcx, env)?
        .try_to_bits(ty.primitive_size(tcx))
        .ok()?;
    Some(if ty.is_signed() {
        ty.primitive_size(tcx).sign_extend(bits).to_string()
    } else {
        bits.to_string()
    })
}

/// A boolean constant operand, as the compiler wrote it.
///
/// [`eval_operand`] answers only for integer types on purpose, so the assertion
/// walk needs this separately: `assert(const false, …)` is how the coroutine
/// transform spells "resumed after completion", and a condition the compiler has
/// already decided must not be handed to the solver as an unconstrained one (S6).
pub fn const_bool<'tcx>(operand: &Operand<'tcx>, tcx: TyCtxt<'tcx>) -> Option<bool> {
    let Operand::Constant(c) = operand else {
        return None;
    };
    if !c.const_.ty().is_bool() {
        return None;
    }
    let env = TypingEnv::new(
        rustc_middle::ty::ParamEnv::empty(),
        TypingMode::PostAnalysis,
    );
    let scalar = c.const_.try_eval_scalar_int(tcx, env)?;
    Some(scalar.to_bits(scalar.size()) != 0)
}

fn truncate(value: i128, ty: IntType) -> i128 {
    let bits = ty.bit_width();
    if bits == 128 {
        return value;
    }
    let masked = (value as u128) & ((1u128 << bits) - 1);
    if !ty.is_unsigned() && masked >= (1u128 << (bits - 1)) {
        masked as i128 - (1i128 << bits)
    } else {
        masked as i128
    }
}

fn fold_binary(op: BinOp, left: i128, right: i128, ty: IntType) -> Option<i128> {
    // i128 is not a carrier for arbitrary unsigned-128 results; stay unknown.
    if ty == IntType::U128 {
        return None;
    }
    let value = match op {
        BinOp::Add => left.wrapping_add(right),
        BinOp::Sub => left.wrapping_sub(right),
        BinOp::Mul => left.wrapping_mul(right),
        BinOp::Div => left.checked_div(right)?,
        BinOp::Rem => left.checked_rem(right)?,
        BinOp::BitAnd => left & right,
        BinOp::BitOr => left | right,
        BinOp::BitXor => left ^ right,
        BinOp::Shl if right >= 0 && right < ty.bit_width() as i128 => {
            left.wrapping_shl(right as u32)
        }
        BinOp::Shr if right >= 0 && right < ty.bit_width() as i128 => {
            left.wrapping_shr(right as u32)
        }
        _ => return None,
    };
    Some(truncate(value, ty))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folded_values_match_fixed_width_rust_exhaustively() {
        for a in i8::MIN..=i8::MAX {
            for b in i8::MIN..=i8::MAX {
                for (op, result) in [
                    (BinOp::Add, a.wrapping_add(b)),
                    (BinOp::Sub, a.wrapping_sub(b)),
                    (BinOp::Mul, a.wrapping_mul(b)),
                    (BinOp::BitAnd, a & b),
                    (BinOp::BitOr, a | b),
                    (BinOp::BitXor, a ^ b),
                ] {
                    assert_eq!(
                        fold_binary(op, a.into(), b.into(), IntType::I8),
                        Some(result.into())
                    );
                }
            }
            for amount in 0..8 {
                assert_eq!(
                    fold_binary(BinOp::Shl, a.into(), amount, IntType::I8),
                    Some(a.wrapping_shl(amount as u32).into())
                );
                assert_eq!(
                    fold_binary(BinOp::Shr, a.into(), amount, IntType::I8),
                    Some(a.wrapping_shr(amount as u32).into())
                );
            }
        }
        assert_eq!(fold_binary(BinOp::Shl, 128, 1, IntType::U8), Some(0));
        assert_eq!(fold_binary(BinOp::Shr, 1, -1, IntType::U8), None);
    }
}

/// Run constant propagation over the MIR body, returning **only** locals whose
/// value is a single constant across every reachable definition.
///
/// This is deliberately a must-analysis, because the previous implementation
/// was unsound: it walked the blocks in index order keeping one value per local
/// (last write wins), with nothing to remove a local once it had been assigned
/// a constant. So for
///
/// ```ignore
/// let m: u8 = if flag { 255 } else { 0 };
/// let total = m + 100;   // overflows only when flag is true
/// ```
///
/// the `0` arm was visited last, `m` was recorded as `0`, `0 + 100` looked
/// safe, and the check was skipped entirely — a *false PASS* on a reachable
/// overflow. A verifier that reports "proved" when it has not is worse than one
/// that reports "unknown", so now a local is only reported when every
/// definition agrees; any runtime definition, or two disagreeing constants,
/// removes it (it stays absent, and callers fall back to the symbolic path).
///
/// Iterating to a fixpoint still resolves dependent constants (`let b = a + 1`
/// after `let a = 5`). If the iteration is cut short the result is still sound:
/// an absent local is simply treated as unknown, never as a value.
pub fn propagate_constants<'tcx>(
    mir: &Body<'tcx>,
    reachable: &std::collections::HashSet<BasicBlock>,
    tcx: TyCtxt<'tcx>,
) -> ConstMap {
    // Round 1 resolves constants whose operands are literals; each further
    // round resolves values that depend on the previous round's discoveries.
    // The lattice only ever grows, so this converges quickly; the cap keeps a
    // pathological CFG from looping.
    const MAX_ROUNDS: usize = 16;

    let mut known: ConstMap = HashMap::new();

    for _ in 0..MAX_ROUNDS {
        let mut next: ConstMap = HashMap::new();
        // Locals proven *not* to hold a single constant (conflicting constants,
        // or a runtime definition). Checked before any further writes so a
        // local can never come back from the dead within a round.
        let mut poisoned: std::collections::HashSet<usize> = std::collections::HashSet::new();

        for (bb, block_data) in mir.basic_blocks.iter_enumerated() {
            if !reachable.contains(&bb) {
                continue;
            }
            for stmt in &block_data.statements {
                if let StatementKind::Assign(box (place, rvalue)) = &stmt.kind {
                    let local_idx = place.local.index();
                    if poisoned.contains(&local_idx) {
                        continue;
                    }

                    // Writing through a projection (`_1.0 = ...`) defines only a
                    // field, never the whole local, so it must not be recorded
                    // as a constant for the local itself.
                    let value = if place.projection.is_empty() {
                        eval_rvalue(rvalue, mir, &known, tcx)
                    } else {
                        None
                    };

                    match value {
                        Some(v) => match next.get(&local_idx) {
                            None => {
                                next.insert(local_idx, v);
                            }
                            // Same constant on another path: still constant.
                            Some(&prev) if prev == v => {}
                            // Two different constants: not a single value.
                            Some(_) => {
                                next.remove(&local_idx);
                                poisoned.insert(local_idx);
                            }
                        },
                        // A definition we cannot evaluate means the local is not
                        // a compile-time constant at all.
                        None => {
                            next.remove(&local_idx);
                            poisoned.insert(local_idx);
                        }
                    }
                }
            }
        }

        if next == known {
            break;
        }
        known = next;
    }

    known
}
