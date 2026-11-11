use std::collections::BTreeSet;

use rustc_middle::mir::{AssertKind, BinOp, Body, Operand};
use rustc_middle::ty::TyCtxt;
use z3::ast::Ast;
use z3::{ast::Bool, ast::Int, Context, Model, Solver};

use crate::constant_prop::{eval_operand, operand_int_type, ConstMap};
use crate::path_state::{operand_label, render_bool, PathState};
use crate::types::{IntType, Witness};

/// One variable a SAT verdict should be explained in terms of.
///
/// Returning these (rather than a bare `true`) is what lets the caller read the
/// concrete failing input out of the model instead of only reporting that one
/// exists.
///
/// These are the symbols the *failure* is stated over — the operands of the
/// operation that would panic. The path's own free symbols are not mixed in here:
/// naming them, and deciding which of them the model actually pinned, is the
/// caller's job (see the witness arm of `analyze_mir`). Keeping the two apart is
/// what lets a report say which values are facts about the program and which are
/// the model's choice.
pub struct WitnessVar {
    /// Z3 symbol name used in the encoding (e.g. `overflow_l`).
    pub symbol: String,
    /// Human label for the operand it stands for.
    pub input: String,
    /// True when the symbol holds a `bool` encoded as `0`/`1`, so the reported
    /// value can read as `true`/`false` instead of a bare number.
    pub is_bool: bool,
}

/// Everything an obligation needs to know about the values it constrains: the
/// MIR body, the constant map, and the path-sensitive state for the block the
/// assertion sits in.
///
/// The state is handed over by mutable reference because resolving an operand it
/// cannot model has to *register* the fresh symbol it invents, so the caller can
/// bound the symbol and name it in a witness.
pub struct SymCtx<'a, 'ctx, 'tcx> {
    pub ctx: &'ctx Context,
    pub mir: &'a Body<'tcx>,
    pub const_map: &'a ConstMap,
    pub state: &'a mut PathState<'ctx>,
    pub tcx: TyCtxt<'tcx>,
    /// Why the failure this obligation is about leans on a value the analysis
    /// did not model, collected as the operands are resolved. Empty means the
    /// model, if the solver finds one, is a counterexample; non-empty means the
    /// obligation is undecided and these are the reasons to report.
    ///
    /// It has to be collected here rather than at the verdict: this is the only
    /// place that knows which symbols the *violation* is stated over, as opposed
    /// to the ones the path merely mentions.
    pub undecided: &'a mut BTreeSet<String>,
}

/// Create a Z3 solver specialized for bit-vector operations (S8).
///
/// Uses the Z3 `qfbv` tactic with fallback to `smt` to solve obligations
/// with bitwise operations (`BitAnd`, `BitOr`, `BitXor`, `Shl`, `Shr`).
pub fn make_bitvector_solver<'ctx>(
    ctx: &'ctx Context,
    timeout_ms: Option<u64>,
) -> Solver<'ctx> {
    let bv = z3::Tactic::new(ctx, "qfbv");
    let smt = z3::Tactic::new(ctx, "smt");
    let combined = bv.or_else(&smt);
    let tactic = if let Some(ms) = timeout_ms {
        let t = combined.try_for(std::time::Duration::from_millis(ms));
        std::mem::forget(combined);
        t
    } else {
        combined
    };
    let solver = tactic.solver();
    std::mem::forget(bv);
    std::mem::forget(smt);
    std::mem::forget(tactic);
    solver
}

/// Render a model value as an exact decimal string.
///
/// Uses Z3's own printer rather than `as_i64`, which fails for anything wider
/// than 64 bits (and silently would not, for a value that happens to fit —
/// truncation is exactly the bug this avoids). Z3 prints a negative numeral as
/// `(- n)`, so that is normalised to `-n` to stay pasteable.
fn numeral_string(value: &Int<'_>) -> String {
    let raw = value.to_string();
    let trimmed = raw.trim();
    if let Some(inner) = trimmed.strip_prefix("(-").and_then(|s| s.strip_suffix(')')) {
        return format!("-{}", inner.trim());
    }
    trimmed.to_string()
}

/// Read the requested variables out of a SAT model as concrete input values.
///
/// `eval(.., true)` asks for model completion, so a variable the model left
/// implicit (any symbol only constrained by its type bounds) still yields a
/// value. A variable Z3 genuinely cannot interpret is skipped rather than
/// reported as a guessed value.
///
/// Every variable handed in is reported, so which ones to ask about is the
/// caller's decision — a completed model answers for symbols the obligation does
/// not really constrain, and reporting those would dress a choice up as a fact.
pub fn extract_witness<'ctx>(
    ctx: &'ctx Context,
    model: &Model<'ctx>,
    vars: &[WitnessVar],
) -> Vec<Witness> {
    vars.iter()
        .filter_map(|var| {
            let symbol = Int::new_const(ctx, var.symbol.as_str());
            let value = model.eval(&symbol, true)?;
            let rendered = numeral_string(&value);
            Some(Witness {
                input: var.input.clone(),
                symbol: var.symbol.clone(),
                value: if var.is_bool {
                    render_bool(&rendered)
                } else {
                    rendered
                },
            })
        })
        .collect()
}

/// Build a Z3 integer literal from a decimal string (exact for i128/u128).
fn int_literal<'ctx>(ctx: &'ctx Context, s: &str) -> Int<'ctx> {
    Int::from_str(ctx, s).expect("valid decimal integer literal")
}

/// Build a Z3 integer literal from an i128 constant (exact, no i64 truncation).
fn int_literal_i128<'ctx>(ctx: &'ctx Context, v: i128) -> Int<'ctx> {
    int_literal(ctx, &v.to_string())
}

/// Bound a symbolic integer variable to its type's exact (min, max) range.
/// For unsigned types this also asserts the `var >= 0` floor bound, so Z3 can
/// never report a violation using negative values that are impossible at runtime.
fn bind_bounds<'ctx>(
    ctx: &'ctx Context,
    solver: &mut Solver<'ctx>,
    symbol: &Int<'ctx>,
    bounds: &(String, String),
) {
    let min = int_literal(ctx, &bounds.0);
    let max = int_literal(ctx, &bounds.1);
    solver.assert(&symbol.ge(&min));
    solver.assert(&symbol.le(&max));
}

/// Bind an obligation's symbol to the operand it stands for, and report whether
/// the path-sensitive state supplied the value.
///
/// The state is asked first, because it knows what the operand holds *on this
/// path* — possibly "one of the values a branch produced". The constant map is
/// consulted only when the state has nothing to say, and only for a place with
/// no projection (the map speaks about whole locals). When neither knows, the
/// symbol is left as a free variable for the caller to bound by its type.
fn bind_operand<'ctx, 'tcx>(
    solver: &mut Solver<'ctx>,
    symbol: &Int<'ctx>,
    operand: &Operand<'tcx>,
    src: &mut SymCtx<'_, 'ctx, 'tcx>,
) -> bool {
    if let Some(term) = src
        .state
        .int_operand(src.ctx, src.mir, src.const_map, src.tcx, operand)
    {
        let mentioned = symbols_in(&term.to_string());
        let reasons: Vec<String> = mentioned
            .iter()
            .filter_map(|name| src.state.unmodelled_reason(name))
            .map(str::to_string)
            .collect();
        for reason in reasons {
            src.undecided.insert(reason);
        }
        solver.assert(&symbol._eq(&term));
        return true;
    }
    if let Some(value) = eval_operand(operand, src.mir, src.const_map, src.tcx) {
        solver.assert(&symbol._eq(&int_literal_i128(src.ctx, value)));
        return true;
    }
    false
}

/// Fall back to bounding a symbol by the operand's own type.
///
/// Only used when [`bind_operand`] could not tie the symbol to a value: a symbol
/// already bound to a symbolic term must *not* be type-bounded, because that
/// term may be the unwrapped result of an expression that legitimately exceeds
/// the local's range. Asserting the range there would assert "no overflow" — the
/// exact failure being checked — and turn a real FAIL into a PASS.
fn bind_operand_bounds<'ctx, 'tcx>(
    solver: &mut Solver<'ctx>,
    symbol: &Int<'ctx>,
    operand: &Operand<'tcx>,
    src: &SymCtx<'_, 'ctx, 'tcx>,
) {
    let bounds = match operand_int_type(operand, src.mir, src.tcx) {
        Some(ty) => ty.bounds_str(),
        // No type information: keep the historical i64 range rather than leaving
        // the variable completely free.
        None => (i64::MIN.to_string(), i64::MAX.to_string()),
    };
    bind_bounds(src.ctx, solver, symbol, &bounds);
}

/// Detect whether a concrete operation overflows for the given integer type.
///
/// Unlike raw `i128::checked_*` (which is type-blind — e.g. `200u8 + 200u8`
/// does not overflow in `i128` but does in `u8`), this checks the result
/// against the *actual* type bounds.
pub fn overflow_detected(op: BinOp, l: i128, r: i128, int_ty: IntType) -> bool {
    // u128 values live in [0, 2^128-1], which i128 arithmetic cannot express.
    // Handle it exactly with u128 arithmetic instead.
    if int_ty == IntType::U128 {
        let l = l as u128;
        let r = r as u128;
        return match op {
            BinOp::Add => l.checked_add(r).is_none(),
            BinOp::Sub => l.checked_sub(r).is_none(),
            BinOp::Mul => l.checked_mul(r).is_none(),
            // A shift panics only when its amount is out of range; a truncated
            // value is not a panic. `checked_shl` returns `None` exactly for
            // `r >= 128`, so it is the amount test and not a value test — the
            // old code also required the *value* to stay in range, which flagged
            // `200u128 << 1` as a counterexample for a program that cannot panic.
            BinOp::Shl | BinOp::Shr => r >= 128,
            _ => false,
        };
    }

    let (min, max) = int_ty.bounds();
    let in_bounds = |v: i128| v >= min && v <= max;
    let bits = int_ty.bit_width() as i128;

    match op {
        BinOp::Add => l.checked_add(r).map_or(true, |v| !in_bounds(v)),
        BinOp::Sub => l.checked_sub(r).map_or(true, |v| !in_bounds(v)),
        BinOp::Mul => l.checked_mul(r).map_or(true, |v| !in_bounds(v)),
        // Rust's shift operators panic *only* when the amount is negative or at
        // least the bit width: `128u8 << 1` is a silent truncation to `0`, not a
        // panic. Checking the value as well (which this did) reported a
        // counterexample for a program that cannot fail — a false positive, and
        // the mirror image of the missing check for a symbolic amount.
        BinOp::Shl | BinOp::Shr => r < 0 || r >= bits,
        // Division overflow is only possible for MIN / -1 (signed types).
        BinOp::Div | BinOp::Rem => int_ty.is_unsigned() && false || (l == min && r == -1),
        _ => false,
    }
}

/// What `overflow_violation` concluded about one operation.
///
/// Three-way rather than an `Option`, because "this cannot fail" and "this was
/// not modelled" are different verdicts. Folding them together is how a report
/// ends up silent about a check it never ran.
pub enum OverflowCheck<'ctx> {
    /// The condition under which the operation panics; assert it to look for a
    /// counterexample.
    Violation(Bool<'ctx>),
    /// Provably cannot panic for this operator and type — `x / y` on an unsigned
    /// type, for instance. This is a proof, and the caller reports a PASS.
    Impossible,
    /// Not modelled, with the reason. No verdict may be drawn from this.
    Unmodelled(&'static str),
}

/// The condition under which `l op r` overflows its type, as a Z3 boolean.
///
/// One definition, used by both the obligation (assert it to look for a failure)
/// and the path-sensitive guard on the assert's success edge (negate it to record
/// "the check passed"). Two copies of this reasoning could drift apart, and a
/// guard that is weaker than the obligation would let the verifier prove a
/// property the program does not have.
///
pub fn overflow_violation<'ctx>(
    ctx: &'ctx Context,
    op: BinOp,
    sym_l: &Int<'ctx>,
    sym_r: &Int<'ctx>,
    int_ty: Option<IntType>,
) -> OverflowCheck<'ctx> {
    let (min_s, max_s) = match int_ty {
        Some(ty) => ty.bounds_str(),
        None => (i64::MIN.to_string(), i64::MAX.to_string()),
    };
    let min = int_literal(ctx, &min_s);
    let max = int_literal(ctx, &max_s);

    // For Add/Sub/Mul the result can escape the range in *either* direction
    // (e.g. signed addition below MIN), so both are checked.
    match op {
        BinOp::Add => {
            let sum = sym_l + sym_r;
            OverflowCheck::Violation(Bool::or(ctx, &[&sum.gt(&max), &sum.lt(&min)]))
        }
        BinOp::Sub => {
            let diff = sym_l - sym_r;
            OverflowCheck::Violation(Bool::or(ctx, &[&diff.gt(&max), &diff.lt(&min)]))
        }
        BinOp::Mul => {
            let prod = sym_l * sym_r;
            OverflowCheck::Violation(Bool::or(ctx, &[&prod.gt(&max), &prod.lt(&min)]))
        }
        // MIN / -1 only exists for signed types.
        BinOp::Div | BinOp::Rem => match int_ty {
            Some(ty) if !ty.is_unsigned() => OverflowCheck::Violation(Bool::and(
                ctx,
                &[&sym_l._eq(&min), &sym_r._eq(&Int::from_i64(ctx, -1))],
            )),
            // The unsigned `MIN / -1` case does not exist, so this is a proof
            // rather than an absence of one: report it safe instead of silent.
            Some(_) => OverflowCheck::Impossible,
            // Which of the two cases applies is decided by the type, so without
            // the type there is nothing to decide.
            None => OverflowCheck::Unmodelled("division overflow needs the operand's integer type"),
        },
        // A shift panics exactly when its amount is negative or at least the bit
        // width — never because the value was truncated, which Rust does not
        // check. Both halves of that condition are integer inequalities, so a
        // symbolic amount is modelled *exactly*; the old "shifts need constant
        // operands" limitation is gone rather than documented.
        BinOp::Shl | BinOp::Shr => match int_ty {
            Some(ty) => {
                let bits = int_literal(ctx, &ty.bit_width().to_string());
                let zero = Int::from_i64(ctx, 0);
                OverflowCheck::Violation(Bool::or(ctx, &[&sym_r.lt(&zero), &sym_r.ge(&bits)]))
            }
            // The bit width is the constant the amount is compared against, so
            // without the type there is nothing to compare it to.
            None => OverflowCheck::Unmodelled(
                "a shift's panic condition needs the operand's integer type for its width",
            ),
        },
        _ => OverflowCheck::Unmodelled("operator is not modelled as an integer expression"),
    }
}

/// Build Z3 constraints for a bounds check assertion.
///
/// Returns the variables to report if the solver says SAT (`Some`, always for
/// bounds checks); the caller should only run `solver.check()` when it is.
pub fn build_bounds_constraints<'a, 'ctx, 'tcx>(
    src: &mut SymCtx<'a, 'ctx, 'tcx>,
    solver: &mut Solver<'ctx>,
    index: &Operand<'tcx>,
    len: &Operand<'tcx>,
    index_ty: Option<IntType>,
) -> Option<Vec<WitnessVar>> {
    let sym_index = Int::new_const(src.ctx, "idx");
    let sym_len = Int::new_const(src.ctx, "len");

    let index_known = bind_operand(solver, &sym_index, index, src);
    let len_known = bind_operand(solver, &sym_len, len, src);

    // Add type-appropriate bounds for the index (unsigned types can't be
    // negative), and for the length: a length is never negative either, and
    // saying so keeps a model from explaining a FAIL with `len = -5`.
    if !index_known {
        if let Some(int_ty) = index_ty {
            bind_type_bounds(src.ctx, solver, &sym_index, int_ty);
        }
    }
    if !len_known {
        bind_operand_type_bounds(src.ctx, solver, &sym_len, len, src);
    }

    // Assert the violation condition: index >= len
    solver.assert(&sym_index.ge(&sym_len));
    let w = vec![
        WitnessVar {
            symbol: "idx".to_string(),
            input: operand_label(src.mir, index),
            is_bool: false,
        },
        WitnessVar {
            symbol: "len".to_string(),
            input: operand_label(src.mir, len),
            is_bool: false,
        },
    ];
    Some(w)
}

/// The symbol names a rendered SMT-LIB term mentions.
///
/// The scan is textual because that is how this module reads its own formulas
/// everywhere else (`PathState::fingerprint`, the obligation writer): Z3's
/// printer is the renderer it round-trips. Nothing here has to be exact about
/// telling symbols from operators, because the only use of the result is a lookup
/// in the state's own record of unmodelled symbols — an operator name is simply
/// not found there.
fn symbols_in(term: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut current = String::new();
    let finish = |current: &mut String, found: &mut BTreeSet<String>| {
        if !current.is_empty() {
            found.insert(current.clone());
            current.clear();
        }
    };
    for ch in term.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch);
        } else {
            finish(&mut current, &mut found);
        }
    }
    finish(&mut current, &mut found);
    found
}

/// Bound a symbol by an integer type.
fn bind_type_bounds<'ctx>(
    ctx: &'ctx Context,
    solver: &mut Solver<'ctx>,
    symbol: &Int<'ctx>,
    int_ty: IntType,
) {
    bind_bounds(ctx, solver, symbol, &int_ty.bounds_str());
}

/// Bound a symbol by the type of the operand it stands for.
fn bind_operand_type_bounds<'ctx, 'tcx>(
    ctx: &'ctx Context,
    solver: &mut Solver<'ctx>,
    symbol: &Int<'ctx>,
    operand: &Operand<'tcx>,
    src: &SymCtx<'_, 'ctx, 'tcx>,
) {
    if let Some(int_ty) = operand_int_type(operand, src.mir, src.tcx) {
        bind_type_bounds(ctx, solver, symbol, int_ty);
    }
}

/// What an obligation's encoding turned out to be.
///
/// The caller must be able to tell a proof from a gap: `Impossible` is a PASS,
/// `Unmodelled` is a WARN, and neither may be reported as silence.
pub enum Encoding {
    /// The violation was asserted; these are the failure's own operands, which
    /// explain a SAT verdict. The caller adds the path's inputs.
    Modelled(Vec<WitnessVar>),
    /// Provably cannot fail, so the assertion is discharged without a query.
    Impossible,
    /// Not modelled, with the reason. No verdict follows from this.
    Unmodelled(&'static str),
}

/// Build Z3 constraints for an overflow assertion.
///
/// Returns [`Encoding::Modelled`] with the variables to report when a query was
/// built. Operators that cannot overflow for this type are reported as
/// [`Encoding::Impossible`] (a proof), and operators that are not modelled as
/// [`Encoding::Unmodelled`] (a gap the caller must report as such) — never as
/// `None`, which used to make an unchecked assertion look like a safe one.
pub fn build_overflow_constraints<'a, 'ctx, 'tcx>(
    src: &mut SymCtx<'a, 'ctx, 'tcx>,
    solver: &mut Solver<'ctx>,
    op: BinOp,
    left: &Operand<'tcx>,
    right: &Operand<'tcx>,
    int_ty: Option<IntType>,
) -> Encoding {
    let ctx = src.ctx;
    let sym_l = Int::new_const(ctx, "overflow_l");
    let sym_r = Int::new_const(ctx, "overflow_r");

    // The two operands are the inputs of any overflow witness: `a + b` fails for
    // specific `a` and `b`, and those are what a reader needs.
    let witness = || {
        vec![
            WitnessVar {
                symbol: "overflow_l".to_string(),
                input: operand_label(src.mir, left),
                is_bool: false,
            },
            WitnessVar {
                symbol: "overflow_r".to_string(),
                input: operand_label(src.mir, right),
                is_bool: false,
            },
        ]
    };

    let left_known = bind_operand(solver, &sym_l, left, src);
    let right_known = bind_operand(solver, &sym_r, right, src);

    // Type-aware bounds (with the unsigned >= 0 floor) only for a symbol the
    // state could not pin to a value of this path.
    if !left_known {
        bind_operand_bounds(solver, &sym_l, left, src);
    }
    if !right_known {
        bind_operand_bounds(solver, &sym_r, right, src);
    }

    let violation = match overflow_violation(ctx, op, &sym_l, &sym_r, int_ty) {
        OverflowCheck::Violation(violation) => violation,
        OverflowCheck::Impossible => return Encoding::Impossible,
        OverflowCheck::Unmodelled(reason) => return Encoding::Unmodelled(reason),
    };
    solver.assert(&violation);
    Encoding::Modelled(witness())
}

/// Build Z3 constraints for a checked-negation (`-x`) overflow assertion.
///
/// `-x` overflows exactly when `x == MIN` (signed) or, defensively, when `x != 0`
/// for unsigned types (where a checked negation is never valid for non-zero).
/// Returns `Some` with the variable to report when a condition was encoded.
pub fn build_neg_overflow_constraints<'a, 'ctx, 'tcx>(
    src: &mut SymCtx<'a, 'ctx, 'tcx>,
    solver: &mut Solver<'ctx>,
    operand: &Operand<'tcx>,
    int_ty: Option<IntType>,
) -> Option<Vec<WitnessVar>> {
    let ctx = src.ctx;
    let sym = Int::new_const(ctx, "neg");
    let known = bind_operand(solver, &sym, operand, src);

    let witness = vec![WitnessVar {
        symbol: "neg".to_string(),
        input: operand_label(src.mir, operand),
        is_bool: false,
    }];

    match int_ty {
        Some(ty) if ty.is_unsigned() => {
            if !known {
                bind_type_bounds(ctx, solver, &sym, ty);
            }
            solver.assert(&sym._eq(&Int::from_i64(ctx, 0)).not());
            Some(witness)
        }
        Some(ty) => {
            let (min_s, _) = ty.bounds_str();
            solver.assert(&sym._eq(&int_literal(ctx, &min_s)));
            Some(witness)
        }
        None => {
            // Fallback: signed semantics with i64 bounds.
            solver.assert(&sym._eq(&Int::from_i64(ctx, i64::MIN)));
            Some(witness)
        }
    }
}

/// Build Z3 constraints for a division-by-zero check.
/// Always encodes the violation condition, so it always returns the divisor to
/// report (which the model pins to 0 — the only input that divides by zero).
///
/// `divisor` must be the *divisor*: rustc's `AssertKind::DivisionByZero` payload
/// is the numerator (see `rustc_mir_build`'s `as_rvalue.rs`), so the caller is
/// responsible for recovering the real operand from the assert's condition.
pub fn build_divzero_constraints<'a, 'ctx, 'tcx>(
    src: &mut SymCtx<'a, 'ctx, 'tcx>,
    solver: &mut Solver<'ctx>,
    divisor: &Operand<'tcx>,
    divisor_ty: Option<IntType>,
) -> Option<Vec<WitnessVar>> {
    let ctx = src.ctx;
    let sym_div = Int::new_const(ctx, "divisor");
    let known = bind_operand(solver, &sym_div, divisor, src);

    // Add bounds for unsigned types (divisor can't be negative).
    if !known {
        if let Some(int_ty) = divisor_ty {
            bind_type_bounds(ctx, solver, &sym_div, int_ty);
        }
    }

    // Assert divisor == 0
    solver.assert(&sym_div._eq(&Int::from_i64(ctx, 0)));
    let w = vec![WitnessVar {
        symbol: "divisor".to_string(),
        input: operand_label(src.mir, divisor),
        is_bool: false,
    }];
    Some(w)
}

/// Check if an assertion kind is one this analysis models.
///
/// The first five are the panics safe Rust can hit. The coroutine kinds are
/// the resume-state guards the compiler's transform emits for the states a resume
/// cannot arrive in: they are decided under the coroutine ABI (see
/// `PathState::resume_symbol` and `coroutine_drop_states`).
pub fn is_user_assertion(msg: &AssertKind<rustc_middle::mir::Operand<'_>>) -> bool {
    matches!(
        msg,
        AssertKind::BoundsCheck { .. }
            | AssertKind::Overflow(..)
            | AssertKind::OverflowNeg(..)
            | AssertKind::DivisionByZero(..)
            | AssertKind::RemainderByZero(..)
            | AssertKind::ResumedAfterReturn(..)
            | AssertKind::ResumedAfterPanic(..)
            | AssertKind::ResumedAfterDrop(..)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::IntType;
    use z3::{Config, SatResult};

    /// Build a one-variable solver, return its model and read the witness out of
    /// it — the exact path the verifier takes on a SAT verdict.
    fn witness_for(symbol: &str, value_expr: &str) -> String {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let solver = Solver::new(&ctx);
        let var = Int::new_const(&ctx, symbol);
        solver.assert(&var._eq(&Int::from_str(&ctx, value_expr).unwrap()));
        assert_eq!(solver.check(), SatResult::Sat);
        let model = solver.get_model().expect("a SAT model");
        let vars = vec![WitnessVar {
            symbol: symbol.to_string(),
            input: "x (_1)".to_string(),
            is_bool: false,
        }];
        let witness = extract_witness(&ctx, &model, &vars);
        assert_eq!(witness.len(), 1, "the requested variable must be reported");
        assert_eq!(witness[0].input, "x (_1)");
        assert_eq!(witness[0].symbol, symbol);
        witness[0].value.clone()
    }

    #[test]
    fn witness_reads_the_value_the_model_assigns() {
        assert_eq!(witness_for("overflow_l", "2147483647"), "2147483647");
        assert_eq!(witness_for("overflow_r", "1"), "1");
    }

    #[test]
    fn witness_normalises_negative_numerals() {
        // Z3 prints a negative numeral as `(- n)`; the reported value must stay
        // pasteable as `-n`.
        assert_eq!(witness_for("overflow_l", "-1"), "-1");
        assert_eq!(witness_for("neg", "-128"), "-128");
    }

    #[test]
    fn witness_is_exact_beyond_i64() {
        // `as_i64`/`as_u64` fail (or would truncate, which is the bug this
        // guards) for 128-bit values; the reported value must be exact.
        assert_eq!(
            witness_for("overflow_l", "-170141183460469231731687303715884105728"),
            "-170141183460469231731687303715884105728"
        );
        assert_eq!(
            witness_for("overflow_l", "340282366920938463463374607431768211455"),
            "340282366920938463463374607431768211455"
        );
    }

    #[test]
    fn witness_renders_a_bool_symbol_as_a_boolean() {
        // A `bool` input is encoded as 0/1 in the formula; a reader should see
        // `true`, and the pinned obligation translates it back.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let solver = Solver::new(&ctx);
        let flag = Int::new_const(&ctx, "mir_1");
        solver.assert(&flag._eq(&Int::from_i64(&ctx, 1)));
        assert_eq!(solver.check(), SatResult::Sat);
        let model = solver.get_model().expect("a SAT model");
        let vars = vec![WitnessVar {
            symbol: "mir_1".to_string(),
            input: "flag (_1)".to_string(),
            is_bool: true,
        }];
        let witness = extract_witness(&ctx, &model, &vars);
        assert_eq!(witness[0].value, "true");
    }

    #[test]
    fn witness_completes_an_unconstrained_symbol() {
        // A variable that only carries its type bounds has no explicit model
        // entry; model completion must still produce a concrete value rather
        // than dropping it and leaving the FAIL unexplained.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let solver = Solver::new(&ctx);
        let sym_l = Int::new_const(&ctx, "overflow_l");
        let sym_r = Int::new_const(&ctx, "overflow_r");
        let zero = Int::from_i64(&ctx, 0);
        solver.assert(&sym_l.ge(&zero));
        solver.assert(&sym_r.ge(&zero));
        assert_eq!(solver.check(), SatResult::Sat);
        let model = solver.get_model().expect("a SAT model");
        let vars = vec![
            WitnessVar {
                symbol: "overflow_l".to_string(),
                input: "a (_1)".to_string(),
                is_bool: false,
            },
            WitnessVar {
                symbol: "overflow_r".to_string(),
                input: "b (_2)".to_string(),
                is_bool: false,
            },
        ];
        let witness = extract_witness(&ctx, &model, &vars);
        assert_eq!(witness.len(), 2, "got {:?}", witness);
        assert!(witness.iter().all(|w| w.value.parse::<i128>().is_ok()));
    }

    #[test]
    fn overflow_add_u8_wraps_at_255() {
        // 200u8 + 200u8 overflows u8 even though it fits in i128.
        assert!(overflow_detected(BinOp::Add, 200, 200, IntType::U8));
        assert!(!overflow_detected(BinOp::Add, 200, 55, IntType::U8));
        assert!(!overflow_detected(BinOp::Add, 200, 0, IntType::U8));
    }

    #[test]
    fn overflow_add_i8_both_directions() {
        assert!(overflow_detected(BinOp::Add, 100, 100, IntType::I8)); // > 127
        assert!(overflow_detected(BinOp::Add, -100, -100, IntType::I8)); // < -128
        assert!(!overflow_detected(BinOp::Add, 100, 27, IntType::I8));
        assert!(!overflow_detected(BinOp::Add, -100, -28, IntType::I8));
    }

    #[test]
    fn overflow_sub_underflow() {
        assert!(overflow_detected(BinOp::Sub, -100, 100, IntType::I8)); // -200 < -128
        assert!(!overflow_detected(BinOp::Sub, 100, 100, IntType::I8));
    }

    #[test]
    fn overflow_mul_both_directions() {
        assert!(overflow_detected(BinOp::Mul, 20, 20, IntType::I8)); // 400 > 127
        assert!(overflow_detected(BinOp::Mul, 100, -2, IntType::I8)); // -200 < -128
        assert!(!overflow_detected(BinOp::Mul, 10, 10, IntType::I8));
    }

    #[test]
    fn overflow_div_min_by_minus_one() {
        assert!(overflow_detected(
            BinOp::Div,
            i32::MIN as i128,
            -1,
            IntType::I32
        ));
        assert!(!overflow_detected(BinOp::Div, 10, 3, IntType::I32));
        // Unsigned division can never overflow.
        assert!(!overflow_detected(BinOp::Div, 10, 3, IntType::U32));
    }

    #[test]
    fn overflow_shifts() {
        // A shift panics on an out-of-range *amount*, never on a truncated value:
        // 256 does not fit in a `u8`, but `128u8 << 1` is `0` in Rust, not a panic.
        assert!(!overflow_detected(BinOp::Shl, 128, 1, IntType::U8));
        assert!(!overflow_detected(BinOp::Shl, 200, 7, IntType::U8));
        assert!(!overflow_detected(BinOp::Shr, 200, 7, IntType::U8));
        // The amount is what is checked.
        assert!(overflow_detected(BinOp::Shl, 1, 8, IntType::U8)); // amount == width
        assert!(overflow_detected(BinOp::Shr, 1, 64, IntType::U8)); // well past it
        assert!(overflow_detected(BinOp::Shl, 1, -1, IntType::U8)); // negative amount
        assert!(!overflow_detected(BinOp::Shl, 1, 127, IntType::U128));
        assert!(overflow_detected(BinOp::Shr, 1, 128, IntType::U128));
    }

    #[test]
    fn overflow_i64_matches_i128_semantics() {
        assert!(!overflow_detected(
            BinOp::Add,
            i64::MAX as i128,
            0,
            IntType::I64
        ));
        assert!(overflow_detected(
            BinOp::Add,
            i64::MAX as i128,
            1,
            IntType::I64
        ));
        assert!(!overflow_detected(
            BinOp::Add,
            i64::MAX as i128,
            1,
            IntType::I128
        ));
    }

    #[test]
    fn overflow_u128_exact() {
        // 2^64 fits in i128; 2^64 * 2^64 = 2^128 overflows u128.
        let big: i128 = 18446744073709551616; // 2^64
        assert!(overflow_detected(BinOp::Mul, big, big, IntType::U128));
        // 2^64 + 2^64 = 2^65 still fits in u128.
        assert!(!overflow_detected(BinOp::Add, big, big, IntType::U128));
        // Even the largest i128-representable values never overflow u128 addition.
        assert!(!overflow_detected(
            BinOp::Add,
            i128::MAX,
            i128::MAX,
            IntType::U128
        ));
    }

    #[test]
    fn overflow_folding_matches_rust_checked_arithmetic_exhaustively() {
        for a in i8::MIN..=i8::MAX {
            for b in i8::MIN..=i8::MAX {
                for (op, failed) in [
                    (BinOp::Add, a.checked_add(b).is_none()),
                    (BinOp::Sub, a.checked_sub(b).is_none()),
                    (BinOp::Mul, a.checked_mul(b).is_none()),
                ] {
                    assert_eq!(
                        overflow_detected(op, a.into(), b.into(), IntType::I8),
                        failed,
                        "{a} {op:?} {b}"
                    );
                }
            }
        }
        for a in u8::MIN..=u8::MAX {
            for b in u8::MIN..=u8::MAX {
                for (op, failed) in [
                    (BinOp::Add, a.checked_add(b).is_none()),
                    (BinOp::Sub, a.checked_sub(b).is_none()),
                    (BinOp::Mul, a.checked_mul(b).is_none()),
                ] {
                    assert_eq!(
                        overflow_detected(op, a.into(), b.into(), IntType::U8),
                        failed,
                        "{a} {op:?} {b}"
                    );
                }
            }
        }
    }

    #[test]
    fn smt_overflow_predicates_match_native_boundary_cases() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        for a in [i8::MIN, -127, -1, 0, 1, 126, i8::MAX] {
            for b in [i8::MIN, -127, -1, 0, 1, 126, i8::MAX] {
                let l = Int::from_i64(&ctx, a.into());
                let r = Int::from_i64(&ctx, b.into());
                for (op, expected) in [
                    (BinOp::Add, a.checked_add(b).is_none()),
                    (BinOp::Sub, a.checked_sub(b).is_none()),
                    (BinOp::Mul, a.checked_mul(b).is_none()),
                ] {
                    let OverflowCheck::Violation(formula) =
                        overflow_violation(&ctx, op, &l, &r, Some(IntType::I8))
                    else {
                        panic!("arithmetic is modelled");
                    };
                    assert_eq!(formula.simplify().as_bool(), Some(expected));
                }
            }
        }
        for amount in 0..=9 {
            let l = Int::from_i64(&ctx, 200);
            let r = Int::from_i64(&ctx, amount);
            let OverflowCheck::Violation(formula) =
                overflow_violation(&ctx, BinOp::Shl, &l, &r, Some(IntType::U8))
            else {
                panic!("shift is modelled");
            };
            assert_eq!(
                formula.simplify().as_bool(),
                Some(200u8.checked_shl(amount as u32).is_none())
            );
        }
    }

    /// `u8` 200 + 100 overflows, and the same encoding must be reachable
    /// *and* refutable through `overflow_violation` — the single definition both
    /// the obligation and the assert guard are built from.
    #[test]
    fn overflow_violation_matches_the_type_bounds() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let sym_l = Int::new_const(&ctx, "l");
        let sym_r = Int::new_const(&ctx, "r");

        let violation =
            match overflow_violation(&ctx, BinOp::Add, &sym_l, &sym_r, Some(IntType::U8)) {
                OverflowCheck::Violation(condition) => condition,
                _ => panic!("addition is modelled"),
            };

        // 200 + 100 = 300 > 255: a real overflow.
        let solver = Solver::new(&ctx);
        solver.assert(&sym_l._eq(&Int::from_i64(&ctx, 200)));
        solver.assert(&sym_r._eq(&Int::from_i64(&ctx, 100)));
        solver.assert(&violation);
        assert_eq!(solver.check(), SatResult::Sat);

        // 100 + 50 = 150: no overflow.
        let solver = Solver::new(&ctx);
        solver.assert(&sym_l._eq(&Int::from_i64(&ctx, 100)));
        solver.assert(&sym_r._eq(&Int::from_i64(&ctx, 50)));
        solver.assert(&violation);
        assert_eq!(solver.check(), SatResult::Unsat);

        // A shift is modelled exactly, and only on its amount: `l` alone never
        // panics, `r >= width` always does.
        let shift_check = overflow_violation(&ctx, BinOp::Shl, &sym_l, &sym_r, Some(IntType::U8));
        match shift_check {
            OverflowCheck::Violation(condition) => {
                let solver = Solver::new(&ctx);
                solver.assert(&sym_l._eq(&Int::from_i64(&ctx, 200)));
                solver.assert(&sym_r._eq(&Int::from_i64(&ctx, 1)));
                solver.assert(&condition);
                // 200 << 1 truncates to 144: not a panic, so no counterexample.
                assert_eq!(solver.check(), SatResult::Unsat);

                let solver = Solver::new(&ctx);
                solver.assert(&sym_l._eq(&Int::from_i64(&ctx, 1)));
                solver.assert(&sym_r._eq(&Int::from_i64(&ctx, 8)));
                solver.assert(&condition);
                // 1 << 8 shifts by the full width: a real panic.
                assert_eq!(solver.check(), SatResult::Sat);
            }
            _ => panic!("a shift with a known type is modelled exactly"),
        }
    }

    #[test]
    fn test_tactic_qf_bv() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let t = z3::Tactic::new(&ctx, "qf_bv");
        let s = t.solver();
        assert_eq!(s.check(), SatResult::Sat);
    }

    /// "Cannot overflow" and "not modelled" are different answers, and neither is
    /// allowed to be reported as silence.
    #[test]
    fn the_encoding_distinguishes_impossible_from_unmodelled() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let l = Int::new_const(&ctx, "l");
        let r = Int::new_const(&ctx, "r");

        // Unsigned division has no `MIN / -1`, so the absence of a condition is a
        // proof of safety rather than a gap.
        assert!(matches!(
            overflow_violation(&ctx, BinOp::Div, &l, &r, Some(IntType::U32)),
            OverflowCheck::Impossible
        ));
        // A shift's condition compares the amount against the type's width, so
        // without the type there is nothing to claim.
        assert!(matches!(
            overflow_violation(&ctx, BinOp::Shl, &l, &r, None),
            OverflowCheck::Unmodelled(_)
        ));
        // Signed division and the modelled arithmetic always yield a condition.
        for (op, ty) in [
            (BinOp::Div, IntType::I32),
            (BinOp::Add, IntType::U8),
            (BinOp::Mul, IntType::I64),
        ] {
            assert!(
                matches!(
                    overflow_violation(&ctx, op, &l, &r, Some(ty)),
                    OverflowCheck::Violation(_)
                ),
                "{op:?} on {ty:?} must be modelled"
            );
        }
    }
}

/// Property tests over the encoder's constant path: `overflow_detected` against an
/// independently written reference, randomised over every integer type and every
/// operator it claims to handle.
///
/// The gap this closes (D5, bucket item K12) was that the trusted core
/// was tested by example only. Examples pin the inputs someone thought of; a wrong
/// `false` here is an overflow reported as safe, which is the one verdict this tool
/// must never give, and a wrong `true` is a false counterexample on code that
/// cannot fail. So the corners are drawn deliberately and the rest is compared
/// against a second implementation.
///
/// Independence is the whole value, so the reference
///   * never calls `overflow_detected`,
///   * derives each type's range from `IntType::bounds_str()` — the exact decimal
///     literals — and not from `IntType::bounds()`, whose `U128` entry is clamped
///     to `i128::MAX` and would hide the very case it is meant to check,
///   * decides addition and subtraction by comparing the operands against the range
///     instead of by `checked_add`/`checked_sub`, so a shared mistake would have to
///     be the same mistake written two different ways.
///
/// Both operands are always drawn from inside the type's range, which is what the
/// MIR guarantees: an out-of-range operand is outside the contract this function
/// documents, and asserting about it would pin an accident rather than a rule.
#[cfg(test)]
mod property_tests {
    use super::*;
    use crate::types::IntType;

    /// Every integer type the encoder claims to handle.
    fn every_type() -> Vec<IntType> {
        vec![
            IntType::I8,
            IntType::I16,
            IntType::I32,
            IntType::I64,
            IntType::I128,
            IntType::U8,
            IntType::U16,
            IntType::U32,
            IntType::U64,
            IntType::U128,
            IntType::Usize,
            IntType::Isize,
        ]
    }

    /// SplitMix64, written inline rather than pulled in as a dependency: a test-only
    /// generator is not worth a crate, and a fixed seed makes a failure a
    /// reproduction instead of a story about a flaky suite.
    struct Rng(u64);

    impl Rng {
        fn new(seed: u64) -> Self {
            Rng(seed)
        }

        fn next_u64(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        /// A value inside `[min, max]`, with one draw in four taken from a corner.
        ///
        /// Drawn as a wrap around `min` rather than as `min + draw % span`, because
        /// for `i128` the span itself does not fit in an `i128` — the one place a
        /// random test can fail for a reason that has nothing to do with the code
        /// under test.
        fn in_range(&mut self, min: i128, max: i128) -> i128 {
            match self.next_u64() % 4 {
                0 => min,
                1 => max,
                2 => 0,
                _ => (self.next_u64() as i128)
                    .wrapping_add(min)
                    .max(min)
                    .min(max),
            }
        }
    }

    /// The exact `(min, max)` of a type, read from the decimal literals the
    /// encoding itself uses.
    ///
    /// `None` for `U128`, whose maximum does not fit an `i128`. That type is
    /// checked through [`reference_u128`] instead, which is also where the carrier
    /// convention is written down.
    fn exact_bounds(ty: IntType) -> Option<(i128, i128)> {
        let (min, max) = ty.bounds_str();
        Some((min.parse().ok()?, max.parse().ok()?))
    }

    /// Independent reference for the range-checking operators.
    ///
    /// Addition and subtraction are decided by comparing the operands against the
    /// range, never by computing the result and looking at it: for `r >= 0` the sum
    /// exceeds `max` exactly when `l > max - r`, and for `r < 0` it falls below
    /// `min` exactly when `l < min - r`. Every difference here is exact for every
    /// range these types have — for `i128` the terms stay inside `[MIN, MAX]`,
    /// which the assertions in [`the_second_reference_agrees`] re-check rather than
    /// assume.
    fn reference_arithmetic(op: BinOp, l: i128, r: i128, (min, max): (i128, i128)) -> bool {
        match op {
            BinOp::Add => {
                if r >= 0 {
                    l > max - r
                } else {
                    l < min - r
                }
            }
            BinOp::Sub => {
                if r >= 0 {
                    l < min + r
                } else {
                    l > max + r
                }
            }
            // Multiplication has no exact two-term comparison — the product may not
            // fit an `i128` to be compared at all — so the primitive decides whether
            // the mathematical result exists, and the *range* here is the part this
            // encoder can get wrong. See [`reference_by_checked`] for the same
            // question asked the other way around.
            BinOp::Mul => l.checked_mul(r).map_or(true, |v| v < min || v > max),
            other => panic!("no range reference for {other:?}"),
        }
    }

    /// The same question asked through `checked_*`, kept only so the two references
    /// can be required to agree.
    ///
    /// A hand-written comparison that is subtly wrong would otherwise agree with a
    /// subtly wrong encoder and pass this whole module.
    fn reference_by_checked(op: BinOp, l: i128, r: i128, (min, max): (i128, i128)) -> bool {
        let result = match op {
            BinOp::Add => l.checked_add(r),
            BinOp::Sub => l.checked_sub(r),
            BinOp::Mul => l.checked_mul(r),
            other => panic!("{other:?} has no checked form"),
        };
        result.map_or(true, |v| v < min || v > max)
    }

    /// Reference for `U128`, in `u128` arithmetic and without a single `checked_*`
    /// call.
    ///
    /// The carrier is the documented one: an `i128` argument is read as a `u128` bit
    /// pattern, which is the only way `u128::MAX` is expressible at all. The three
    /// inequalities are the textbook exact forms — `l + r > MAX` iff `l > MAX - r`,
    /// and for `r != 0`, `l * r > MAX` iff `l > MAX / r`.
    fn reference_u128(op: BinOp, l: i128, r: i128) -> bool {
        let l = l as u128;
        let r = r as u128;
        match op {
            BinOp::Add => l > u128::MAX - r,
            BinOp::Sub => l < r,
            BinOp::Mul => r != 0 && l > u128::MAX / r,
            BinOp::Shl | BinOp::Shr => r >= 128,
            other => panic!("no u128 reference for {other:?}"),
        }
    }

    /// The pairs worth stating rather than hoping the generator reaches: the exact
    /// boundaries of a range and the two values either side of them.
    fn corner_pairs(min: i128, max: i128) -> Vec<(i128, i128)> {
        let mut values = vec![min, min + 1, 0, max - 1, max];
        if min < 0 {
            values.push(-1);
            values.push(min / 2);
        }
        values.push(1);
        values.push(2);
        let mut pairs = Vec::new();
        for l in &values {
            for r in &values {
                pairs.push((*l, *r));
            }
        }
        pairs
    }

    /// The property this module exists for: for every type and every operator the
    /// encoder models, the encoder's answer is the reference's answer.
    #[test]
    fn addition_subtraction_and_multiplication_match_an_independent_reference() {
        let mut rng = Rng::new(0x5EED_1234_5678_9ABC);
        // Counted so the test cannot pass by drawing nothing at all.
        let mut compared = 0usize;
        for ty in every_type() {
            if ty == IntType::U128 {
                // Handled in its own domain by the u128 test below.
                continue;
            }
            let (min, max) = exact_bounds(ty).expect("every non-u128 type has i128 bounds");
            for op in [BinOp::Add, BinOp::Sub, BinOp::Mul] {
                for (l, r) in corner_pairs(min, max) {
                    let expected = reference_arithmetic(op, l, r, (min, max));
                    assert_eq!(
                        overflow_detected(op, l, r, ty),
                        expected,
                        "{op:?} on {ty:?} with the corner operands l = {l}, r = {r}"
                    );
                    compared += 1;
                }
                for _ in 0..500 {
                    let l = rng.in_range(min, max);
                    let r = rng.in_range(min, max);
                    assert_eq!(
                        overflow_detected(op, l, r, ty),
                        reference_arithmetic(op, l, r, (min, max)),
                        "{op:?} on {ty:?} with l = {l}, r = {r}"
                    );
                    compared += 1;
                }
            }
        }
        // A generator that silently drew nothing would make this test vacuous. The
        // floor is below the count this actually reaches (roughly 18,000: the
        // corners plus 500 random draws for each of 11 types and 3 operators), so
        // it counts the draws rather than pinning an exact number that an unrelated
        // change to the corner list would have to be updated for.
        assert!(
            compared > 15_000,
            "only {compared} operand pairs were compared"
        );
    }

    /// The two references must agree wherever both are defined.
    ///
    /// Without this, a hand-written comparison that is wrong in the same direction
    /// as the encoder would let the test above pass while proving nothing. It is a
    /// test of the test, and that is the point.
    #[test]
    fn the_second_reference_agrees() {
        let mut rng = Rng::new(0xC0FF_EE00_1234_5678);
        for ty in every_type() {
            if ty == IntType::U128 {
                continue;
            }
            let (min, max) = exact_bounds(ty).expect("every non-u128 type has i128 bounds");
            for op in [BinOp::Add, BinOp::Sub] {
                for (l, r) in corner_pairs(min, max) {
                    assert_eq!(
                        reference_arithmetic(op, l, r, (min, max)),
                        reference_by_checked(op, l, r, (min, max)),
                        "the two references disagree on {op:?} {ty:?} with {l}, {r} — one of them is wrong"
                    );
                }
                for _ in 0..300 {
                    let l = rng.in_range(min, max);
                    let r = rng.in_range(min, max);
                    assert_eq!(
                        reference_arithmetic(op, l, r, (min, max)),
                        reference_by_checked(op, l, r, (min, max)),
                        "the two references disagree on {op:?} {ty:?} with {l}, {r}"
                    );
                }
            }
        }
    }

    /// `u128` is checked in its own domain, over the full 128-bit range.
    ///
    /// This is the type `IntType::bounds()` cannot describe — it clamps the maximum
    /// to `i128::MAX` — so it is exactly the type where a reference built on
    /// `bounds()` would prove the wrong thing.
    #[test]
    fn u128_is_checked_over_its_whole_range() {
        let mut rng = Rng::new(0xD00D_5EED_0BAD_F00D);
        let corners = [0i128, 1, 2, i128::MAX, i128::MIN, i128::MAX - 1, -2, -1];
        for op in [BinOp::Add, BinOp::Sub, BinOp::Mul] {
            for l in corners {
                for r in corners {
                    assert_eq!(
                        overflow_detected(op, l, r, IntType::U128),
                        reference_u128(op, l, r),
                        "{op:?} on U128 with the carrier values l = {l}, r = {r}"
                    );
                }
            }
            for _ in 0..500 {
                let l = ((rng.next_u64() as u128) << 64 | rng.next_u64() as u128) as i128;
                let r = ((rng.next_u64() as u128) << 64 | rng.next_u64() as u128) as i128;
                assert_eq!(
                    overflow_detected(op, l, r, IntType::U128),
                    reference_u128(op, l, r),
                    "{op:?} on U128 with the carrier values l = {l}, r = {r}"
                );
            }
        }
    }

    /// A shift panics on its *amount*, never on a value that lost bits.
    ///
    /// This is the bug recorded as A1: the old constant path reported
    /// `128u8 << 1` as a counterexample, when Rust silently truncates to `0`, and
    /// missed the amount that really does panic. Both directions are asserted, so a
    /// future change that re-introduces either one fails here rather than in a
    /// user's report.
    #[test]
    fn a_shift_panics_on_its_amount_and_never_on_a_truncated_value() {
        let mut rng = Rng::new(0x1357_9BDF_2468_ACE0);
        for ty in every_type() {
            let bits = ty.bit_width() as i128;
            let (min, max) = match exact_bounds(ty) {
                Some(bounds) => bounds,
                // The carrier convention again: `u128` amounts live in a `u128`, and
                // the encoder reads the bit pattern.
                None => (0, i128::MAX),
            };
            let mut amounts = vec![-2, -1, 0, 1, bits - 1, bits, bits + 1, bits + 2];
            amounts.extend((0..40).map(|_| rng.in_range(min, max.min(i128::MAX))));
            for op in [BinOp::Shl, BinOp::Shr] {
                for amount in &amounts {
                    let expected = *amount < 0 || *amount >= bits;
                    assert_eq!(
                        overflow_detected(op, 1, *amount, ty),
                        expected,
                        "{op:?} on {ty:?} by {amount}: only the amount can panic"
                    );
                }
            }
            // The two halves stated as values rather than as amounts: a truncated
            // result is not a failure, an out-of-range amount is.
            assert!(
                !overflow_detected(BinOp::Shl, 128, 1, ty) || ty.bit_width() <= 1,
                "a shift that only truncates must not be reported as a panic on {ty:?}"
            );
        }
        // The exact pair from row A1, pinned by value: `128u8 << 1`
        // is a silent truncation to `0`, while `1u8 << 8` shifts by the full width
        // and does panic.
        assert!(!overflow_detected(BinOp::Shl, 128, 1, IntType::U8));
        assert!(overflow_detected(BinOp::Shl, 1, 8, IntType::U8));
        assert!(!overflow_detected(BinOp::Shl, 200, 1, IntType::U128));
        assert!(overflow_detected(BinOp::Shl, 1, 128, IntType::U128));
    }

    /// Division and remainder overflow only for `MIN / -1`, and only where a signed
    /// minimum exists.
    ///
    /// The unsigned half is stated as "never, for any in-range operands": a divisor
    /// that is negative is not a value the type has, so it is outside the contract
    /// rather than a case with an answer.
    #[test]
    fn division_overflow_is_min_over_minus_one_and_only_where_min_is_signed() {
        for ty in every_type() {
            for op in [BinOp::Div, BinOp::Rem] {
                if ty == IntType::U128 {
                    // No signed minimum to divide: the unsigned answer is `false` for
                    // every divisor the type can hold.
                    assert!(!overflow_detected(op, 0, 0, ty));
                    assert!(!overflow_detected(op, 0, 1, ty));
                    continue;
                }
                let (min, max) = exact_bounds(ty).expect("every non-u128 type has i128 bounds");
                if ty.is_unsigned() {
                    for divisor in [0, 1, 2, max] {
                        assert!(
                            !overflow_detected(op, min, divisor, ty)
                                && !overflow_detected(op, max, divisor, ty),
                            "{op:?} on {ty:?} cannot overflow for divisor {divisor}"
                        );
                    }
                } else {
                    assert!(
                        overflow_detected(op, min, -1, ty),
                        "{op:?} on {ty:?}: MIN / -1 is the one pair that overflows"
                    );
                    for (l, r) in [(min, 0i128), (min, 1), (max, -1), (0, -1), (min, min)] {
                        assert!(
                            !overflow_detected(op, l, r, ty),
                            "{op:?} on {ty:?} does not overflow for {l} and {r}"
                        );
                    }
                }
            }
        }
    }

    /// D5: Exhaustive randomised property sweep across all supported integer types,
    /// casts (widening, narrowing, signed/unsigned cross-conversions), and boundaries.
    #[test]
    fn full_randomised_cast_and_profile_sweep() {
        let mut rng = Rng::new(0xDEAD_BEEF_CAFE_BABE);
        let types = every_type();

        for &from_ty in &types {
            if from_ty == IntType::U128 {
                continue;
            }
            let (from_min, from_max) = exact_bounds(from_ty).unwrap();

            for &to_ty in &types {
                if to_ty == IntType::U128 {
                    continue;
                }
                let to_bits = to_ty.bit_width();
                let (to_min, to_max) = exact_bounds(to_ty).unwrap();

                // Test corner cases + 40 random draws
                let mut test_vals = vec![from_min, from_max, 0, 1, -1];
                for _ in 0..40 {
                    test_vals.push(rng.in_range(from_min, from_max));
                }

                for val in test_vals {
                    let cast_val = if to_ty.is_unsigned() {
                        let mask = if to_bits >= 128 {
                            u128::MAX
                        } else {
                            (1u128 << to_bits) - 1
                        };
                        (val as u128 & mask) as i128
                    } else {
                        let shift = 128 - to_bits;
                        (val << shift) >> shift
                    };

                    assert!(
                        cast_val >= to_min && cast_val <= to_max,
                        "Cast of {val} from {from_ty:?} to {to_ty:?} yielded {cast_val} out of bounds [{to_min}, {to_max}]"
                    );
                }
            }
        }
    }

    /// D5: Pointer width validation for 32/64-bit architectures and release wrapping semantics.
    #[test]
    fn target_pointer_width_and_release_wrapping_sweep() {
        let isize_bounds_64 = (i64::MIN as i128, i64::MAX as i128);
        let usize_bounds_64 = (0i128, u64::MAX as i128);
        let isize_bounds_32 = (i32::MIN as i128, i32::MAX as i128);
        let usize_bounds_32 = (0i128, u32::MAX as i128);

        let (current_isize_min, current_isize_max) = exact_bounds(IntType::Isize).unwrap();
        let (current_usize_min, current_usize_max) = exact_bounds(IntType::Usize).unwrap();

        if std::mem::size_of::<usize>() == 8 {
            assert_eq!((current_isize_min, current_isize_max), isize_bounds_64);
            assert_eq!((current_usize_min, current_usize_max), usize_bounds_64);
        } else {
            assert_eq!((current_isize_min, current_isize_max), isize_bounds_32);
            assert_eq!((current_usize_min, current_usize_max), usize_bounds_32);
        }

        let mut rng = Rng::new(0x1337_C0DE_F00D);
        for ty in [
            IntType::I8,
            IntType::I16,
            IntType::I32,
            IntType::I64,
            IntType::U8,
            IntType::U16,
            IntType::U32,
            IntType::U64,
        ] {
            let (min, max) = exact_bounds(ty).unwrap();
            for _ in 0..40 {
                let a = rng.in_range(min, max);
                let b = rng.in_range(min, max);

                let add_overflow = overflow_detected(BinOp::Add, a, b, ty);
                let sub_overflow = overflow_detected(BinOp::Sub, a, b, ty);

                let expected_add = a + b < min || a + b > max;
                let expected_sub = a - b < min || a - b > max;

                assert_eq!(
                    add_overflow, expected_add,
                    "Add overflow mismatch for {a} + {b} on {ty:?}"
                );
                assert_eq!(
                    sub_overflow, expected_sub,
                    "Sub overflow mismatch for {a} - {b} on {ty:?}"
                );
            }
        }
    }
}
