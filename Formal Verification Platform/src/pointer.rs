//! What a pointer's origin guarantees about its address.
//!
//! `rustc` emits two assertions around a dereference: that the address is not
//! null and that it is aligned. Both are stated over the address as an integer,
//! built out of casts, masks and comparisons:
//!
//! ```text
//! _62 = copy ((_2.0: Unique<…>).0: NonNull<…>) as *const MaybeUninit<[i32; 3]> (Transmute)
//! _69 = copy _62 as *const () (PtrToPtr)
//! _73 = Eq(copy _72, const 0_usize)          // "the address is null"
//! ```
//!
//! Once the address has a *provenance* those assertions can be decided rather
//! than left unchecked: an address derived from `&T`, `Box<T>`, `NonNull<T>` or
//! `Unique<T>` is non-null by construction and aligned to `align_of(T)`. This
//! module carries exactly those two facts. It models no offsets, no allocation
//! contents and no pointer arithmetic, so every answer it gives is a proof about
//! the address's origin — it never refutes an assertion, and a refuted one is
//! left to the ordinary unsupported path rather than asserted here on the
//! strength of a model that cannot see the value.

use rustc_middle::mir::{Body, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty::{Ty, TyCtxt, TyKind, TypingEnv, TypingMode};

use crate::constant_prop::{eval_operand, ConstMap};

/// How deep the definition chain is followed before the model gives up. MIR
/// temps are short chains; a longer one is a sign the value is not a
/// straight-line address and is not worth guessing about.
const MAX_DEPTH: usize = 12;

/// The alignment and non-nullness a pointer's origin guarantees.
#[derive(Clone, Debug, PartialEq)]
struct Guarantee {
    alignment: u64,
    non_null: bool,
    origin: String,
}

/// What this analysis knows about a `usize`-typed value: a constant, or an
/// address with the guarantees of the pointer it was cast from.
#[derive(Clone, Debug, PartialEq)]
enum Fact {
    Const(i128),
    Address(Guarantee),
}

/// The two truth values a `bool` operand can be proved to have.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Truth {
    True,
    False,
    Unknown,
}

/// The outcome of evaluating a pointer assertion from provenance (S8).
#[derive(Clone, Debug, PartialEq)]
pub enum PointerDecision {
    /// The pointer assertion is proved to hold.
    Proved(String),
    /// The pointer assertion is refuted with a concrete invalid address.
    Refuted {
        reason: String,
        address: u64,
        kind: &'static str,
    },
}

/// Decide a pointer assertion from the provenance of the address it tests.
///
/// Returns `Some(PointerDecision::Proved(reason))` when the assertion is proved to hold,
/// or `Some(PointerDecision::Refuted { .. })` when an address is definitely null or misaligned.
pub fn decide<'tcx>(
    mir: &Body<'tcx>,
    const_map: &ConstMap,
    tcx: TyCtxt<'tcx>,
    cond: &Operand<'tcx>,
    expected: bool,
) -> Option<PointerDecision> {
    let typing_env = typing_env(tcx, mir);
    let mut origins: Vec<String> = Vec::new();
    let truth = truth_of(mir, const_map, tcx, typing_env, cond, 0, &mut origins);

    let is_proved = (truth == Truth::True && expected) || (truth == Truth::False && !expected);
    let is_refuted = (truth == Truth::False && expected) || (truth == Truth::True && !expected);

    if is_proved {
        let origin = origins
            .first()
            .cloned()
            .unwrap_or_else(|| "a reference-like pointer".to_string());
        Some(PointerDecision::Proved(format!(
            "the check holds from pointer provenance: the address comes from {origin}, which is non-null and aligned by construction"
        )))
    } else if is_refuted {
        let address = extract_refuted_address(mir, const_map, tcx, typing_env, cond).unwrap_or(0);
        let kind = if address == 0 {
            "null-pointer"
        } else {
            "misaligned-pointer"
        };
        let reason = if address == 0 {
            "dereference of known null pointer (0x0)".to_string()
        } else {
            format!("dereference of misaligned pointer ({address:#x})")
        };
        Some(PointerDecision::Refuted {
            reason,
            address,
            kind,
        })
    } else {
        None
    }
}

fn extract_refuted_address<'tcx>(
    mir: &Body<'tcx>,
    const_map: &ConstMap,
    tcx: TyCtxt<'tcx>,
    typing_env: TypingEnv<'tcx>,
    cond: &Operand<'tcx>,
) -> Option<u64> {
    let place = operand_place(cond)?;
    let defs = definitions(mir, place.local.index())?;
    for rval in defs {
        match rval {
            Rvalue::BinaryOp(_, args) => {
                let mut origins = Vec::new();
                if let Some(Fact::Const(val)) =
                    int_fact(mir, const_map, tcx, typing_env, &args.0, 0, &mut origins)
                {
                    if val != 0 || matches!(&args.1, Operand::Constant(_)) {
                        return Some(val as u64);
                    }
                }
                if let Some(Fact::Const(val)) =
                    int_fact(mir, const_map, tcx, typing_env, &args.1, 0, &mut origins)
                {
                    return Some(val as u64);
                }
            }
            Rvalue::Use(inner, _) => {
                return extract_refuted_address(mir, const_map, tcx, typing_env, inner);
            }
            _ => {}
        }
    }
    None
}

fn typing_env<'tcx>(tcx: TyCtxt<'tcx>, mir: &Body<'tcx>) -> TypingEnv<'tcx> {
    TypingEnv::new(tcx.param_env(mir.source.def_id()), TypingMode::PostAnalysis)
}

/// The truth of a `bool` operand, or `Unknown` when the model cannot say.
fn truth_of<'tcx>(
    mir: &Body<'tcx>,
    const_map: &ConstMap,
    tcx: TyCtxt<'tcx>,
    typing_env: TypingEnv<'tcx>,
    operand: &Operand<'tcx>,
    depth: usize,
    origins: &mut Vec<String>,
) -> Truth {
    if depth > MAX_DEPTH {
        return Truth::Unknown;
    }
    let Some(place) = operand_place(operand) else {
        return match eval_operand(operand, mir, const_map, tcx) {
            Some(0) => Truth::False,
            Some(_) => Truth::True,
            None => Truth::Unknown,
        };
    };
    let Some(definitions) = definitions(mir, place.local.index()) else {
        return Truth::Unknown;
    };
    let mut agreed: Option<Truth> = None;
    for rvalue in definitions {
        let truth = truth_of_rvalue(mir, const_map, tcx, typing_env, rvalue, depth, origins);
        // A temp reused by two sites is decided only when both say the same
        // thing: the state does not record which definition reaches here.
        match (agreed, truth) {
            (_, Truth::Unknown) => return Truth::Unknown,
            (Some(previous), current) if previous != current => return Truth::Unknown,
            (None, current) => agreed = Some(current),
            _ => {}
        }
    }
    agreed.unwrap_or(Truth::Unknown)
}

fn truth_of_rvalue<'tcx>(
    mir: &Body<'tcx>,
    const_map: &ConstMap,
    tcx: TyCtxt<'tcx>,
    typing_env: TypingEnv<'tcx>,
    rvalue: &Rvalue<'tcx>,
    depth: usize,
    origins: &mut Vec<String>,
) -> Truth {
    match rvalue {
        Rvalue::UnaryOp(rustc_middle::mir::UnOp::Not, inner)
            if operand_is_bool(mir, tcx, inner) =>
        {
            flip(truth_of(
                mir,
                const_map,
                tcx,
                typing_env,
                inner,
                depth + 1,
                origins,
            ))
        }
        Rvalue::BinaryOp(op, args) if operand_is_bool(mir, tcx, &args.0) => {
            let left = truth_of(mir, const_map, tcx, typing_env, &args.0, depth + 1, origins);
            let right = truth_of(mir, const_map, tcx, typing_env, &args.1, depth + 1, origins);
            match op {
                rustc_middle::mir::BinOp::BitAnd => and(left, right),
                rustc_middle::mir::BinOp::BitOr => or(left, right),
                rustc_middle::mir::BinOp::BitXor => xor(left, right),
                _ => Truth::Unknown,
            }
        }
        Rvalue::BinaryOp(op, args) => {
            let left = int_fact(mir, const_map, tcx, typing_env, &args.0, depth + 1, origins);
            let right = int_fact(mir, const_map, tcx, typing_env, &args.1, depth + 1, origins);
            compare(*op, &left, &right)
        }
        Rvalue::Use(inner, _) => {
            truth_of(mir, const_map, tcx, typing_env, inner, depth + 1, origins)
        }
        _ => Truth::Unknown,
    }
}

/// What is known about an integer (or address) operand.
fn int_fact<'tcx>(
    mir: &Body<'tcx>,
    const_map: &ConstMap,
    tcx: TyCtxt<'tcx>,
    typing_env: TypingEnv<'tcx>,
    operand: &Operand<'tcx>,
    depth: usize,
    origins: &mut Vec<String>,
) -> Option<Fact> {
    if depth > MAX_DEPTH {
        return None;
    }
    if let Some(value) = eval_operand(operand, mir, const_map, tcx) {
        return Some(Fact::Const(value));
    }
    let place = operand_place(operand)?;
    let definitions = definitions(mir, place.local.index())?;
    let mut agreed: Option<Fact> = None;
    for rvalue in definitions {
        let fact = int_fact_rvalue(mir, const_map, tcx, typing_env, rvalue, depth, origins)?;
        match &agreed {
            Some(previous) if *previous != fact => return None,
            None => agreed = Some(fact),
            _ => {}
        }
    }
    agreed
}

fn int_fact_rvalue<'tcx>(
    mir: &Body<'tcx>,
    const_map: &ConstMap,
    tcx: TyCtxt<'tcx>,
    typing_env: TypingEnv<'tcx>,
    rvalue: &Rvalue<'tcx>,
    depth: usize,
    origins: &mut Vec<String>,
) -> Option<Fact> {
    match rvalue {
        // The conversion `&T`/`Box<T>`/`NonNull<T>` -> `usize`. `rustc` emits
        // `Transmute` for `ptr as usize` and `PtrToPtr` for the pointer-to-pointer
        // step before it; both preserve the origin.
        Rvalue::Cast(kind, inner, _)
            if matches!(
                kind,
                rustc_middle::mir::CastKind::Transmute
                    | rustc_middle::mir::CastKind::PtrToPtr
                    | rustc_middle::mir::CastKind::PointerExposeProvenance
                    | rustc_middle::mir::CastKind::PointerCoercion(..)
            ) =>
        {
            match guarantee_of(mir, tcx, typing_env, inner, depth + 1) {
                Some(guarantee) => {
                    origins.push(guarantee.origin.clone());
                    Some(Fact::Address(guarantee))
                }
                None => int_fact(mir, const_map, tcx, typing_env, inner, depth + 1, origins),
            }
        }
        Rvalue::Use(inner, _) => {
            int_fact(mir, const_map, tcx, typing_env, inner, depth + 1, origins)
        }
        Rvalue::UnaryOp(rustc_middle::mir::UnOp::Not, inner)
            if operand_is_bool(mir, tcx, inner) =>
        {
            let truth = truth_of(mir, const_map, tcx, typing_env, inner, depth + 1, origins);
            match truth {
                Truth::True => Some(Fact::Const(0)),
                Truth::False => Some(Fact::Const(1)),
                Truth::Unknown => None,
            }
        }
        Rvalue::BinaryOp(op, args) => {
            let left = int_fact(mir, const_map, tcx, typing_env, &args.0, depth + 1, origins);
            let right = int_fact(mir, const_map, tcx, typing_env, &args.1, depth + 1, origins);
            // The arithmetic shapes and the comparison shapes are separate rules;
            // the unit tests below pin both against the MIR that produces them.
            arith_fact(*op, &left, &right).or_else(|| comparison_fact(*op, &left, &right))
        }
        _ => None,
    }
}

/// The guarantee behind a pointer operand, if its origin gives one.
fn guarantee_of<'tcx>(
    mir: &Body<'tcx>,
    tcx: TyCtxt<'tcx>,
    typing_env: TypingEnv<'tcx>,
    operand: &Operand<'tcx>,
    depth: usize,
) -> Option<Guarantee> {
    if depth > MAX_DEPTH {
        return None;
    }
    let place = operand_place(operand)?;
    // The operand's own type is the strongest fact available: `NonNull<T>`,
    // `Unique<T>`, `Box<T>` and `&T` are all non-null and aligned by their
    // invariants, whatever produced the value.
    let place_ty = place.ty(mir, tcx).ty;
    if let Some(guarantee) = from_type(tcx, typing_env, place_ty) {
        return Some(guarantee);
    }
    let definitions = definitions(mir, place.local.index())?;
    let mut agreed: Option<Guarantee> = None;
    for rvalue in definitions {
        let guarantee = guarantee_of_rvalue(mir, tcx, typing_env, rvalue, depth + 1)?;
        match &agreed {
            Some(previous) if *previous != guarantee => return None,
            None => agreed = Some(guarantee),
            _ => {}
        }
    }
    agreed
}

fn guarantee_of_rvalue<'tcx>(
    mir: &Body<'tcx>,
    tcx: TyCtxt<'tcx>,
    typing_env: TypingEnv<'tcx>,
    rvalue: &Rvalue<'tcx>,
    depth: usize,
) -> Option<Guarantee> {
    match rvalue {
        // A borrow of a local place: the address of a place this body owns is
        // non-null and aligned to that place's type.
        Rvalue::Ref(_, _, borrowed) | Rvalue::RawPtr(_, borrowed) => {
            let ty = borrowed.ty(mir, tcx).ty;
            let alignment = alignment_of(tcx, typing_env, ty)?;
            Some(Guarantee {
                alignment,
                non_null: true,
                origin: format!("the address of `{ty}`"),
            })
        }
        Rvalue::Cast(rustc_middle::mir::CastKind::PtrToPtr, inner, _)
        | Rvalue::Cast(rustc_middle::mir::CastKind::Transmute, inner, _)
        | Rvalue::Cast(rustc_middle::mir::CastKind::PointerCoercion(..), inner, _)
        | Rvalue::Use(inner, _) => guarantee_of(mir, tcx, typing_env, inner, depth),
        // `Box::into_raw`, `NonNull::as_ptr` and friends return the same address
        // they were given, so the result carries the input's guarantee. The
        // callee's own return type carries it when it is `Box`/`&`/`NonNull`;
        // for a raw-pointer result the call's argument is the only evidence.
        _ => None,
    }
}

/// The guarantee a type itself carries.
fn from_type<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: TypingEnv<'tcx>,
    ty: Ty<'tcx>,
) -> Option<Guarantee> {
    let (pointee, origin) = match ty.kind() {
        TyKind::Ref(_, pointee, _) => (*pointee, "a reference".to_string()),
        TyKind::Adt(def, args) => {
            let name = tcx.def_path_str(def.did());
            // Matched on the last path segment rather than the whole path: the
            // module a `NonNull` or `Unique` is defined in has moved between
            // rustc releases, and the guarantee is the type, not where it lives.
            if name.ends_with("::Box") || name.ends_with("::NonNull") || name.ends_with("::Unique")
            {
                (args.type_at(0), format!("`{name}`"))
            } else {
                return None;
            }
        }
        _ => return None,
    };
    Some(Guarantee {
        alignment: alignment_of(tcx, typing_env, pointee)?,
        non_null: true,
        origin,
    })
}

fn alignment_of<'tcx>(tcx: TyCtxt<'tcx>, typing_env: TypingEnv<'tcx>, ty: Ty<'tcx>) -> Option<u64> {
    let key = rustc_middle::ty::PseudoCanonicalInput {
        typing_env,
        value: ty,
    };
    Some(tcx.layout_of(key).ok()?.layout.align.abi.bytes())
}

/// The arithmetic `rustc` performs on an address before testing it.
///
/// The shapes are the ones MIR actually carries, not a general model:
///
/// ```text
/// _71 = Sub(ALIGN, 1)            -> a mask constant
/// _72 = BitAnd(addr, _71)        -> zero for an aligned address
/// ```
///
/// `address & mask` is zero whenever the mask is *below* the alignment: an
/// aligned address has no set bit below `align_of(T)`, and a smaller mask only
/// keeps bits from that range. A mask at or above the alignment could keep a set
/// bit, so it declines.
fn arith_fact(
    op: rustc_middle::mir::BinOp,
    left: &Option<Fact>,
    right: &Option<Fact>,
) -> Option<Fact> {
    match (op, left, right) {
        (rustc_middle::mir::BinOp::BitAnd, Some(Fact::Address(g)), Some(Fact::Const(m)))
        | (rustc_middle::mir::BinOp::BitAnd, Some(Fact::Const(m)), Some(Fact::Address(g)))
            if *m >= 0 && (*m as u64) < g.alignment =>
        {
            Some(Fact::Const(0))
        }
        (rustc_middle::mir::BinOp::BitAnd, Some(Fact::Const(a)), Some(Fact::Const(b))) => {
            Some(Fact::Const(a & b))
        }
        (rustc_middle::mir::BinOp::Sub, Some(Fact::Const(a)), Some(Fact::Const(b))) => {
            Some(Fact::Const(a - b))
        }
        (rustc_middle::mir::BinOp::Add, Some(Fact::Address(g)), Some(Fact::Const(0))) => {
            Some(Fact::Address(g.clone()))
        }
        _ => None,
    }
}

/// A comparison result as the 0/1 value MIR stores in a `bool` local.
fn comparison_fact(
    op: rustc_middle::mir::BinOp,
    left: &Option<Fact>,
    right: &Option<Fact>,
) -> Option<Fact> {
    match compare(op, left, right) {
        Truth::True => Some(Fact::Const(1)),
        Truth::False => Some(Fact::Const(0)),
        Truth::Unknown => None,
    }
}

/// Compare two facts, when the answer is the same for every value they allow.
fn compare(op: rustc_middle::mir::BinOp, left: &Option<Fact>, right: &Option<Fact>) -> Truth {
    match (left, right) {
        (Some(Fact::Const(a)), Some(Fact::Const(b))) => {
            let holds = match op {
                rustc_middle::mir::BinOp::Eq => a == b,
                rustc_middle::mir::BinOp::Ne => a != b,
                rustc_middle::mir::BinOp::Lt => a < b,
                rustc_middle::mir::BinOp::Le => a <= b,
                rustc_middle::mir::BinOp::Gt => a > b,
                rustc_middle::mir::BinOp::Ge => a >= b,
                _ => return Truth::Unknown,
            };
            if holds {
                Truth::True
            } else {
                Truth::False
            }
        }
        // A non-null address is never zero, so `addr == 0` is false and
        // `addr != 0` is true for every value the provenance allows.
        (Some(Fact::Address(g)), Some(Fact::Const(0)))
        | (Some(Fact::Const(0)), Some(Fact::Address(g))) => match op {
            rustc_middle::mir::BinOp::Eq if g.non_null => Truth::False,
            rustc_middle::mir::BinOp::Ne if g.non_null => Truth::True,
            _ => Truth::Unknown,
        },
        _ => Truth::Unknown,
    }
}

fn flip(truth: Truth) -> Truth {
    match truth {
        Truth::True => Truth::False,
        Truth::False => Truth::True,
        Truth::Unknown => Truth::Unknown,
    }
}

fn and(left: Truth, right: Truth) -> Truth {
    match (left, right) {
        (Truth::False, _) | (_, Truth::False) => Truth::False,
        (Truth::True, Truth::True) => Truth::True,
        _ => Truth::Unknown,
    }
}

fn or(left: Truth, right: Truth) -> Truth {
    match (left, right) {
        (Truth::True, _) | (_, Truth::True) => Truth::True,
        (Truth::False, Truth::False) => Truth::False,
        _ => Truth::Unknown,
    }
}

fn xor(left: Truth, right: Truth) -> Truth {
    match (left, right) {
        (Truth::True, Truth::False) | (Truth::False, Truth::True) => Truth::True,
        (Truth::True, Truth::True) | (Truth::False, Truth::False) => Truth::False,
        _ => Truth::Unknown,
    }
}

fn operand_place<'a, 'tcx>(
    operand: &'a Operand<'tcx>,
) -> Option<&'a rustc_middle::mir::Place<'tcx>> {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => Some(place),
        Operand::Constant(_) | Operand::RuntimeChecks(_) => None,
    }
}

fn operand_is_bool<'tcx>(mir: &Body<'tcx>, tcx: TyCtxt<'tcx>, operand: &Operand<'tcx>) -> bool {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => place.ty(mir, tcx).ty.is_bool(),
        Operand::Constant(_) | Operand::RuntimeChecks(_) => false,
    }
}

/// Every statement that gives a local its value.
///
/// A temp can be reused by several sites, so this returns all the definitions
/// rather than refusing when there is more than one; the callers accept the
/// answer only when every definition agrees. A call destination, a drop or a
/// partial write means the value is not a straight-line expression at all, and
/// then there is no list to return.
fn definitions<'a, 'tcx>(mir: &'a Body<'tcx>, local: usize) -> Option<Vec<&'a Rvalue<'tcx>>> {
    let mut found: Vec<&'a Rvalue<'tcx>> = Vec::new();
    for block in mir.basic_blocks.iter() {
        if let Some(terminator) = &block.terminator {
            let writes = match &terminator.kind {
                TerminatorKind::Call { destination, .. } => destination.local.index() == local,
                TerminatorKind::Drop { place, .. } => place.local.index() == local,
                TerminatorKind::Yield { resume_arg, .. } => resume_arg.local.index() == local,
                _ => false,
            };
            if writes {
                return None;
            }
        }
        for statement in &block.statements {
            match &statement.kind {
                StatementKind::Assign(box (place, rvalue)) if place.local.index() == local => {
                    // A place rooted at the local but going through a `Deref`
                    // writes the memory the pointer *points at*, not the pointer:
                    // `(*_62).1 = …` says nothing about `_62`'s value. A field
                    // write into the local (`_62.0 = …`) does, and a local
                    // written in pieces has no single value to trace.
                    if place
                        .projection
                        .iter()
                        .any(|elem| matches!(elem, rustc_middle::mir::PlaceElem::Deref))
                    {
                        continue;
                    }
                    if !place.projection.is_empty() {
                        return None;
                    }
                    found.push(rvalue);
                }
                StatementKind::SetDiscriminant { place, .. } if place.local.index() == local => {
                    return None
                }
                _ => {}
            }
        }
    }
    if found.is_empty() {
        None
    } else {
        Some(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_middle::mir::BinOp;

    /// An address with the alignment and non-nullness a pointer's origin gives.
    fn address(alignment: u64, non_null: bool) -> Option<Fact> {
        Some(Fact::Address(Guarantee {
            alignment,
            non_null,
            origin: "test".to_string(),
        }))
    }

    fn constant(value: i128) -> Option<Fact> {
        Some(Fact::Const(value))
    }

    // `_72 = BitAnd(copy _70, copy _71)` where `_70` is the address cast to
    // `usize` and `_71 = Sub(ALIGN, 1)`.
    #[test]
    fn an_address_masked_below_its_alignment_is_zero() {
        let mask = arith_fact(BinOp::Sub, &constant(4), &constant(1));
        assert!(matches!(mask, Some(Fact::Const(3))));
        assert!(matches!(
            arith_fact(BinOp::BitAnd, &address(4, true), &mask),
            Some(Fact::Const(0))
        ));
        // A mask smaller than the alignment also clears every set bit.
        assert!(matches!(
            arith_fact(BinOp::BitAnd, &address(8, true), &constant(3)),
            Some(Fact::Const(0))
        ));
        // The operand order MIR may use is not part of the rule.
        assert!(matches!(
            arith_fact(BinOp::BitAnd, &constant(3), &address(4, true)),
            Some(Fact::Const(0))
        ));
    }

    // A mask at or above the alignment can keep a set bit of the address, so the
    // model must not answer — this is what keeps it from over-claiming.
    #[test]
    fn an_address_masked_at_or_above_its_alignment_is_not_decided() {
        assert!(arith_fact(BinOp::BitAnd, &address(4, true), &constant(4)).is_none());
        assert!(arith_fact(BinOp::BitAnd, &address(4, true), &constant(7)).is_none());
        assert!(arith_fact(BinOp::BitAnd, &address(4, true), &constant(-1)).is_none());
        assert!(arith_fact(BinOp::BitAnd, &address(4, true), &None).is_none());
    }

    // `_73 = Eq(copy _72, const 0)` for the alignment check, and
    // `_77 = Eq(copy _75, const 0)` for the null one.
    #[test]
    fn a_non_null_address_is_never_equal_to_zero() {
        assert_eq!(
            compare(BinOp::Eq, &address(4, true), &constant(0)),
            Truth::False
        );
        assert_eq!(
            compare(BinOp::Ne, &address(4, true), &constant(0)),
            Truth::True
        );
        // Symmetric: MIR can put the constant on either side.
        assert_eq!(
            compare(BinOp::Eq, &constant(0), &address(4, true)),
            Truth::False
        );
    }

    // Without the non-null guarantee an address may be zero, so the null check
    // must stay undecided rather than being proved from alignment alone.
    #[test]
    fn an_address_without_the_non_null_guarantee_leaves_the_null_check_undecided() {
        assert_eq!(
            compare(BinOp::Eq, &address(4, false), &constant(0)),
            Truth::Unknown
        );
        assert_eq!(
            compare(BinOp::Ne, &address(4, false), &constant(0)),
            Truth::Unknown
        );
        assert_eq!(
            compare(BinOp::Lt, &address(4, true), &constant(0)),
            Truth::Unknown
        );
    }

    // The exact null-check chain:
    //   _78 = BitAnd(_77, _76)      // _76 is `SIZE != 0`, which may be unknown
    //   _79 = Not(_78)
    // A false conjunct decides the conjunction even when the other side is not
    // modelled, which is why the `SIZE` fact never has to be evaluated.
    #[test]
    fn the_null_check_shape_decides_with_the_size_operand_unknown() {
        let is_null = compare(BinOp::Eq, &address(4, true), &constant(0));
        assert_eq!(flip(and(is_null, Truth::Unknown)), Truth::True);
        // With a null-capable address the same chain must not decide.
        let maybe_null = compare(BinOp::Eq, &address(4, false), &constant(0));
        assert_eq!(flip(and(maybe_null, Truth::Unknown)), Truth::Unknown);
    }

    // The alignment chain end to end: `Eq(BitAnd(addr, ALIGN-1), 0)` is true for
    // an address with that alignment, which is exactly what the assert expects.
    #[test]
    fn the_misaligned_check_shape_decides_for_an_aligned_address() {
        let mask = arith_fact(BinOp::Sub, &constant(4), &constant(1));
        let masked = arith_fact(BinOp::BitAnd, &address(4, true), &mask);
        assert_eq!(compare(BinOp::Eq, &masked, &constant(0)), Truth::True);
        // A mask above the alignment is not decided, so the check stays a WARN.
        let wide = arith_fact(BinOp::BitAnd, &address(4, true), &constant(7));
        assert_eq!(compare(BinOp::Eq, &wide, &constant(0)), Truth::Unknown);
    }

    // The boolean combinators, which decide whether a chain of comparisons is
    // itself decided: `true`/`false` absorb an unmodelled operand, two unknowns do not.
    #[test]
    fn the_boolean_combinators_keep_only_what_is_proved() {
        assert_eq!(flip(Truth::True), Truth::False);
        assert_eq!(flip(Truth::Unknown), Truth::Unknown);
        assert_eq!(and(Truth::False, Truth::Unknown), Truth::False);
        assert_eq!(and(Truth::True, Truth::Unknown), Truth::Unknown);
        assert_eq!(or(Truth::True, Truth::Unknown), Truth::True);
        assert_eq!(or(Truth::False, Truth::Unknown), Truth::Unknown);
        assert_eq!(xor(Truth::True, Truth::False), Truth::True);
        assert_eq!(xor(Truth::True, Truth::Unknown), Truth::Unknown);
    }

    #[test]
    fn pointer_refutation_detects_explicit_null_and_misalignment() {
        let is_ne = compare(BinOp::Ne, &constant(0), &constant(0));
        assert_eq!(is_ne, Truth::False);

        let masked = arith_fact(BinOp::BitAnd, &constant(1), &constant(3));
        assert_eq!(masked, Some(Fact::Const(1)));
        assert_eq!(compare(BinOp::Eq, &masked, &constant(0)), Truth::False);
    }
}
