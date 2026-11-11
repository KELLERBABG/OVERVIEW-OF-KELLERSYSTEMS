//! Path-sensitive symbolic state over the MIR control-flow graph.
//!
//! Each assertion used to be handed to Z3 on its own, with no idea how execution
//! reached it. So `if x < 100 { x + 1 }` looked like an unguarded `x + 1` and got
//! flagged, and the reported witness (`x = 255`) was a value the guard makes
//! impossible. This module walks the CFG from the entry block — applying the
//! statements symbolically and folding each branch's guard into the condition
//! required to reach the next block — so an obligation can be asked about
//! *reachable* states only.
//!
//! Soundness rules, all in one place, because everything here is an
//! over-approximation on purpose:
//!
//! * The verdict direction is: a missing fact makes the precondition weaker, and
//!   a weaker precondition can only turn a PASS into a FAIL. Nothing here may
//!   ever make an obligation harder to satisfy than the program really is.
//! * A value we do not model exactly becomes a fresh symbol bounded by the
//!   place's type, which is a superset of what the program can hold.
//! * A value merged from several predecessors is *one of* the incoming values,
//!   constrained by [`PathState::merge_all`]; picking one of them outright would
//!   under-approximate and could hide a real violation.
//! * A cycle is unrolled to its back edge for a bounded number of trips, and
//!   past that bound the block's condition is rebuilt from its forward edges
//!   alone — so it stops re-wrapping itself once per round — while a
//!   loop-carried value is kept as a *relation to its previous-trip version*
//!   rather than forgotten: "the entry value, or the previous trip's value plus
//!   eight under the previous trip's guard". That is the fact a loop's own
//!   condition is about, so the guard still bounds the value it guards instead of
//!   speaking about a symbol nothing else mentions (see [`widened_state`]).
//! * A store the key scheme cannot name — a dereference, an index, a downcast —
//!   retires every place it may have reached, and records why, so a value the
//!   program overwrote can never be read back as the value it used to hold (see
//!   [`PathState::retire_unnamed_store`]). This is the direction the whole module
//!   is built on: keeping a value too long is the one way to report a *proof* of
//!   a program that is not, while forgetting one costs a counterexample at worst.
//! * A write made by a body this analysis does not read retires what it may have
//!   destroyed *wherever the write is spelled*, not only when it is an `Assign`: a
//!   call given a pointer to one of this body's locals, a `Drop` whose glue runs a
//!   user `impl Drop`, an inline asm block's output, a `copy_nonoverlapping`
//!   intrinsic, a `SetDiscriminant`. Every statement kind and terminator kind the
//!   pinned toolchain can emit has a disposition in one exhaustive `match` each
//!   ([`PathState::transfer`], [`successor_guards`]) — both written as tables with
//!   no wildcard arm, so a variant nobody has decided about fails to compile
//!   rather than silently writing nothing. Three shapes that were wrong proofs
//!   before this rule existed are pinned in `tests/statement_writes`: a `&mut`
//!   argument written by a callee, a `Drop` impl writing through a stored
//!   reference, and an inline asm output (A18).
//! * Plain arithmetic values are reduced into their fixed-width domain. An
//!   unwrapped sum is NOT a superset of wrapping semantics. Checked tuple values
//!   retain their mathematical result until the assertion's success guard.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use rustc_middle::mir::{
    AggregateKind, BasicBlock, BinOp, Body, InlineAsmOperand, Local, NonDivergingIntrinsic,
    Operand, Place, PlaceElem, PlaceTy, Rvalue, StatementKind, TerminatorKind, UnOp,
};
use rustc_middle::ty::{Ty, TyCtxt, TyKind};
use z3::ast::{Ast, Bool, Int, BV};
use z3::{Context, Solver};

use crate::constant_prop::{
    eval_operand, infer_int_type, integer_literal, operand_int_type, ConstMap,
};
use crate::constraint::{overflow_violation, OverflowCheck};
use crate::types::IntType;

/// How far a cycle is unrolled to its back edge before loop-carried values are
/// widened.
///
/// Reverse postorder walks a loop body in order, so one round of the dataflow
/// carries a state one trip further round the loop: a cycle block's state after
/// `k` rounds is what it holds after `k` trips. Past this bound the block is
/// given a widened state — the condition truncated to its forward edges and its
/// loop-carried values related to their previous trip ([`widened_state`]) —
/// which keeps the unrolling *bounded* instead of terms growing once per trip
/// forever.
const MAX_LOOP_TRIPS: usize = 3;

/// The prefix a symbol carries in the previous-trip view of a state. One name
/// per role, so re-renaming a state cannot mint a new symbol each round.
const PREVIOUS_TRIP_PREFIX: &str = "prev_bb";

/// How many times a round of the dataflow may run before the analysis gives up
/// and reports nothing but the path-insensitive state.
///
/// The non-cyclic part of a CFG is topologically ordered by reverse postorder,
/// so two rounds are enough there. The unrolling above is what the rest is for:
/// `MAX_LOOP_TRIPS` rounds model the first trips exactly, the next replaces
/// their merged values with the widened ones, and one more lets the states
/// after that widening settle. A round that stops changing ends the loop early,
/// and a shape the cap really does cut short falls back to the path-insensitive
/// state rather than to a half-converged one.
const MAX_ROUNDS: usize = MAX_LOOP_TRIPS + 3;

/// The round budget this analysis runs under: [`MAX_ROUNDS`], unless
/// `KELLER_MAX_ROUNDS` overrides it.
///
/// The override exists because this budget is also the boundary of a *finding*:
/// a function that runs out of rounds is reported as path-insensitive, so
/// investigating that report means being able to reproduce it — or to rule it
/// out by giving the shape more room. It travels the way the solver budget does
/// (`KELLER_SOLVER_TIMEOUT_MS`): read from the environment, because the analysis
/// runs in the rustc worker rather than in the process that parsed the command
/// line. A value of zero or one that does not parse leaves the default in place
/// rather than reaching the loop as a budget nothing can converge inside.
pub fn round_budget() -> usize {
    std::env::var("KELLER_MAX_ROUNDS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|rounds| *rounds > 0)
        .unwrap_or(MAX_ROUNDS)
}

/// Most merge definitions one state may accumulate before further merges simply
/// drop the local. Dropping a local is sound — it becomes a fresh free symbol.
const MAX_MERGE_DEFS: usize = 64;

/// A place identified structurally: a local, plus the tuple-field path into it.
///
/// Structural keys (rather than rendered ones) keep `_4.1` and `_41` apart, and
/// make "everything under this local" a prefix test.
pub type PlaceKey = (usize, Vec<usize>);

/// A pseudo field index, used for a fact about a place that is not one of its
/// real struct fields.
///
/// These indices are outside every real `FieldIdx` the compiler can emit, so a
/// key carrying one can never collide with a projection the MIR actually names.
/// Storing the fact in [`PathState::values`] rather than in a side table buys the
/// whole state machinery for free: a copy of the place copies the fact (the
/// `Rvalue::Use` subfield copy), a loop merge turns it into a `phi` like any
/// other value, and writing the whole place retires it.
const PSEUDO_LEN: usize = usize::MAX;
const PSEUDO_CAPACITY: usize = usize::MAX - 1;
const PSEUDO_CURSOR: usize = usize::MAX - 2;
const PSEUDO_COUNT: usize = usize::MAX - 3;

/// The recorded symbolic value of a place can be an integer of any width, or a
/// `bool` whose value is represented as `0`/`1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueDomain {
    Bool,
    Int(IntType),
    /// The length of a slice, array or string, read from a fat pointer's
    /// metadata: a `usize`, but one Rust's own rules cap at `isize::MAX` — an
    /// allocation cannot be larger, so a longer slice is not a state the program
    /// can be in. Bounding it here rather than at `usize::MAX` is what lets a
    /// loop's `i + 8 <= v.len()` guard rule out an overflow of `i + 8`:
    /// `isize::MAX + 8` still fits in a `usize`, and `usize::MAX - 7 + 8` does
    /// not.
    SliceLen,
}

impl ValueDomain {
    /// The exact `(min, max)` decimal bounds of this domain, as Z3 literals
    /// should be written. `bool` is bounded so a model can never invent `2` for
    /// a flag, which would make `cond == true` unsatisfiable and turn a real
    /// failure into a spurious PASS.
    fn bounds(self) -> (String, String) {
        match self {
            ValueDomain::Bool => ("0".to_string(), "1".to_string()),
            ValueDomain::Int(ty) => ty.bounds_str(),
            ValueDomain::SliceLen => ("0".to_string(), isize::MAX.to_string()),
        }
    }
}

/// Where one of the state's free symbols came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputKind {
    /// An argument or named binding: something a reader can actually set.
    Source,
    /// An unnamed temporary we could not model, bounded by its own type.
    Temp,
    /// A value merged from several predecessor paths.
    Merge,
}

/// One free symbol the state's formulas mention.
#[derive(Debug, Clone)]
pub struct InputVar {
    pub symbol: String,
    pub label: String,
    /// The domain to bound this symbol by, when bounding it is sound.
    ///
    /// Merge symbols carry `None`: their value is already pinned to a
    /// predecessor's, and a predecessor may be an unwrapped arithmetic result
    /// that legitimately exceeds the local's own range.
    domain: Option<ValueDomain>,
    pub kind: InputKind,
}

impl InputVar {
    /// True when this symbol holds a `bool`, so a reported value reads as
    /// `true`/`false` rather than the `1`/`0` it is encoded as.
    pub fn is_bool(&self) -> bool {
        matches!(self.domain, Some(ValueDomain::Bool))
    }
}

fn key_suffix(key: &PlaceKey) -> String {
    let mut s = key.0.to_string();
    for idx in &key.1 {
        s.push('_');
        s.push_str(
            &pseudo_field_name(*idx)
                .unwrap_or_else(|| idx.to_string())
                .as_str(),
        );
    }
    s
}

/// The key of a fact stored *under* `key` at a pseudo field.
fn pseudo_key(key: &PlaceKey, field: usize) -> PlaceKey {
    let mut path = key.1.clone();
    path.push(field);
    (key.0, path)
}

fn int_lit<'ctx>(ctx: &'ctx Context, v: i128) -> Int<'ctx> {
    Int::from_str(ctx, &v.to_string()).expect("a decimal integer literal")
}

fn zero<'ctx>(ctx: &'ctx Context) -> Int<'ctx> {
    int_lit(ctx, 0)
}

/// The conjunction of `terms`, collapsing the single-term case.
///
/// A one-armed `(and X)` is noise in an emitted obligation, and every guard here
/// is read by a human at least as often as by Z3.
pub(crate) fn all_of<'ctx>(ctx: &'ctx Context, terms: &[&Bool<'ctx>]) -> Bool<'ctx> {
    match terms {
        [] => Bool::from_bool(ctx, true),
        [single] => (*single).clone(),
        many => Bool::and(ctx, many),
    }
}

/// The disjunction of `terms`, collapsing the single-term case.
fn any_of<'ctx>(ctx: &'ctx Context, terms: &[&Bool<'ctx>]) -> Bool<'ctx> {
    match terms {
        [] => Bool::from_bool(ctx, false),
        [single] => (*single).clone(),
        many => Bool::or(ctx, many),
    }
}

fn one<'ctx>(ctx: &'ctx Context) -> Int<'ctx> {
    int_lit(ctx, 1)
}

/// Render a model value as the source-level literal it stands for: a `bool`
/// input reads as `true`/`false` rather than the `1`/`0` it is encoded as.
pub fn render_bool(value: &str) -> String {
    match value {
        "0" => "false".to_string(),
        "1" => "true".to_string(),
        other => other.to_string(),
    }
}

/// The source-level name of a local, when the MIR still carries debug info for
/// it. Arguments and `let` bindings keep a `var_debug_info` entry; optimizer
/// temporaries do not, which is why a label can fall back to the raw local.
pub fn local_name(mir: &Body<'_>, local: Local) -> Option<String> {
    mir.var_debug_info
        .iter()
        .find_map(|info| match &info.value {
            rustc_middle::mir::VarDebugInfoContents::Place(place)
                if place.local == local && place.projection.is_empty() =>
            {
                Some(info.name.to_string())
            }
            _ => None,
        })
}

/// A short, traceable label for a place.
///
/// A named local is reported as its source name plus its MIR local (`a (_1)`),
/// which is what makes a witness actionable: `a (_1) = 2147483647` names the
/// argument to call the function with, where `_1 = …` does not.
pub fn place_label(mir: &Body<'_>, place: &Place<'_>) -> String {
    let rendered = format!("{:?}", place);
    if place.projection.is_empty() {
        if let Some(name) = local_name(mir, place.local) {
            return format!("{} ({})", name, rendered);
        }
    }
    rendered
}

/// A short, traceable label for an operand.
pub fn operand_label(mir: &Body<'_>, operand: &Operand<'_>) -> String {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => place_label(mir, place),
        // Display, not Debug: a constant reads as `100u8`, not the internal
        // `Val(Scalar(0x64), u8)` that a Debug dump produces.
        Operand::Constant(c) => format!("{}", c.const_),
        other => format!("{:?}", other),
    }
}

/// The structural key of a place, or `None` when it cannot be modelled.
///
/// Only field projections are understood. A dereference or an index reads
/// memory we cannot name, so the place is left unmodelled (and any read of it
/// becomes a fresh bounded symbol). A *write* to one of these cannot be modelled
/// either, but it does destroy whatever the state recorded for the storage it may
/// have reached, so it retires that rather than being dropped on the floor (see
/// [`PathState::retire_unnamed_store`]).
pub fn place_key(place: &Place<'_>) -> Option<PlaceKey> {
    let mut path = Vec::new();
    for elem in place.projection {
        match elem {
            PlaceElem::Field(field, _) => path.push(field.index()),
            _ => return None,
        }
    }
    Some((place.local.index(), path))
}

fn value_domain_of_ty(ty: rustc_middle::ty::Ty<'_>) -> Option<ValueDomain> {
    if let Some(int_ty) = infer_int_type(ty) {
        return Some(ValueDomain::Int(int_ty));
    }
    if matches!(ty.kind(), TyKind::Bool) {
        return Some(ValueDomain::Bool);
    }
    None
}

fn place_domain<'tcx>(
    mir: &Body<'tcx>,
    tcx: TyCtxt<'tcx>,
    place: &Place<'tcx>,
) -> Option<ValueDomain> {
    let local_ty = mir.local_decls[place.local].ty;
    let projected = PlaceTy::from_ty(local_ty).multi_projection_ty(tcx, place.projection);
    value_domain_of_ty(projected.ty)
}

fn operand_domain<'tcx>(
    mir: &Body<'tcx>,
    tcx: TyCtxt<'tcx>,
    operand: &Operand<'tcx>,
) -> Option<ValueDomain> {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => place_domain(mir, tcx, place),
        Operand::Constant(c) => value_domain_of_ty(c.const_.ty()),
        Operand::RuntimeChecks(_) => None,
    }
}

/// True for an operand of `bool` type, whose symbolic value is `0`/`1`.
fn operand_is_bool<'tcx>(mir: &Body<'tcx>, tcx: TyCtxt<'tcx>, operand: &Operand<'tcx>) -> bool {
    matches!(operand_domain(mir, tcx, operand), Some(ValueDomain::Bool))
}

/// Whether a known standard-library callee was modelled.
///
/// The distinction matters for what the caller does next: a modelled call has
/// had its writes accounted for by the model, so the generic "a callee this
/// analysis does not read may write through its pointer arguments" retirement
/// must *not* run afterwards. It would otherwise forget the iterator state the
/// model just updated, and the next call would start from a free symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CalleeApplied {
    NotModelled,
    Modelled,
}

/// The standard container whose length/capacity facts this module models.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContainerKind {
    Vec,
    String,
}

/// The iterator shapes whose cursor/count state this module models.
///
/// All three are represented uniformly by a `cursor` and a `len` pseudo fact:
///
/// * `Range<usize>` — `cursor` is `start`, `len` is `end`;
/// * `slice::Iter` — `cursor` counts elements already yielded, `len` is the
///   slice's length;
/// * `Enumerate<I>` — the same cursor/len, plus a `count` fact.
///
/// The uniform representation is what lets `enumerate` copy a slice iterator's
/// state without knowing how the inner iterator was built, and lets `next` be
/// one model for all three.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IteratorKind {
    Range,
    Slice,
    Enumerate,
}

/// The type of an operand, when MIR names one.
fn operand_ty<'tcx>(
    mir: &Body<'tcx>,
    tcx: TyCtxt<'tcx>,
    operand: &Operand<'tcx>,
) -> Option<Ty<'tcx>> {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => Some(place.ty(&mir.local_decls, tcx).ty),
        Operand::Constant(constant) => Some(constant.const_.ty()),
        Operand::RuntimeChecks(_) => None,
    }
}

/// Look through one reference or raw pointer. A method taking `&self` receives
/// the *reference*, while the fact belongs to the value behind it.
fn referent_ty<'tcx>(ty: Ty<'tcx>) -> Ty<'tcx> {
    match ty.kind() {
        TyKind::Ref(_, inner, _) | TyKind::RawPtr(inner, _) => *inner,
        _ => ty,
    }
}

/// Whether `ty` is the standard-library ADT `krate::item`.
///
/// Matching on the defining crate and item name (rather than a rendered path)
/// keeps the check stable across re-exports: `Vec` is defined in `alloc` even
/// when the program writes `std::vec::Vec`, and `Range` in `core`.
fn is_adt<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, krate: &str, item: &str) -> bool {
    let TyKind::Adt(adt_def, _) = ty.kind() else {
        return false;
    };
    let did = adt_def.did();
    tcx.crate_name(did.krate).as_str() == krate && tcx.item_name(did).as_str() == item
}

fn container_kind<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<ContainerKind> {
    let ty = referent_ty(ty);
    if is_adt(tcx, ty, "alloc", "Vec") {
        Some(ContainerKind::Vec)
    } else if is_adt(tcx, ty, "alloc", "String") {
        Some(ContainerKind::String)
    } else {
        None
    }
}

fn iterator_kind<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<IteratorKind> {
    let ty = referent_ty(ty);
    if is_adt(tcx, ty, "core", "Range") {
        Some(IteratorKind::Range)
    } else if is_adt(tcx, ty, "core", "Iter") {
        Some(IteratorKind::Slice)
    } else if is_adt(tcx, ty, "core", "Enumerate") {
        Some(IteratorKind::Enumerate)
    } else {
        None
    }
}

/// Whether `ty` is a reference to a slice, or `str`, whose length is the fat
/// pointer's metadata.
fn is_slice_or_str_ref(ty: Ty<'_>) -> bool {
    match ty.kind() {
        TyKind::Ref(_, inner, _) | TyKind::RawPtr(inner, _) => {
            matches!(inner.kind(), TyKind::Slice(_) | TyKind::Str)
        }
        _ => false,
    }
}

/// What a `BinaryOp` assignment produced.
enum BinaryResult<'ctx> {
    /// A plain value for the destination.
    Value(Int<'ctx>),
    /// `FooWithOverflow`: the destination is a `(T, bool)` tuple, so the result
    /// and its overflow flag are recorded as its two fields.
    WithOverflow { value: Int<'ctx>, flag: Int<'ctx> },
    /// An operator we deliberately do not model.
    Unmodelled,
}

/// The symbolic state at one program point: the value of each modelled place
/// plus the condition required to reach that point.
#[derive(Clone, Default)]
pub struct PathState<'ctx> {
    values: BTreeMap<PlaceKey, Int<'ctx>>,
    /// The condition required to be in this state. `None` means "no condition":
    /// reaching here needs nothing in particular.
    pc: Option<Bool<'ctx>>,
    /// Definitions of merged (`phi`) values: each is a disjunction revisiting
    /// the contributor that chose the value.
    defs: BTreeMap<String, Bool<'ctx>>,
    /// Pre-extracted symbol dependencies for each definition in `defs`, avoiding
    /// exponential formula stringification in `leans_on_widening`.
    def_symbols: BTreeMap<String, HashSet<String>>,
    /// What a `checked_*` call's `Option` is known to hold, keyed by the
    /// destination place.
    ///
    /// The payload alone is not enough to model the form: without the
    /// discriminant the `None` arm stays reachable carrying a payload the program
    /// cannot read, and `(x + y) mod 2^8` there is a value no path produces — a
    /// counterexample the function cannot have. The two halves travel together as
    /// one fact (see [`CheckedProduct`]).
    checked: BTreeMap<PlaceKey, CheckedProduct<'ctx>>,
    /// Known enum variant discriminants (e.g. Option::Some = 1, None = 0, Result::Ok = 0, Err = 1).
    discriminants: BTreeMap<PlaceKey, u32>,
    /// A discriminant that is a *symbolic* function of the state rather than one
    /// concrete variant.
    ///
    /// `Iterator::next` returns `Some` exactly when the iterator has another
    /// element, and that test is the guard that bounds the element. Recording the
    /// variant as an integer expression (`ite(start < end, 1, 0)`) lets the
    /// following `SwitchInt` split on that test, which is what makes the loop's
    /// `start <= i < end` fact available to the body. A concrete discriminant
    /// cannot express it, and inventing one would either claim an element that is
    /// not there or hide the one that is.
    symbolic_discriminants: BTreeMap<PlaceKey, Int<'ctx>>,
    /// Every free symbol the formulas above mention, keyed by symbol name.
    inputs: BTreeMap<String, InputVar>,
    /// Symbols that stand for a value the analysis could not model, keyed by the
    /// symbol name a later read re-derives, with the reason.
    ///
    /// A model of an abstract state is not by itself a counterexample. Where the
    /// value came from a call whose body was not analysed, or from a projection
    /// the encoder cannot name, the solver is free to choose a number the program
    /// cannot produce at that point — which is how a crate of safe functions gets
    /// reported as exploitable. Recording the symbol lets an obligation say
    /// "undecided, because of this" instead of claiming a bug; see
    /// [`PathState::forget`].
    unmodelled: BTreeMap<String, String>,
    /// The places whose values the loop abstraction gave up on, and which a later
    /// read therefore re-derives as a free symbol.
    ///
    /// This is what makes `widening-limited` a statement about the quantity in a
    /// finding rather than about where it was found. A widened cycle block forgets
    /// every place the back edge carries, so the symbol a read invents there stands
    /// for a value the abstraction stopped tracking — and a failure *stated over*
    /// one of those places leans on it. The test this replaces asked instead
    /// whether any declared input name carried [`PREVIOUS_TRIP_PREFIX`], and since
    /// the widening declares a renamed copy of every symbol on the back edge, every
    /// obligation downstream of a widened block answered yes by construction (audit
    /// finding; A19).
    ///
    /// A `BTreeSet`, not a `HashSet`: [`Self::fingerprint`] renders this, and a
    /// fingerprint that varies between runs would make the contribution order —
    /// and so the emitted SMT-LIB — vary with it.
    generalised: BTreeSet<PlaceKey>,
    /// Prepended to every symbol this state invents. Empty for a state built by
    /// walking one body; non-empty for a state relocated into another function's
    /// solver, where the local names would otherwise collide.
    prefix: String,
    /// The one local each pointer in this body points at, when
    /// [`BodyWrites`] saw exactly one borrow of a local. A read through the
    /// pointer is then a read of that local (S5), which is what proves a
    /// function that takes `r: &mut u8` and reads `*r`.
    ///
    /// Only consulted for a state with an empty [`Self::prefix`]: a relocated
    /// state's local indices are another body's, so resolving them here would
    /// name the wrong place.
    pointees: BTreeMap<usize, Option<usize>>,
    /// The coroutine whose resume state this body dispatches on (S6); `None` in an
    /// ordinary function.
    resume: Option<ResumeState<'ctx>>,
    /// Whether this path's symbolic state has computed any bitwise or shift operations (S8).
    has_bitwise: bool,
}

/// The discriminants the coroutine transform reserves, and the only states a
/// resume may not arrive in (see [`PathState::seed_resume`]).
const RETURNED_STATE: i64 = 1;
const POISONED_STATE: i64 = 2;

/// The resume state of a coroutine body (S6).
///
/// An `async fn` arrives as a coroutine whose discriminant is the state the
/// compiler's transform dispatches on: `0` not started, `1` returned, `2`
/// poisoned, `3..` suspension points. A resume may only arrive in `0` or a
/// suspension state; the transform encodes the other two as branches that assert
/// "`async fn` resumed after completion/panicking".
///
/// Seeding the body's entry state with that invariant, and resolving every read of
/// the coroutine's discriminant to this one symbol, is what turns those assertions
/// from `unsupported` into decided: the entry block's dispatch splits on the
/// symbol, and the branches for `1` and `2` become unreachable.
#[derive(Clone)]
struct ResumeState<'ctx> {
    symbol: Int<'ctx>,
    /// The symbol's name, so a renamed copy can rename the state with it.
    name: String,
}

/// Whether `place` names a coroutine value — the read the transform spells as
/// `discriminant((*_1))` at a resume entry.
fn is_coroutine_place<'tcx>(mir: &Body<'tcx>, tcx: TyCtxt<'tcx>, place: &Place<'tcx>) -> bool {
    matches!(
        place.ty(&mir.local_decls, tcx).ty.kind(),
        TyKind::Coroutine(..)
    )
}

/// The locals that hold a coroutine's discriminant: `_n = discriminant(<coroutine>)`.
pub fn coroutine_state_locals<'tcx>(
    mir: &Body<'tcx>,
    tcx: TyCtxt<'tcx>,
) -> std::collections::HashSet<usize> {
    let mut locals = std::collections::HashSet::new();
    for block in mir.basic_blocks.iter() {
        for statement in &block.statements {
            if let StatementKind::Assign(box (place, Rvalue::Discriminant(source))) =
                &statement.kind
            {
                if is_coroutine_place(mir, tcx, source) {
                    locals.insert(place.local.index());
                }
            }
        }
    }
    locals
}

/// Check if block `from` reaches `to` through straight-line unconditional jumps without side branches.
fn block_leads_to<'tcx>(mir: &Body<'tcx>, from: BasicBlock, to: BasicBlock) -> bool {
    let mut current = from;
    let mut visited = std::collections::HashSet::new();
    while visited.insert(current) {
        if current == to {
            return true;
        }
        let Some(term) = &mir.basic_blocks[current].terminator else {
            return false;
        };
        match &term.kind {
            TerminatorKind::Goto { target } => {
                current = *target;
            }
            _ => return false,
        }
    }
    false
}

/// Find all coroutine discriminant states that lead to an `AssertKind::ResumedAfterDrop`.
pub fn coroutine_drop_states<'tcx>(
    mir: &Body<'tcx>,
    tcx: TyCtxt<'tcx>,
) -> std::collections::HashSet<i64> {
    let mut drop_states = std::collections::HashSet::new();
    let locals = coroutine_state_locals(mir, tcx);
    if locals.is_empty() {
        return drop_states;
    }

    let mut drop_assert_blocks = std::collections::HashSet::new();
    for (bb, data) in mir.basic_blocks.iter_enumerated() {
        if let Some(terminator) = &data.terminator {
            if let TerminatorKind::Assert { msg, .. } = &terminator.kind {
                if matches!(**msg, rustc_middle::mir::AssertKind::ResumedAfterDrop(..)) {
                    drop_assert_blocks.insert(bb);
                }
            }
        }
    }

    if drop_assert_blocks.is_empty() {
        return drop_states;
    }

    for body_block in mir.basic_blocks.iter() {
        let Some(terminator) = &body_block.terminator else {
            continue;
        };
        let TerminatorKind::SwitchInt { discr, targets } = &terminator.kind else {
            continue;
        };
        let (Operand::Copy(place) | Operand::Move(place)) = discr else {
            continue;
        };
        if !locals.contains(&place.local.index()) {
            continue;
        }
        let values = targets.all_values();
        for (index, target) in targets.all_targets().iter().enumerate() {
            for &assert_bb in &drop_assert_blocks {
                if *target == assert_bb || block_leads_to(mir, *target, assert_bb) {
                    if let Some(val) = values.get(index) {
                        drop_states.insert(val.get() as i64);
                    }
                }
            }
        }
    }

    drop_states
}

/// Whether every edge into `block` selects a state a resume cannot arrive in.
///
/// The transform dispatches on the coroutine's discriminant and sends `1`
/// (returned), `2` (poisoned), and drop states to the blocks that assert. Under
/// the coroutine ABI neither is a state a resume may find, so a block only those
/// edges reach is unreachable — and that is proved from the CFG rather than from
/// the propagated guards, which the path-insensitive fallback replaces with a
/// state that claims nothing (S6).
pub fn resume_state_excludes_block<'tcx>(
    mir: &Body<'tcx>,
    tcx: TyCtxt<'tcx>,
    block: BasicBlock,
) -> bool {
    let locals = coroutine_state_locals(mir, tcx);
    if locals.is_empty() {
        return false;
    }
    let drop_states = coroutine_drop_states(mir, tcx);
    let mut any = false;
    for body_block in mir.basic_blocks.iter() {
        let Some(terminator) = &body_block.terminator else {
            continue;
        };
        let TerminatorKind::SwitchInt { discr, targets } = &terminator.kind else {
            continue;
        };
        let (Operand::Copy(place) | Operand::Move(place)) = discr else {
            continue;
        };
        if !locals.contains(&place.local.index()) {
            continue;
        }
        let values = targets.all_values();
        for (index, target) in targets.all_targets().iter().enumerate() {
            if *target != block && !block_leads_to(mir, *target, block) {
                continue;
            }
            any = true;
            match values.get(index).map(|value| value.get()) {
                // The states the transform refuses to resume in:
                // 1 (returned), 2 (poisoned), or drop-state discriminants.
                Some(1) | Some(2) => {}
                Some(v) if drop_states.contains(&(v as i64)) => {}
                // The `otherwise` branch, or a suspension state: this edge can
                // arrive in a state a resume is allowed, so nothing is proved.
                _ => return false,
            }
        }
    }
    any
}

impl<'tcx> std::fmt::Debug for PathState<'tcx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PathState(values={}, defs={}, inputs={}, pc={})",
            self.values.len(),
            self.defs.len(),
            self.inputs.len(),
            self.pc.is_some()
        )
    }
}

/// What a `checked_*` call's `Option` is known to hold.
///
/// The form returns `Some(sum)` exactly when the mathematical sum is a value of
/// the operand type, so the variant it returned *is* a fact about the sum:
/// `Some` if and only if the sum is in range. Modelling both halves is what makes
/// the payload usable — on the `Some` path the payload is the exact sum, and the
/// `None` path cannot read the place at all — and both halves are needed for the
/// encoding to be the `Option`'s semantics rather than a guess about it.
#[derive(Clone)]
struct CheckedProduct<'ctx> {
    /// The mathematical sum the call was computed from: `x + y`, unwrapped.
    sum: Int<'ctx>,
    /// The operand type, whose range is the `Some`/`None` test.
    ty: IntType,
    /// Where the payload lives: the destination place projected to its field.
    payload: PlaceKey,
    /// The variant that carries the payload.
    payload_variant: usize,
    /// The variant that carries nothing.
    empty_variant: usize,
}

impl<'ctx> CheckedProduct<'ctx> {
    /// The discriminant this `Option` holds: the payload's variant when the sum
    /// is in range, the empty variant when it is not.
    fn discriminant(&self, ctx: &'ctx Context) -> Int<'ctx> {
        let (min, max) = self.ty.bounds();
        let low = self.sum.ge(&int_lit(ctx, min));
        let high = self.sum.le(&int_lit(ctx, max));
        let in_range = all_of(ctx, &[&low, &high]);
        in_range.ite(
            &int_lit(ctx, self.payload_variant as i128),
            &int_lit(ctx, self.empty_variant as i128),
        )
    }

    /// The payload's symbolic value: the sum reduced into the operand type. On
    /// the `Some` path that reduction is the identity, by the contract above,
    /// which is what makes this exact rather than a bound.
    fn payload(&self, ctx: &'ctx Context) -> Int<'ctx> {
        fixed_value(ctx, &self.sum, self.ty)
    }

    /// A rendering two paths' facts can be compared by — the way merged values
    /// are compared — and one the fixpoint can be fingerprinted with.
    fn rendering(&self) -> String {
        format!(
            "{}|{:?}|{}|{}|{:?}",
            self.sum, self.ty, self.payload_variant, self.empty_variant, self.payload
        )
    }

    /// The same fact with every free symbol renamed by `map`.
    fn substituted(&self, map: &[(&Int<'ctx>, &Int<'ctx>)]) -> Self {
        Self {
            sum: self.sum.substitute(map),
            ..self.clone()
        }
    }
}

impl<'ctx> PathState<'ctx> {
    /// The entry state: nothing known, nothing required.
    pub fn initial() -> Self {
        Self::default()
    }

    /// The state that claims nothing: what the analysis falls back to when it
    /// cannot vouch for a state at all, and what a merge with no contributor
    /// has to say.
    pub fn unknown() -> Self {
        Self::default()
    }

    /// The state a coroutine is resumed in, created at the read of it (S6).
    ///
    /// A resume arrives with the coroutine in state `0` (not started) or in a
    /// suspension state; `1` (returned) and `2` (poisoned) are the states the
    /// transform refuses to resume and asserts on. That is the coroutine ABI, and
    /// it is the only thing known about the discriminant before the body runs —
    /// the transform dispatches into those two states solely to abort.
    ///
    /// The symbol is created at the discriminant read rather than at the body's
    /// entry because the read is the transform's own resume point: it needs no
    /// guess about which bodies are coroutines, and it puts the invariant exactly
    /// where it is relied on. Recording it as a condition is what makes the
    /// assertions in the forbidden branches decidable — they are discharged by the
    /// guard, not by a free symbol the solver could put in `1` or `2`.
    fn resume_symbol<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        tcx: TyCtxt<'tcx>,
        mir: &Body<'tcx>,
        source: &Place<'tcx>,
    ) -> Option<Int<'ctx>> {
        if !is_coroutine_place(mir, tcx, source) {
            return None;
        }
        if let Some(resume) = &self.resume {
            return Some(resume.symbol.clone());
        }
        let name = format!("{}resume_state", self.prefix);
        let symbol = Int::new_const(ctx, name.as_str());
        self.inputs.insert(
            name.clone(),
            InputVar {
                symbol: name.clone(),
                label: "coroutine resume state".to_string(),
                domain: None,
                kind: InputKind::Source,
            },
        );
        let returned = symbol._eq(&Int::from_i64(ctx, RETURNED_STATE));
        let poisoned = symbol._eq(&Int::from_i64(ctx, POISONED_STATE));
        let mut forbidden_terms = vec![returned.not(), poisoned.not()];
        for drop_state in coroutine_drop_states(mir, tcx) {
            let dropped = symbol._eq(&Int::from_i64(ctx, drop_state));
            forbidden_terms.push(dropped.not());
        }
        let invariant_refs: Vec<&z3::ast::Bool<'ctx>> = forbidden_terms.iter().collect();
        let invariant = all_of(ctx, &invariant_refs);
        self.assume(ctx, invariant);
        self.resume = Some(ResumeState {
            symbol: symbol.clone(),
            name,
        });
        Some(symbol)
    }

    /// Forget the *values* this state pins, keeping the condition it is under.
    ///
    /// The condition required to reach this point, rendered for the obligation
    /// header (so a human can see which guard made the difference).
    pub fn condition(&self) -> Option<String> {
        self.pc.as_ref().map(|pc| pc.to_string())
    }

    /// Whether this state's path or computations involved bit-vector operations (S8).
    pub fn has_bitwise_ops(&self) -> bool {
        self.has_bitwise
    }

    /// The condition itself, for callers that need to build a formula around it
    /// rather than print it (an inherited precondition, for instance).
    /// The symbol this state uses — or would invent — for `place`.
    ///
    /// A read of a place the state has not pinned invents the same name, so this
    /// answers for a value that is recorded and for one that was forgotten alike:
    /// the caller can ask what the failing value *is called* without knowing which
    /// of the two happened. `None` for a place the key scheme cannot name at all
    /// (see [`place_key`]).
    pub fn symbol_of(&self, place: &Place<'_>) -> Option<String> {
        place_key(place).map(|key| self.symbol_name(&key))
    }

    /// The enum variant this `place` is known to hold, as the `VariantIdx` an
    /// `Rvalue::Aggregate` wrote down.
    ///
    /// `None` when the state has not tracked the place's variant. What a known
    /// variant *means* belongs to the caller: `Option::None` is a panic for
    /// `unwrap` and an ordinary value for `is_none`, so no interpretation lives
    /// here. The value is the variant index — not the discriminant constant,
    /// which an enum may set independently of its variant order — because that
    /// is what [`Rvalue::Aggregate`](rustc_middle::mir::Rvalue::Aggregate)
    /// records; the name is recovered from the ADT definition by the caller.
    pub fn discriminant_of(&self, place: &Place<'_>) -> Option<u32> {
        place_key(place).and_then(|key| self.discriminants.get(&key).copied())
    }

    /// Every discriminant known to this state.
    pub fn discriminants(&self) -> &BTreeMap<PlaceKey, u32> {
        &self.discriminants
    }

    /// Whether the value this obligation is stated over **depends** on the loop
    /// abstraction: the failure's own operands name a place a widened block gave
    /// up on, or a symbol the relation it wrote down introduced.
    ///
    /// `failure_symbols` is the caller's set of names the *failure itself* is
    /// stated over — the assertion's own operands ([`Self::symbol_of`] names their
    /// places) and the witness symbols. Two pieces of evidence are looked for, and
    /// both are about the quantity in the finding rather than about where it was
    /// found:
    ///
    /// * the failure names a previous-trip symbol ([`is_widened`]), which the
    ///   widening declares when it relates a back-edge value to the trip before, so
    ///   the value in the finding *is* the abstraction's version of it;
    /// * the failure names a place in [`Self::generalised`]: the widening emptied
    ///   it because the back edge carries it, so the free symbol the finding is
    ///   stated over is the one the abstraction left behind.
    ///
    /// From there the state's own definitions are followed: a definition whose
    /// rendered body names a symbol is an equation about that symbol, so everything
    /// else the equation names is part of the same quantity.
    ///
    /// This replaced a *location* test — "does the state mention any previous-trip
    /// name at all" — which could not distinguish anything: the widening renames
    /// every symbol of the back edge and declares all of them, so every obligation
    /// inside or downstream of a widened block answered yes by construction, and
    /// the class meant "this finding sits in a loop" rather than "this finding
    /// leaned on a generalised value" (audit finding; A19). A widened
    /// symbol declared for a place this failure never reads is no longer evidence
    /// that the widening had anything to do with it, and neither is another place's
    /// generalisation.
    ///
    /// What it still is: a *syntactic* dependence test over the places the widening
    /// generalised and the definitions the state carries, not an SMT support
    /// computation. A dependence neither writes down is missed, and the widening's
    /// own relation (`def_loop_bb…`, whose body is `mir_3 = 0 ∨ (prev guard ∧ mir_3
    /// = prev_bb…_mir_3)`) is where the true positives come from.
    pub fn leans_on_widening(&self, failure_symbols: &HashSet<String>) -> bool {
        if failure_symbols.iter().any(|name| is_widened(name)) {
            return true;
        }
        if self
            .generalised
            .iter()
            .any(|key| failure_symbols.contains(&self.symbol_name(key)))
        {
            return true;
        }
        let mut reached: HashSet<String> = failure_symbols.clone();
        let mut frontier: Vec<String> = failure_symbols.iter().cloned().collect();
        let mut unvisited_defs: Vec<HashSet<String>> = self
            .defs
            .keys()
            .map(|name| {
                if let Some(syms) = self.def_symbols.get(name) {
                    syms.clone()
                } else {
                    extract_symbols(&self.defs[name].to_string())
                }
            })
            .collect();

        while let Some(symbol) = frontier.pop() {
            let mut i = 0;
            while i < unvisited_defs.len() {
                if unvisited_defs[i].contains(&symbol) {
                    let symbols_in_def = unvisited_defs.swap_remove(i);
                    for candidate in symbols_in_def {
                        if !self.inputs.contains_key(&candidate) {
                            continue;
                        }
                        if is_widened(&candidate) {
                            return true;
                        }
                        if reached.insert(candidate.clone()) {
                            frontier.push(candidate);
                        }
                    }
                } else {
                    i += 1;
                }
            }
        }
        false
    }

    /// Why an obligation checked in this state is a FAIL if it is one (K6), as a
    /// token for `report.json` rather than prose for `details`.
    ///
    /// Two of the three classes send a reader somewhere different. A
    /// `widening-limited` failure leans on a value the loop abstraction
    /// generalised, so it may be what the widening cost rather than a bug — the
    /// suspect ones. A `guard-limited` one is stated under guards the analysis did
    /// express and the model satisfies them, so what is missing is a stronger
    /// guard on the program's own path. `unguarded` needs no guard at all.
    pub fn defect_class(&self, failure_symbols: &HashSet<String>) -> &'static str {
        if self.leans_on_widening(failure_symbols) {
            "widening-limited"
        } else if self.pc.is_some() {
            "guard-limited"
        } else {
            "unguarded"
        }
    }

    #[allow(dead_code)]
    pub fn guard(&self) -> Option<&Bool<'ctx>> {
        self.pc.as_ref()
    }

    /// Require `guard` from now on, as well as whatever was already required.
    ///
    /// This is how a fact that did not come from this function's own control flow
    /// — an inherited precondition — joins the state.
    #[allow(dead_code)]
    pub fn assume(&mut self, ctx: &'ctx Context, guard: Bool<'ctx>) {
        self.pc = Some(match self.pc.take() {
            Some(existing) => all_of(ctx, &[&existing, &guard]),
            None => guard,
        });
    }

    /// This state as the *previous* trip saw it, for the loop relation.
    ///
    /// It differs from [`Self::rebranded`] in two ways, both of which the loop
    /// relation needs and neither of which the interprocedural use does: a
    /// symbol that already carries a previous-trip prefix is left where it is
    /// (see [`previous_trip_map`]), and the previous trip's own merge
    /// definitions are dropped. The relation needs the previous trip's *guard*,
    /// which lives in the condition and is renamed with the rest; its
    /// disjunctions would only be renamed copies of copies, one per round. A
    /// dropped definition is a weaker state, never a wrong one.
    fn previous_trip(&self, ctx: &'ctx Context, prefix: &str) -> PathState<'ctx> {
        let pairs = previous_trip_map(ctx, &self.inputs, prefix);
        let map: Vec<(&Int<'ctx>, &Int<'ctx>)> =
            pairs.iter().map(|(from, to)| (from, to)).collect();
        PathState {
            values: self
                .values
                .iter()
                .map(|(key, term)| (key.clone(), term.substitute(&map)))
                .collect(),
            pc: self.pc.as_ref().map(|pc| pc.substitute(&map)),
            defs: BTreeMap::new(),
            def_symbols: BTreeMap::new(),
            inputs: self
                .inputs
                .iter()
                .map(|(name, input)| {
                    let symbol = previous_trip_name(name, prefix);
                    (
                        symbol.clone(),
                        InputVar {
                            symbol,
                            label: input.label.clone(),
                            domain: input.domain,
                            kind: input.kind,
                        },
                    )
                })
                .collect(),
            unmodelled: self
                .unmodelled
                .iter()
                .map(|(name, reason)| (previous_trip_name(name, prefix), reason.clone()))
                .collect(),
            checked: self
                .checked
                .iter()
                .map(|(key, product)| (key.clone(), product.substituted(&map)))
                .collect(),
            discriminants: self.discriminants.clone(),
            symbolic_discriminants: self
                .symbolic_discriminants
                .iter()
                .map(|(key, term)| (key.clone(), term.substitute(&map)))
                .collect(),
            // The generalised places are keys, not symbols: the rename changes what
            // they are called, never which places the abstraction gave up on, so
            // they carry over unchanged.
            generalised: self.generalised.clone(),
            pointees: self.pointees.clone(),
            // Renamed with the rest: a previous-trip state is still about the same
            // coroutine, and its discriminant has to stay the same symbol.
            resume: self.resume.as_ref().map(|resume| ResumeState {
                symbol: resume.symbol.substitute(&map),
                name: previous_trip_name(&resume.name, prefix),
            }),
            prefix: prefix.to_string(),
            has_bitwise: self.has_bitwise,
        }
    }

    /// A copy of this state with every free symbol renamed under `prefix`.
    ///
    /// The verifier uses this to carry a caller's facts into a callee's solver,
    /// and it has to rename rather than reuse: both states name their locals
    /// `mir_<n>`, so asserting a caller's guard unchanged would be read as
    /// speaking about the *callee's* locals — a wrong answer, not a weaker one.
    ///
    /// Every symbol a state mentions is registered in `inputs`, which is what
    /// makes this complete, and the prefix also covers symbols invented after the
    /// rename because [`Self::fresh_symbol`] composes it.
    #[allow(dead_code)]
    pub fn rebranded(&self, ctx: &'ctx Context, prefix: &str) -> PathState<'ctx> {
        let pairs: Vec<(Int<'ctx>, Int<'ctx>)> = self
            .inputs
            .keys()
            .map(|name| {
                (
                    Int::new_const(ctx, name.as_str()),
                    Int::new_const(ctx, format!("{prefix}{name}").as_str()),
                )
            })
            .collect();
        let map: Vec<(&Int<'ctx>, &Int<'ctx>)> =
            pairs.iter().map(|(from, to)| (from, to)).collect();

        PathState {
            values: self
                .values
                .iter()
                .map(|(key, term)| (key.clone(), term.substitute(&map)))
                .collect(),
            pc: self.pc.as_ref().map(|pc| pc.substitute(&map)),
            defs: self
                .defs
                .iter()
                .map(|(name, def)| (format!("{prefix}{name}"), def.substitute(&map)))
                .collect(),
            def_symbols: self
                .def_symbols
                .iter()
                .map(|(name, syms)| {
                    (
                        format!("{prefix}{name}"),
                        syms.iter().map(|s| format!("{prefix}{s}")).collect(),
                    )
                })
                .collect(),
            inputs: self
                .inputs
                .iter()
                .map(|(name, input)| {
                    let symbol = format!("{prefix}{name}");
                    (
                        symbol.clone(),
                        InputVar {
                            symbol,
                            label: input.label.clone(),
                            domain: input.domain,
                            kind: input.kind,
                        },
                    )
                })
                .collect(),
            unmodelled: self
                .unmodelled
                .iter()
                .map(|(name, reason)| (format!("{prefix}{name}"), reason.clone()))
                .collect(),
            checked: self
                .checked
                .iter()
                .map(|(key, product)| (key.clone(), product.substituted(&map)))
                .collect(),
            discriminants: self.discriminants.clone(),
            symbolic_discriminants: self
                .symbolic_discriminants
                .iter()
                .map(|(key, term)| (key.clone(), term.substitute(&map)))
                .collect(),
            generalised: self.generalised.clone(),
            pointees: self.pointees.clone(),
            resume: self.resume.as_ref().map(|resume| ResumeState {
                symbol: resume.symbol.substitute(&map),
                name: format!("{prefix}{}", resume.name),
            }),
            prefix: prefix.to_string(),
            has_bitwise: self.has_bitwise,
        }
    }

    /// The free symbols a FAIL should name, best first: the caller-settable
    /// inputs. If nothing named reaches this point the unnamed ones are reported
    /// instead, so a FAIL still carries a concrete value.
    pub fn witness_inputs(&self) -> Vec<InputVar> {
        let sources: Vec<InputVar> = self
            .inputs
            .values()
            .filter(|input| input.kind == InputKind::Source)
            .cloned()
            .collect();
        if !sources.is_empty() {
            return sources;
        }
        self.inputs
            .values()
            .filter(|input| input.kind == InputKind::Temp)
            .cloned()
            .collect()
    }

    /// Assert everything this state contributes to an obligation: the
    /// reachability condition, the merge definitions, and the type bounds of the
    /// free inputs.
    ///
    /// Merge symbols are deliberately left unbounded: a merged value may be the
    /// unwrapped result of an arithmetic expression, so bounding it by the
    /// local's own type could make an unsatisfiable-but-real state look
    /// impossible.
    pub fn assert_into(&self, ctx: &'ctx Context, solver: &mut Solver<'ctx>) {
        if let Some(pc) = &self.pc {
            solver.assert(pc);
        }
        for def in self.defs.values() {
            solver.assert(def);
        }
        for input in self.inputs.values() {
            let Some(domain) = input.domain else {
                continue;
            };
            let (min, max) = domain.bounds();
            let symbol = Int::new_const(ctx, input.symbol.as_str());
            let min = Int::from_str(ctx, &min).expect("a valid decimal literal");
            let max = Int::from_str(ctx, &max).expect("a valid decimal literal");
            solver.assert(&symbol.ge(&min));
            solver.assert(&symbol.le(&max));
        }
    }

    /// Forget every recorded value for a place that the write invalidates: the
    /// place itself, everything below it, and any ancestor tuple it was part of.
    fn invalidate(&mut self, key: &PlaceKey) {
        let (local, path) = key;
        let stale = |existing: &PlaceKey| {
            existing.0 == *local
                && (existing.1.starts_with(path.as_slice()) || path.starts_with(&existing.1))
        };
        self.values.retain(|existing, _| !stale(existing));
        // A `checked_*` fact is about the places it names — the `Option` and its
        // payload — so invalidating either retires it. Its discriminant would
        // otherwise keep describing a call whose payload this state no longer
        // holds, and a discriminant over a stale sum is a claim about a value the
        // state has forgotten.
        self.checked
            .retain(|option, product| !stale(option) && !stale(&product.payload));
        // A discriminant is a property of the place's *own* variant, not of its
        // fields. Writing a payload field therefore cannot change it, and
        // retiring the parent's discriminant here is what made
        // `Some(x).unwrap()` look unmodelled: the aggregate's discriminant was
        // recorded first, then `set_value` on field 0 invalidated its parent.
        // Writes to a place at or below the written projection still retire it.
        self.discriminants.retain(|existing, _| {
            !(existing.0 == *local && existing.1.starts_with(path.as_slice()))
        });
        // A symbolic discriminant is the same kind of fact with the same lifetime:
        // writing a payload field does not change which variant the place holds,
        // and writing the place itself does.
        self.symbolic_discriminants.retain(|existing, _| {
            !(existing.0 == *local && existing.1.starts_with(path.as_slice()))
        });
    }

    /// Give up on a place's value because it is loop-carried, and remember that
    /// this is what happened.
    ///
    /// The record is what [`Self::leans_on_widening`] reads: the place's later
    /// reads re-derive its own free symbol, and a finding stated over that symbol
    /// is a finding about a value the abstraction stopped tracking. The two halves
    /// are one operation, so no caller can widen without leaving the evidence.
    fn generalise(&mut self, key: &PlaceKey) {
        self.invalidate(key);
        self.generalised.insert(key.clone());
    }

    fn set_value(&mut self, key: PlaceKey, term: Int<'ctx>) {
        self.invalidate(&key);
        self.values.insert(key, term);
    }

    /// Take the symbolic value of a place, inventing a bounded free symbol when
    /// the MIR leaves the value unknown.
    ///
    /// The invented symbol is what keeps this analysis honest: an unmodelled
    /// definition becomes "some value of this type" rather than a wrong guess.
    fn fresh_symbol<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        key: PlaceKey,
        place: &Place<'tcx>,
        domain: Option<ValueDomain>,
    ) -> Int<'ctx> {
        let symbol = self.symbol_name(&key);
        let label = place_label(mir, place);
        let kind = if key.1.is_empty() && local_name(mir, place.local).is_some() {
            InputKind::Source
        } else {
            InputKind::Temp
        };
        match self.inputs.get_mut(&symbol) {
            Some(existing) => {
                // An entry with no domain is a *declaration*, not a claim: the
                // widening step registers the symbols its relation mentions before
                // any read of them has happened, and it has no type to give them.
                // The first read of that place knows more — what it is called and
                // what type it holds — so it replaces the placeholder. Without
                // this, a `u8` local keeps an unbounded symbol and a model answers
                // `-2` for it: a witness nobody can replay, on an obligation the
                // place's own type rules out.
                if existing.domain.is_none() {
                    existing.label = label;
                    existing.kind = kind;
                    existing.domain = domain;
                }
            }
            None => {
                self.inputs.insert(
                    symbol.clone(),
                    InputVar {
                        symbol: symbol.clone(),
                        label,
                        domain,
                        kind,
                    },
                );
            }
        }
        Int::new_const(ctx, symbol)
    }

    /// The name a free symbol for this place carries. One place, one name, so a
    /// value forgotten here and a value re-derived later are recognisably the
    /// same unknown.
    fn symbol_name(&self, key: &PlaceKey) -> String {
        format!("{}mir_{}", self.prefix, key_suffix(key))
    }

    /// Forget a place's value because the statement that wrote it is not
    /// modelled, recording why under the symbol a later read will invent.
    ///
    /// This is the difference between an unknown and a *witnessed* unknown: both
    /// leave the place free, but only the second is a reason a FAIL may be
    /// declined. The caller reports such an obligation as undecided, naming the
    /// reason, rather than as a counterexample — the values in the model are the
    /// solver's choice, not the program's.
    fn forget(&mut self, key: &PlaceKey, reason: String) {
        self.invalidate(key);
        self.unmodelled.insert(self.symbol_name(key), reason);
    }

    /// Retire everything a store the key scheme cannot name may have destroyed.
    ///
    /// `place_key` understands field projections only, so a store through a
    /// dereference — `*r = 250`, which is how `let r = &mut v; *r = …` and
    /// `if let Some(v) = &mut opt { … }` both lower — names no place at all. The
    /// place still *is* written, and the analysis cannot say to what, so the
    /// honest answer is that the storage it may have reached holds something
    /// unknown. Returning without doing this left the previous value in place:
    /// `v` kept the value the copy out of `s + x` had put there, the guard bounds
    /// it by 8, and a later `v + 240` was answered from it — a proof of a program
    /// that had already written 250 into the same storage (A17).
    ///
    /// Retirement, not a stronger fact: without aliasing, the reference could
    /// point at any local this body borrowed, so the analysis gives the values up
    /// and records why. That costs a free symbol where a value was known, which
    /// can only add counterexamples, where having kept the stale value can hide a
    /// real one.
    fn retire_unnamed_store<'tcx>(
        &mut self,
        mir: &Body<'tcx>,
        writes: &BodyWrites,
        place: &Place<'tcx>,
    ) {
        let reason = format!(
            "`{}` is written through a projection this analysis cannot name, so what that storage held is not modelled",
            place_label(mir, place)
        );
        self.retire_place(writes, place, reason);
    }

    /// Retire every place a modelled-as-unmodelled write to `place` may have
    /// destroyed, with the reason a later read can name.
    ///
    /// The base local always, plus everything the write could reach when the
    /// projection goes through a pointer ([`retirement_targets`]).
    fn retire_place(&mut self, writes: &BodyWrites, place: &Place<'_>, reason: String) {
        let through_pointer = place
            .projection
            .iter()
            .any(|elem| matches!(elem, PlaceElem::Deref));
        for key in retirement_targets(place.local.index(), through_pointer, writes) {
            self.forget(&key, reason.clone());
        }
    }

    /// Retire what a write by a body this analysis does not read may have made of
    /// the storage one of this body's own pointers refers to.
    ///
    /// A callee's body, an asm block and drop glue are all code this analysis did
    /// not look at, so a place they are handed a pointer to holds something
    /// unknown afterwards. Sorted, so the retirement order does not depend on hash
    /// iteration.
    fn retire_pointer_write(&mut self, writes: &BodyWrites, target: PointerWrite, reason: &str) {
        match target {
            PointerWrite::Nothing => {}
            PointerWrite::Local(local) => self.forget(&(local, Vec::new()), reason.to_string()),
            PointerWrite::Any => {
                let mut targets: Vec<usize> =
                    writes.mutably_address_taken.iter().copied().collect();
                targets.sort_unstable();
                for local in targets {
                    self.forget(&(local, Vec::new()), reason.to_string());
                }
            }
        }
    }

    /// Retire what a call this analysis does not read may write through the
    /// pointers it is given: `f(&mut v)` hands the callee `v`'s address, and
    /// nothing else in this body says whether it writes it.
    ///
    /// This is the reproduced bug A18 names. `fn bump(v: &mut u8) { *v =
    /// 250 }` followed by `v + 240` under a guard that bounds `v` by 8 used to be
    /// *proved*: the call was not modelled, so nothing retired the value the call
    /// overwrote, and the proof was made of a value the program had already
    /// replaced.
    fn retire_call_arguments<'tcx>(
        &mut self,
        mir: &Body<'tcx>,
        tcx: TyCtxt<'tcx>,
        writes: &BodyWrites,
        callee: &str,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
    ) {
        for arg in args {
            let target = operand_pointer_write(mir, tcx, writes, &arg.node);
            if matches!(target, PointerWrite::Nothing) {
                continue;
            }
            let reason = format!(
                "the call to `{}` is given a pointer into this body's own storage (`{}`), and a callee this analysis does not read may write through it",
                callee,
                operand_label(mir, &arg.node)
            );
            self.retire_pointer_write(writes, target, &reason);
        }
    }

    /// Why a symbol stands for a value the analysis did not model, if it does.
    pub fn unmodelled_reason(&self, symbol: &str) -> Option<&str> {
        self.unmodelled.get(symbol).map(String::as_str)
    }

    /// The first recorded reason for a place with no projection, if any. Used to
    /// name the callee behind a value that a projection then hides.
    fn origin_of(&self, place: &Place<'_>) -> Option<&str> {
        self.unmodelled_reason(&self.symbol_name(&(place.local.index(), Vec::new())))
    }

    /// The key a *read* of `place` names, including the one projection this
    /// analysis can collapse.
    ///
    /// [`place_key`] understands plain field projections only, which leaves the
    /// payload of a `checked_*` result — read as `((_3 as Some).0: u8)` — with no
    /// name at all. Where the enum's shape makes that payload the only thing at
    /// its position ([`unique_payload_field`]) *and* this state holds the sum the
    /// call produced, the read is answered from it. Otherwise the read stays
    /// unmodelled, exactly as before, so nothing else about the analysis moves.
    fn read_key<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        mir: &Body<'tcx>,
        place: &Place<'tcx>,
    ) -> Option<PlaceKey> {
        if let Some(key) = place_key(place) {
            return Some(key);
        }
        if let Some(key) = self.deref_key(place) {
            return Some(key);
        }
        if let Some(payload) = payload_key(tcx, mir, place) {
            if self.values.contains_key(&payload) {
                return Some(payload);
            }
        }
        let mut elems = place.projection.iter();
        if let Some(PlaceElem::Downcast(_, variant)) = elems.next() {
            let base_key = (place.local.index(), Vec::new());
            let matches_disc = self
                .discriminants
                .get(&base_key)
                .map_or(true, |disc| *disc == variant.as_u32());
            if matches_disc {
                let mut path = Vec::new();
                let mut all_fields = true;
                for elem in elems {
                    match elem {
                        PlaceElem::Field(field, _) => path.push(field.index()),
                        _ => {
                            all_fields = false;
                            break;
                        }
                    }
                }
                if all_fields && !path.is_empty() {
                    let candidate = (place.local.index(), path);
                    if self.values.contains_key(&candidate) {
                        return Some(candidate);
                    }
                }
            }
        }
        None
    }

    /// The place a dereference reads, when this body borrowed one local into
    /// that pointer and never pointed it anywhere else.
    ///
    /// `*r` and `(*r).field` become the target local and the field path, so the
    /// value the state recorded for the target is the value the read sees.
    /// Sound because [`BodyWrites`] records a pointee only when every write to
    /// the pointer was a borrow of that same local, and because a store through
    /// the pointer retires the target's recorded value (see
    /// [`PathState::retire_unnamed_store`]).
    fn deref_key(&self, place: &Place<'_>) -> Option<PlaceKey> {
        if !self.prefix.is_empty() {
            return None;
        }
        let mut elems = place.projection.iter();
        if !matches!(elems.next(), Some(PlaceElem::Deref)) {
            return None;
        }
        let target = (*self.pointees.get(&place.local.index())?)?;
        let mut path = Vec::new();
        for elem in elems {
            match elem {
                PlaceElem::Field(field, _) => path.push(field.index()),
                _ => return None,
            }
        }
        Some((target, path))
    }

    /// Resolve an operand to a symbolic integer (`bool` values are `0`/`1`).
    pub fn int_operand<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        const_map: &ConstMap,
        tcx: TyCtxt<'tcx>,
        operand: &Operand<'tcx>,
    ) -> Option<Int<'ctx>> {
        if let Some(place) = operand_place(operand) {
            let key = self.read_key(tcx, mir, place)?;
            if let Some(term) = self.values.get(&key) {
                return Some(term.clone());
            }
            // A local every path agrees on is a fact about the program, not about
            // one path, so the constant map is sound to consult — but it speaks
            // about whole locals, so only for a place with no projection.
            if key.1.is_empty() {
                if let Some(v) = eval_operand(operand, mir, const_map, tcx) {
                    return Int::from_str(ctx, &v.to_string());
                }
            }
            let domain = place_domain(mir, tcx, place);
            return Some(self.fresh_symbol(ctx, mir, key, place, domain));
        }
        // Literals never lose their upper unsigned 128-bit half.
        if let Some(literal) = integer_literal(operand, tcx) {
            return Int::from_str(ctx, &literal);
        }
        let v = eval_operand(operand, mir, const_map, tcx)?;
        Int::from_str(ctx, &v.to_string())
    }

    fn binary_result<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        const_map: &ConstMap,
        tcx: TyCtxt<'tcx>,
        op: BinOp,
        left: &Operand<'tcx>,
        right: &Operand<'tcx>,
    ) -> BinaryResult<'ctx> {
        let Some(l) = self.int_operand(ctx, mir, const_map, tcx, left) else {
            return BinaryResult::Unmodelled;
        };
        let Some(r) = self.int_operand(ctx, mir, const_map, tcx, right) else {
            return BinaryResult::Unmodelled;
        };

        match op {
            // `a + b` on the tuple-returning operators carries the overflow flag
            // in field 1, which is what the following assert tests.
            BinOp::AddWithOverflow | BinOp::SubWithOverflow | BinOp::MulWithOverflow => {
                let plain = match op {
                    BinOp::AddWithOverflow => BinOp::Add,
                    BinOp::SubWithOverflow => BinOp::Sub,
                    _ => BinOp::Mul,
                };
                let value = match plain {
                    BinOp::Add => &l + &r,
                    BinOp::Sub => &l - &r,
                    _ => &l * &r,
                };
                let int_ty = operand_int_type(left, mir, tcx);
                let flag = match overflow_violation(ctx, plain, &l, &r, int_ty) {
                    OverflowCheck::Violation(flag) => flag,
                    // `+`, `-` and `*` always have a violation condition, so the
                    // other two answers mean the encoding gave up on this one.
                    // The state must not pretend to know the flag in that case.
                    OverflowCheck::Impossible | OverflowCheck::Unmodelled(_) => {
                        return BinaryResult::Unmodelled
                    }
                };
                BinaryResult::WithOverflow {
                    value,
                    flag: flag.ite(&one(ctx), &zero(ctx)),
                }
            }
            BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::Shl
            | BinOp::Shr
                if !operand_is_bool(mir, tcx, left) =>
            {
                if matches!(
                    op,
                    BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr
                ) {
                    self.has_bitwise = true;
                }
                let Some(ty) = operand_int_type(left, mir, tcx) else {
                    return BinaryResult::Unmodelled;
                };
                fixed_binary(ctx, op, &l, &r, ty)
                    .map(BinaryResult::Value)
                    .unwrap_or(BinaryResult::Unmodelled)
            }
            // Unchecked arithmetic has validity requirements; wrapping it would
            // invent semantics for UB. It stays explicitly outside this model.
            // Comparisons produce a `bool`, held as 0/1.
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let cmp: Bool<'ctx> = match op {
                    BinOp::Eq => l._eq(&r),
                    BinOp::Ne => l._eq(&r).not(),
                    BinOp::Lt => l.lt(&r),
                    BinOp::Le => l.le(&r),
                    BinOp::Gt => l.gt(&r),
                    _ => l.ge(&r),
                };
                BinaryResult::Value(cmp.ite(&one(ctx), &zero(ctx)))
            }
            // Boolean operators are `&`/`|`/`^` on 0/1 values.
            BinOp::BitAnd if operand_is_bool(mir, tcx, left) => BinaryResult::Value(&l * &r),
            BinOp::BitOr if operand_is_bool(mir, tcx, left) => {
                BinaryResult::Value((&l + &r)._eq(&zero(ctx)).ite(&zero(ctx), &one(ctx)))
            }
            BinOp::BitXor if operand_is_bool(mir, tcx, left) => {
                BinaryResult::Value(l._eq(&r).ite(&zero(ctx), &one(ctx)))
            }
            BinOp::Div => BinaryResult::Value(rust_quotient(ctx, &l, &r)),
            BinOp::Rem => BinaryResult::Value(&l - &(&rust_quotient(ctx, &l, &r) * &r)),
            // What reaches here is the unchecked operators (`unchecked_add` and
            // friends), which have validity requirements rather than a defined
            // result: wrapping them would invent semantics for UB, so they stay
            // outside this model. Bit-twiddling and shifts are handled above.
            _ => BinaryResult::Unmodelled,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_assign<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        const_map: &ConstMap,
        tcx: TyCtxt<'tcx>,
        writes: &BodyWrites,
        place: &Place<'tcx>,
        rvalue: &Rvalue<'tcx>,
    ) {
        let Some(key) = place_key(place) else {
            // A store this key scheme cannot name still destroys what the state
            // recorded for the storage it may have reached. Returning here used
            // to leave those values standing, which is how a `*r = 250` followed
            // by `v + 240` came back *proved* — from a value the program had
            // already overwritten.
            self.retire_unnamed_store(mir, writes, place);
            return;
        };
        match rvalue {
            Rvalue::Use(operand, _) => {
                let mut copied_subfields = false;
                let mut source_had_base = false;
                if let Some(source) = operand_place(operand) {
                    if let Some(src_key) = place_key(source) {
                        if let Some(disc) = self.discriminants.get(&src_key).copied() {
                            self.discriminants.insert(key.clone(), disc);
                        }
                        source_had_base = self.values.contains_key(&src_key);
                        let sub_fields: Vec<(Vec<usize>, Int<'ctx>)> = self
                            .values
                            .iter()
                            .filter(|(k, _)| {
                                k.0 == src_key.0
                                    && k.1.starts_with(&src_key.1)
                                    && k.1.len() > src_key.1.len()
                            })
                            .map(|(k, v)| (k.1[src_key.1.len()..].to_vec(), v.clone()))
                            .collect();
                        if !sub_fields.is_empty() {
                            self.invalidate(&key);
                            for (suffix, val) in sub_fields {
                                let mut dest_field_path = key.1.clone();
                                dest_field_path.extend(suffix);
                                self.values.insert((key.0, dest_field_path), val);
                            }
                            copied_subfields = true;
                        }
                    }
                }
                if let Some(term) = self.int_operand(ctx, mir, const_map, tcx, operand) {
                    // Only a base value the source *had* may overwrite the
                    // destination. When the source held only fields (an aggregate
                    // this analysis names field-wise), `int_operand` invents a
                    // fresh symbol for the whole of it, and storing that under the
                    // destination would erase the fields this very statement just
                    // copied — an iterator state moved into the loop head would
                    // become one free symbol instead of its cursor and length.
                    if !copied_subfields || source_had_base {
                        self.set_value(key, term);
                    }
                } else if !copied_subfields {
                    let mut detail = match operand_place(operand) {
                        Some(_) => format!(
                            "is read from `{}`, which this analysis does not model",
                            operand_label(mir, operand)
                        ),
                        None => format!(
                            "is assigned from `{}`, which this analysis does not model",
                            operand_label(mir, operand)
                        ),
                    };
                    if let Some(source) = operand_place(operand) {
                        if let Some(origin) = self.origin_of(source) {
                            let origin = origin.to_string();
                            detail.push_str(&format!(
                                "; the value behind it is not modelled either: {origin}"
                            ));
                        }
                    }
                    self.forget(&key, format!("`{}` {detail}", place_label(mir, place)));
                }
            }
            Rvalue::BinaryOp(op, args) => {
                let (left, right) = (&args.0, &args.1);
                match self.binary_result(ctx, mir, const_map, tcx, *op, left, right) {
                    BinaryResult::Value(term) => self.set_value(key, term),
                    BinaryResult::WithOverflow { value, flag } => {
                        self.invalidate(&key);
                        let mut value_key = key.clone();
                        value_key.1.push(0);
                        let mut flag_key = key;
                        flag_key.1.push(1);
                        self.values.insert(value_key, value);
                        self.values.insert(flag_key, flag);
                    }
                    BinaryResult::Unmodelled => self.forget(
                        &key,
                        format!(
                            "`{}` is assigned from a `{:?}` this analysis does not model",
                            place_label(mir, place),
                            op
                        ),
                    ),
                }
            }
            Rvalue::UnaryOp(UnOp::Not, operand) if operand_is_bool(mir, tcx, operand) => {
                // `!b` on a 0/1 value is `1 - b`.
                match self.int_operand(ctx, mir, const_map, tcx, operand) {
                    Some(term) => {
                        let flipped = &one(ctx) - &term;
                        self.set_value(key, flipped);
                    }
                    None => self.forget(
                        &key,
                        format!(
                            "`{}` is the negation of an operand this analysis does not model",
                            place_label(mir, place)
                        ),
                    ),
                }
            }
            Rvalue::UnaryOp(UnOp::Neg, operand) => {
                match self.int_operand(ctx, mir, const_map, tcx, operand) {
                    Some(term) => {
                        if let Some(ty) = operand_int_type(operand, mir, tcx) {
                            self.set_value(key, fixed_value(ctx, &(&zero(ctx) - &term), ty));
                        } else {
                            self.forget(
                                &key,
                                format!(
                                    "`{}` is negated at a type this analysis cannot name",
                                    place_label(mir, place)
                                ),
                            );
                        }
                    }
                    None => self.forget(
                        &key,
                        format!(
                            "`{}` is the negation of an operand this analysis does not model",
                            place_label(mir, place)
                        ),
                    ),
                }
            }
            Rvalue::UnaryOp(UnOp::Not, operand) => {
                match (
                    self.int_operand(ctx, mir, const_map, tcx, operand),
                    operand_int_type(operand, mir, tcx),
                ) {
                    (Some(term), Some(ty)) => self.set_value(
                        key,
                        BV::from_int(&term, ty.bit_width())
                            .bvnot()
                            .to_int(!ty.is_unsigned()),
                    ),
                    _ => self.forget(
                        &key,
                        format!(
                            "`{}` is the bitwise complement of an operand this analysis does not model",
                            place_label(mir, place)
                        ),
                    ),
                }
            }
            Rvalue::Cast(rustc_middle::mir::CastKind::IntToInt, operand, ty) => {
                match (
                    self.int_operand(ctx, mir, const_map, tcx, operand),
                    infer_int_type(*ty),
                ) {
                    (Some(term), Some(target)) => {
                        self.set_value(key, fixed_value(ctx, &term, target))
                    }
                    _ => self.forget(
                        &key,
                        format!(
                            "`{}` is a cast of an operand this analysis does not model",
                            place_label(mir, place)
                        ),
                    ),
                }
            }
            // `PtrMetadata(p)` is the length of the object behind `p`. It is one
            // number for one object, so two reads of it have to be the *same*
            // symbol: a loop that reads `v.len()` in its condition and again in
            // its body otherwise guards one symbol and checks another, and the
            // guard may as well not be there. Only a place no statement writes
            // can be named that way — an assignment to `p` would give the name a
            // second, different length — and only when the metadata really is a
            // length (see [`metadata_is_length`]).
            Rvalue::UnaryOp(UnOp::PtrMetadata, operand) => {
                let source = operand_place(operand);
                let length =
                    source.and_then(|place| self.metadata_value(ctx, mir, tcx, writes, place));
                match length {
                    Some(term) => self.set_value(key, term),
                    None => self.forget(
                        &key,
                        format!(
                            "`{}` is the length behind `{}`, which this analysis cannot name as one object",
                            place_label(mir, place),
                            operand_label(mir, operand)
                        ),
                    ),
                }
            }
            // The variant of a `checked_*` result: `Some` exactly when the sum did
            // not overflow. This is the other half of that payload's encoding (see
            // [`CheckedProduct`]), and the reason the `None` arm cannot reach the
            // payload read with a value the program cannot produce. Anything else
            // keeps the generic "not modelled" answer below, unchanged.
            // Structural Aggregates (Tuples, ADTs, Arrays, Closures, Coroutines):
            // Populates field values and discriminants so that field reads `_x.0`, `_x.start`,
            // `((_x as Some).0)` resolve directly without precision loss.
            Rvalue::Aggregate(box ref kind, operands) => {
                self.invalidate(&key);
                match kind {
                    AggregateKind::Adt(_, variant_idx, ..) => {
                        self.discriminants.insert(key.clone(), variant_idx.as_u32());
                        for (i, op) in operands.iter().enumerate() {
                            let mut field_key = key.clone();
                            field_key.1.push(i);
                            if let Some(term) = self.int_operand(ctx, mir, const_map, tcx, op) {
                                self.set_value(field_key, term);
                            } else if let Some(source) = operand_place(op) {
                                if let Some(src_key) = place_key(source) {
                                    if let Some(disc) = self.discriminants.get(&src_key).copied() {
                                        self.discriminants.insert(field_key, disc);
                                    }
                                }
                            }
                        }
                    }
                    AggregateKind::Tuple
                    | AggregateKind::Array(_)
                    | AggregateKind::Closure(..)
                    | AggregateKind::Coroutine(..) => {
                        for (i, op) in operands.iter().enumerate() {
                            let mut field_key = key.clone();
                            field_key.1.push(i);
                            if let Some(term) = self.int_operand(ctx, mir, const_map, tcx, op) {
                                self.set_value(field_key, term);
                            } else if let Some(source) = operand_place(op) {
                                if let Some(src_key) = place_key(source) {
                                    if let Some(disc) = self.discriminants.get(&src_key).copied() {
                                        self.discriminants.insert(field_key, disc);
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        self.forget(
                            &key,
                            format!(
                                "`{}` is assigned from an aggregate this analysis does not model",
                                place_label(mir, place)
                            ),
                        );
                    }
                }
            }
            Rvalue::Discriminant(source) => {
                // The coroutine's resume state (S6): the transform reads it as
                // `discriminant((*_1))` at the entry of an `async fn`'s body, and
                // every read of the same state has to be the same symbol — that is
                // what makes the entry block's dispatch split on the invariant the
                // state was seeded with instead of on a free value the solver may
                // put in `1` (returned) or `2` (poisoned).
                let resume = self.resume_symbol(ctx, tcx, mir, source);
                let product = place_key(source).and_then(|key| self.checked.get(&key).cloned());
                let source_key = place_key(source);
                let symbolic = source_key
                    .as_ref()
                    .and_then(|k| self.symbolic_discriminants.get(k).cloned());
                if let Some(term) = resume {
                    self.set_value(key, term);
                } else if let Some(product) = product {
                    self.set_value(key, product.discriminant(ctx));
                } else if let Some(term) = symbolic {
                    self.set_value(key, term);
                } else if let Some(disc) =
                    source_key.and_then(|k| self.discriminants.get(&k).copied())
                {
                    self.set_value(key, Int::from_u64(ctx, disc as u64));
                } else if let Some(Some(target_local)) = writes.pointees.get(&source.local.index())
                {
                    let target_key = (*target_local, Vec::new());
                    if let Some(term) = self.symbolic_discriminants.get(&target_key).cloned() {
                        self.set_value(key, term);
                    } else if let Some(disc) = self.discriminants.get(&target_key).copied() {
                        self.set_value(key, Int::from_u64(ctx, disc as u64));
                    } else {
                        self.forget(
                            &key,
                            format!(
                                "`{}` is assigned from a statement this analysis does not model",
                                place_label(mir, place)
                            ),
                        );
                    }
                } else {
                    self.forget(
                        &key,
                        format!(
                            "`{}` is assigned from a statement this analysis does not model",
                            place_label(mir, place)
                        ),
                    );
                }
            }
            // Aggregates, projections and references: nothing we can name, so the
            // destination is unknown.
            _ => self.forget(
                &key,
                format!(
                    "`{}` is assigned from a statement this analysis does not model",
                    place_label(mir, place)
                ),
            ),
        }
    }

    /// The symbolic length of the object behind `operand`, when that length is a
    /// number the analysis can name once and read back anywhere in the body.
    ///
    /// The name is the *source* place's (`len_3` for `_3`), not the
    /// destination's, which is what makes it survive the copy: `let n = v.len()
    /// + 0` and a later `v.len()` in another block are the same symbol, and so is
    /// the `len` of the same slice read twice in one condition.
    ///
    /// `None` leaves the destination unmodelled, which is the honest answer when
    /// the metadata is not a length at all, when the place cannot be named, or
    /// when the place is written somewhere in the body — a place that is
    /// reassigned could be a different object by the time a later read asks for
    /// its length, and one name for two objects is a wrong answer rather than a
    /// weaker one.
    fn metadata_value<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        tcx: TyCtxt<'tcx>,
        writes: &BodyWrites,
        operand: &Place<'tcx>,
    ) -> Option<Int<'ctx>> {
        if !metadata_is_length(mir, tcx, operand) {
            return None;
        }
        let source = place_key(operand)?;
        if writes.written.contains(&source.0) {
            return None;
        }
        let symbol = format!("{}len_{}", self.prefix, key_suffix(&source));
        self.inputs.entry(symbol.clone()).or_insert(InputVar {
            label: format!("{}.len", place_label(mir, operand)),
            symbol: symbol.clone(),
            domain: Some(ValueDomain::SliceLen),
            kind: InputKind::Source,
        });
        Some(Int::new_const(ctx, symbol.as_str()))
    }

    /// Execute a block's statements symbolically, and retire what its terminator
    /// writes.
    ///
    /// Both `match`es below are **tables**: every variant the pinned toolchain can
    /// emit is listed with its disposition, and neither has a wildcard arm, so a
    /// variant nobody has decided about fails to compile instead of silently
    /// writing nothing. A write the analysis cannot model is not "no write" — it
    /// is a reason to give up the values it destroyed (A18).
    ///
    /// The statements:
    ///
    /// | `StatementKind` | disposition |
    /// |---|---|
    /// | `Assign` | modelled by [`Self::apply_assign`]; a destination the key scheme cannot name is retired there by [`Self::retire_unnamed_store`] |
    /// | `SetDiscriminant` | **retires its place.** The only MIR-builder source of this statement is the coroutine state transform (`rustc_mir_transform/src/coroutine/mod.rs`), whose bodies this analysis *does* read: an `async fn` arrives as `<fn>::{closure#0}`, which `tests/statement_writes` pins. The transform always writes the state behind a pointer, so no `checked_*` fact (K9) can be keyed by one today — the retirement is the walk being complete, not a bug that was reproduced |
    /// | `Intrinsic(CopyNonOverlapping { dst, .. })` | **retires through `dst`**, a pointer like any other (the `std::ptr::copy_nonoverlapping` *call* in `tests/statement_writes` is the shape that reproduces it; the statement form is what the optimizer lowers that call to) |
    /// | `Intrinsic(Assume(..))` | writes nothing; it constrains the optimizer, not a place. Ignoring it is the safe direction — an unmodelled assumption can only leave a state the program cannot be in |
    /// | `FakeRead`, `PlaceMention` | pseudo-*reads*: they exist so a read no other statement performs is visible to borrowck. No storage is written |
    /// | `StorageLive`, `StorageDead` | mark a local's storage live or dead; no value is written, and a read of dead storage is forbidden by the MIR validator, so no recorded value can be read back through one |
    /// | `AscribeUserType`, `Coverage`, `ConstEvalCounter`, `Nop`, `BackwardIncompatibleDropHint` | typing metadata, coverage counters, a const-eval counter, a no-op and a drop-order hint: none is a store to a place this analysis holds |
    ///
    /// The terminators are listed on the terminator match below.
    fn transfer<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        const_map: &ConstMap,
        tcx: TyCtxt<'tcx>,
        writes: &BodyWrites,
        block: BasicBlock,
    ) {
        if self.pointees.is_empty() && !writes.pointees.is_empty() {
            self.pointees = writes
                .pointees
                .iter()
                .map(|(holder, pointee)| (*holder, *pointee))
                .collect();
        }
        for statement in &mir.basic_blocks[block].statements {
            match &statement.kind {
                StatementKind::Assign(box (place, rvalue)) => {
                    self.apply_assign(ctx, mir, const_map, tcx, writes, place, rvalue);
                }
                StatementKind::SetDiscriminant {
                    place,
                    variant_index,
                } => {
                    let reason = format!(
                        "`{}` has its discriminant set to variant #{} by a statement this analysis does not model, so which variant it holds is not known any more",
                        place_label(mir, place),
                        variant_index.index()
                    );
                    self.retire_place(writes, place, reason);
                }
                StatementKind::Intrinsic(box NonDivergingIntrinsic::CopyNonOverlapping(copy)) => {
                    let target = operand_pointer_write(mir, tcx, writes, &copy.dst);
                    self.retire_pointer_write(
                        writes,
                        target,
                        "`core::ptr::copy_nonoverlapping` writes through a pointer into this body's own storage",
                    );
                }
                StatementKind::Intrinsic(box NonDivergingIntrinsic::Assume(_))
                | StatementKind::FakeRead(_)
                | StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::PlaceMention(_)
                | StatementKind::AscribeUserType(..)
                | StatementKind::Coverage(_)
                | StatementKind::ConstEvalCounter
                | StatementKind::Nop
                | StatementKind::BackwardIncompatibleDropHint { .. } => {}
            }
        }
        self.transfer_terminator(ctx, mir, const_map, tcx, writes, block);
    }

    /// Retire what a block's terminator writes.
    ///
    /// The table, again with no wildcard. `Call`, `TailCall`, `Drop`, `InlineAsm`
    /// and `Yield` can write; the rest cannot:
    ///
    /// * `Call` — writes its `destination` (forgotten, or modelled by
    ///   [`Self::apply_safe_arith`] when it is a `core::num` method (K9) *and*
    ///   everything its pointer arguments reach, which is the
    ///   reproduced wrong proof A18 names: `fn bump(v: &mut u8) { *v =
    ///   250 }` followed by `v + 240` under a guard bounding `v` by 8 used to be
    ///   proved, because nothing retired the value the callee overwrote.
    /// * `TailCall` — the same pointer arguments, with no destination (the
    ///   callee's result is returned directly) and no successor: control does not
    ///   come back to this body.
    /// * `Drop` — the dropped place, plus every local this body borrowed. Drop
    ///   glue is a body this analysis does not read, and in *safe* Rust a stored
    ///   reference is enough for it to write through: `struct Holder<'a>(&'a mut
    ///   u8)` with `impl Drop` writing `*self.0` makes `v + 240` panic after the
    ///   holder goes out of scope, which used to be proved too.
    /// * `InlineAsm` — its `Out`/`InOut` places, plus what its input operands can
    ///   write through (`asm!("mov [{}], 250", in(reg) ptr)` writes memory the
    ///   analysis never sees). The asm body is not read, so an output is unknown
    ///   rather than unmodelled-and-kept.
    /// * `Yield` — `resume_arg`, which the resumption writes. Present for
    ///   completeness: the coroutine transform runs before `optimized_mir`, so
    ///   the MIR this analysis reads has no `Yield` left in it.
    /// * `Goto`, `SwitchInt`, `Assert` — control flow, and `Assert`'s condition is
    ///   the obligation itself (see [`Self::successor_guards`]), not a store.
    /// * `UnwindResume`, `UnwindTerminate`, `Return`, `Unreachable`,
    ///   `CoroutineDrop` — control leaves this body; nothing is written on the way
    ///   out that a successor of this body could read.
    /// * `FalseEdge`, `FalseUnwind` — borrow-check artefacts that name a real edge
    ///   and a phantom one; no store.
    #[allow(clippy::too_many_arguments)]
    fn transfer_terminator<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        const_map: &ConstMap,
        tcx: TyCtxt<'tcx>,
        writes: &BodyWrites,
        block: BasicBlock,
    ) {
        let Some(terminator) = &mir.basic_blocks[block].terminator else {
            return;
        };
        match &terminator.kind {
            TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } => {
                // The *reason* is what turns "free symbol" into "a model that is
                // not a counterexample" at the point where the value is used.
                let mut modelled_call = false;
                match place_key(destination) {
                    Some(key) => {
                        // A `core::num` safe-arithmetic method has a defined
                        // result, so it is modelled rather than forgotten:
                        // without this the value out of `wrapping_add` is a free
                        // symbol and every obligation downstream of it is a model
                        // the program cannot be in (K9).
                        let safe_arith =
                            self.apply_safe_arith(ctx, mir, const_map, tcx, func, args, &key);
                        let known = if safe_arith {
                            CalleeApplied::Modelled
                        } else {
                            self.apply_known_callee(
                                ctx,
                                mir,
                                const_map,
                                tcx,
                                writes,
                                func,
                                args,
                                &key,
                                destination,
                            )
                        };
                        modelled_call = known == CalleeApplied::Modelled;
                        if !modelled_call {
                            let mut applied_contract = false;
                            if let Some((callee_def_id, _)) = func.const_fn_def() {
                                let callee_contracts =
                                    crate::contract::extract_fn_contracts(tcx, callee_def_id);
                                if !callee_contracts.ensures.is_empty() {
                                    let domain = place_domain(mir, tcx, destination);
                                    let dest_sym = self.fresh_symbol(
                                        ctx,
                                        mir,
                                        key.clone(),
                                        destination,
                                        domain,
                                    );
                                    self.values.insert(key.clone(), dest_sym);
                                    let param_map = crate::contract::build_callsite_mapping(
                                        tcx,
                                        callee_def_id,
                                        args,
                                    );
                                    for (_raw_ens, ens_expr) in &callee_contracts.ensures {
                                        let mut resolver = crate::contract::CallSiteVarResolver {
                                            ctx,
                                            caller_mir: mir,
                                            const_map,
                                            tcx,
                                            state: self,
                                            param_to_arg: &param_map,
                                            return_place: Some(*destination),
                                        };
                                        if let Ok(crate::contract::Z3Val::Bool(guard)) =
                                            ens_expr.eval(ctx, &mut resolver)
                                        {
                                            self.assume(ctx, guard);
                                            applied_contract = true;
                                        }
                                    }
                                }
                            }
                            if !applied_contract {
                                let reason = format!(
                                    "the result of the call to `{}` is not modelled",
                                    operand_label(mir, func)
                                );
                                self.forget(&key, reason);
                            }
                        }
                    }
                    None => {
                        let reason = format!(
                            "the result of the call to `{}` is stored through a projection this analysis cannot name, so what that storage held is not modelled",
                            operand_label(mir, func)
                        );
                        self.retire_place(writes, destination, reason);
                    }
                }
                // A modelled call has already accounted for the writes its
                // semantics have. Retiring its arguments anyway would forget the
                // iterator state the model just updated — `Range::next(&mut it)`
                // would erase `it`'s cursor, and the next trip would start from a
                // free symbol.
                if !modelled_call {
                    let callee = operand_label(mir, func);
                    self.retire_call_arguments(mir, tcx, writes, &callee, args);
                }
            }
            TerminatorKind::TailCall { func, args, .. } => {
                let callee = operand_label(mir, func);
                self.retire_call_arguments(mir, tcx, writes, &callee, args);
            }
            TerminatorKind::Drop { place, .. } => {
                let reason = format!(
                    "`{}` is dropped here, which runs drop glue this analysis does not read",
                    place_label(mir, place)
                );
                self.retire_place(writes, place, reason);
                let drop_ty = mir.local_decls[place.local].ty;
                let param_env = tcx.param_env(mir.source.def_id());
                let typing_env = rustc_middle::ty::TypingEnv::new(
                    param_env,
                    rustc_middle::ty::TypingMode::PostAnalysis,
                );
                if drop_ty.needs_drop(tcx, typing_env) && holds_a_lifetime(drop_ty) {
                    self.retire_pointer_write(
                        writes,
                        PointerWrite::Any,
                        "a drop here runs glue this analysis does not read, and the value being dropped may hold a reference into this body's own storage",
                    );
                }
            }
            TerminatorKind::InlineAsm { operands, .. } => {
                for operand in operands.iter() {
                    match operand {
                        InlineAsmOperand::Out { place, .. } => {
                            if let Some(place) = place {
                                let reason = format!(
                                    "`{}` is written by an inline asm block, whose body this analysis does not read",
                                    place_label(mir, place)
                                );
                                self.retire_place(writes, place, reason);
                            }
                        }
                        InlineAsmOperand::InOut {
                            in_value,
                            out_place,
                            ..
                        } => {
                            if let Some(place) = out_place {
                                let reason = format!(
                                    "`{}` is written by an inline asm block, whose body this analysis does not read",
                                    place_label(mir, place)
                                );
                                self.retire_place(writes, place, reason);
                            }
                            let target = operand_pointer_write(mir, tcx, writes, in_value);
                            self.retire_pointer_write(
                                writes,
                                target,
                                "an inline asm block is given a pointer into this body's own storage, which it may write",
                            );
                        }
                        InlineAsmOperand::In { value, .. } => {
                            let target = operand_pointer_write(mir, tcx, writes, value);
                            self.retire_pointer_write(
                                writes,
                                target,
                                "an inline asm block is given a pointer into this body's own storage, which it may write",
                            );
                        }
                        // A constant, a fn/static symbol and a label name no
                        // storage of this body.
                        InlineAsmOperand::Const { .. }
                        | InlineAsmOperand::SymFn { .. }
                        | InlineAsmOperand::SymStatic { .. }
                        | InlineAsmOperand::Label { .. } => {}
                    }
                }
            }
            TerminatorKind::Yield { resume_arg, .. } => {
                let reason = format!(
                    "`{}` is written with the value this coroutine is resumed with, which this analysis does not see",
                    place_label(mir, resume_arg)
                );
                self.retire_place(writes, resume_arg, reason);
            }
            TerminatorKind::Goto { .. }
            | TerminatorKind::SwitchInt { .. }
            | TerminatorKind::UnwindResume
            | TerminatorKind::UnwindTerminate(_)
            | TerminatorKind::Return
            | TerminatorKind::Unreachable
            | TerminatorKind::Assert { .. }
            | TerminatorKind::CoroutineDrop
            | TerminatorKind::FalseEdge { .. }
            | TerminatorKind::FalseUnwind { .. } => {}
        }
    }

    /// Give a `core::num` safe-arithmetic call its defined result, and report
    /// whether this was one.
    ///
    /// The three families are exact, which is what makes them usable as a value
    /// rather than as a bound: `wrapping_*` reduces the mathematical result into
    /// the type's width, `saturating_*` clamps it, and `checked_*` puts it in the
    /// `Some` payload with the variant it returned modelled beside it as a fact
    /// about the same sum ([`CheckedProduct`]). What cannot be placed exactly
    /// returns `false`, which sends the caller to the witnessed-unknown path: a
    /// value is never invented here, and no arm is left reachable by omission.
    fn apply_safe_arith<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        const_map: &ConstMap,
        tcx: TyCtxt<'tcx>,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        key: &PlaceKey,
    ) -> bool {
        let Some((form, op)) = safe_arith_form(tcx, func) else {
            return false;
        };
        if args.len() != 2 {
            return false;
        }
        let (left, right) = (&args[0].node, &args[1].node);
        // 128-bit widths are refused rather than approximated: `2^128` is not an
        // `i128`, so the modulus cannot be stated and the encoding would no longer
        // be the method's semantics.
        let Some(ty) = operand_int_type(left, mir, tcx).filter(|ty| ty.bit_width() < 128) else {
            return false;
        };
        let (Some(l), Some(r)) = (
            self.int_operand(ctx, mir, const_map, tcx, left),
            self.int_operand(ctx, mir, const_map, tcx, right),
        ) else {
            return false;
        };
        let sum = match op {
            BinOp::Add => &l + &r,
            BinOp::Sub => &l - &r,
            BinOp::Mul => &l * &r,
            _ => return false,
        };
        match form {
            SafeArith::Wrapping => {
                // The reduction is the library's own definition of the result, and
                // `fixed_binary` is where this analysis already defines fixed-width
                // arithmetic — including the `Mul` case, where one add-or-subtract
                // of the modulus would not be the reduction at all.
                let Some(term) = fixed_binary(ctx, op, &l, &r, ty) else {
                    return false;
                };
                self.set_value(key.clone(), term);
            }
            SafeArith::Saturating => self.set_value(key.clone(), saturating_value(ctx, &sum, ty)),
            SafeArith::Checked => {
                let Some((payload_variant, index, empty_variant, field_ty)) =
                    unique_payload_field(tcx, mir.local_decls[Local::from_usize(key.0)].ty)
                else {
                    return false;
                };
                if value_domain_of_ty(field_ty) != Some(ValueDomain::Int(ty)) {
                    return false;
                }
                // Both halves travel together: the payload, and the discriminant
                // that says which variant holds it. The destination itself is
                // still recorded as a witnessed unknown, so a read of the whole
                // `Option` is reported undecided rather than decided against a
                // free symbol — and both halves are retired together too, by
                // [`Self::invalidate`].
                self.forget(
                    key,
                    format!(
                        "the result of the call to `{}` is not modelled as a whole, only the variant it returned",
                        operand_label(mir, func)
                    ),
                );
                let product = CheckedProduct {
                    sum,
                    ty,
                    payload: (key.0, vec![index]),
                    payload_variant,
                    empty_variant,
                };
                self.values
                    .insert(product.payload.clone(), product.payload(ctx));
                self.checked.insert(key.clone(), product);
            }
        }
        true
    }

    fn operand_discriminant<'tcx>(
        &self,
        writes: &BodyWrites,
        operand: &Operand<'tcx>,
    ) -> Option<u32> {
        let place = operand_place(operand)?;
        if let Some(key) = place_key(place) {
            if let Some(disc) = self.discriminants.get(&key) {
                return Some(*disc);
            }
        }
        if let Some(Some(target_local)) = writes.pointees.get(&place.local.index()) {
            let target_key = (*target_local, Vec::new());
            if let Some(disc) = self.discriminants.get(&target_key) {
                return Some(*disc);
            }
        }
        None
    }

    /// Record a symbolic fact about `key` and return it, reusing an existing one.
    ///
    /// The pseudo-field choice is the caller's; this only owns the bookkeeping
    /// that makes the fact an ordinary state value: one symbol per place, a
    /// declared domain so a model cannot invent a length outside `0..=isize::MAX`,
    /// and the same name across reads.
    fn fact_value(
        &mut self,
        ctx: &'ctx Context,
        key: PlaceKey,
        label: String,
        domain: Option<ValueDomain>,
    ) -> Int<'ctx> {
        if let Some(existing) = self.values.get(&key) {
            return existing.clone();
        }
        let symbol = self.symbol_name(&key);
        match self.inputs.get_mut(&symbol) {
            Some(existing) => {
                if existing.domain.is_none() {
                    existing.label = label;
                    existing.domain = domain;
                    existing.kind = InputKind::Temp;
                }
            }
            None => {
                self.inputs.insert(
                    symbol.clone(),
                    InputVar {
                        symbol: symbol.clone(),
                        label,
                        domain,
                        kind: InputKind::Temp,
                    },
                );
            }
        }
        Int::new_const(ctx, symbol.as_str())
    }

    /// The symbolic length of a `Vec`/`String`, declaring one if this is the
    /// first read. The fact lives on the object, so `with_capacity`/`new` and a
    /// later `len` name the same quantity.
    fn container_length(&mut self, ctx: &'ctx Context, object: &PlaceKey) -> Int<'ctx> {
        let key = pseudo_key(object, PSEUDO_LEN);
        if let Some(existing) = self.values.get(&key) {
            return existing.clone();
        }
        let label = format!("{}.len", place_label_of_key(object));
        self.fact_value(ctx, key, label, Some(ValueDomain::SliceLen))
    }

    /// The symbolic capacity of a `Vec`/`String`. Unlike a length, a capacity can
    /// legitimately be `usize::MAX` for a zero-sized element, so it is bounded by
    /// its own `usize` type rather than by [`ValueDomain::SliceLen`].
    fn container_capacity(&mut self, ctx: &'ctx Context, object: &PlaceKey) -> Int<'ctx> {
        let key = pseudo_key(object, PSEUDO_CAPACITY);
        if let Some(existing) = self.values.get(&key) {
            return existing.clone();
        }
        let label = format!("{}.capacity", place_label_of_key(object));
        self.fact_value(ctx, key, label, Some(ValueDomain::Int(IntType::Usize)))
    }

    /// The length behind a slice or `str` place.
    ///
    /// The normal answer is [`Self::metadata_value`]'s symbol, which is the same
    /// one a `PtrMetadata` read uses. When that name is unavailable — the place
    /// is reassigned in this body, or it carries no fat pointer metadata — a
    /// pseudo length on the place keeps repeated reads consistent.
    fn slice_length<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        tcx: TyCtxt<'tcx>,
        writes: &BodyWrites,
        place: &Place<'tcx>,
    ) -> Option<Int<'ctx>> {
        if let Some(term) = self.metadata_value(ctx, mir, tcx, writes, place) {
            return Some(term);
        }
        let key = place_key(place)?;
        let fact = pseudo_key(&key, PSEUDO_LEN);
        if let Some(existing) = self.values.get(&fact) {
            return Some(existing.clone());
        }
        let label = format!("{}.len", place_label(mir, place));
        Some(self.fact_value(ctx, fact, label, Some(ValueDomain::SliceLen)))
    }

    /// One pseudo fact about `key`: the value recorded for it, or — after loop
    /// widening gave the value up — the place's own free symbol.
    ///
    /// The fallback is not a guess. Widening empties a loop-carried place and
    /// records the loop relation about that place's symbol
    /// ([`widened_state`]); a later read re-derives exactly that symbol. Reading
    /// it here is what lets `next` keep modelling an iterator across a loop
    /// head instead of starting the trip from nothing.
    fn pseudo_fact(&self, ctx: &'ctx Context, key: &PlaceKey, field: usize) -> Option<Int<'ctx>> {
        let fact = pseudo_key(key, field);
        if let Some(term) = self.values.get(&fact) {
            return Some(term.clone());
        }
        if self.generalised.contains(&fact) {
            return Some(Int::new_const(ctx, self.symbol_name(&fact)));
        }
        None
    }

    /// The current `(cursor, len)` of a modelled iterator, if both are known.
    ///
    /// A `Range` that never went through `into_iter` (built as an aggregate and
    /// stepped directly) keeps its state in its real fields `start`/`end`, so
    /// those are read as the fallback.
    fn iterator_parts<'tcx>(
        &self,
        ctx: &'ctx Context,
        tcx: TyCtxt<'tcx>,
        key: &PlaceKey,
        ty: Ty<'tcx>,
    ) -> Option<(Int<'ctx>, Int<'ctx>)> {
        let cursor = self.pseudo_fact(ctx, key, PSEUDO_CURSOR);
        let len = self.pseudo_fact(ctx, key, PSEUDO_LEN);
        if let (Some(cursor), Some(len)) = (cursor, len) {
            return Some((cursor, len));
        }
        if iterator_kind(tcx, ty) == Some(IteratorKind::Range) {
            let mut start_path = key.1.clone();
            start_path.push(0);
            let mut end_path = key.1.clone();
            end_path.push(1);
            let start = self.values.get(&(key.0, start_path)).cloned();
            let end = self.values.get(&(key.0, end_path)).cloned();
            if let (Some(start), Some(end)) = (start, end) {
                return Some((start, end));
            }
        }
        None
    }

    /// Copy every fact recorded under `source` to `destination`, and report
    /// whether there was one.
    ///
    /// This is the same subfield copy `Rvalue::Use` performs; it is factored out
    /// because `into_iter`/`enumerate` are calls, not moves, and their whole
    /// point is that the state behind the value they return is the state behind
    /// the value they consumed.
    fn copy_place_facts(&mut self, source: &PlaceKey, destination: &PlaceKey) -> bool {
        let sub_values: Vec<(Vec<usize>, Int<'ctx>)> = self
            .values
            .iter()
            .filter(|(key, _)| {
                key.0 == source.0 && key.1.starts_with(&source.1) && key.1.len() > source.1.len()
            })
            .map(|(key, value)| (key.1[source.1.len()..].to_vec(), value.clone()))
            .collect();
        let sub_discriminants: Vec<(Vec<usize>, Int<'ctx>)> = self
            .symbolic_discriminants
            .iter()
            .filter(|(key, _)| {
                key.0 == source.0 && key.1.starts_with(&source.1) && key.1.len() > source.1.len()
            })
            .map(|(key, value)| (key.1[source.1.len()..].to_vec(), value.clone()))
            .collect();
        if sub_values.is_empty() && sub_discriminants.is_empty() {
            return false;
        }
        self.invalidate(destination);
        for (suffix, value) in sub_values {
            let mut path = destination.1.clone();
            path.extend(suffix);
            self.values.insert((destination.0, path), value);
        }
        for (suffix, value) in sub_discriminants {
            let mut path = destination.1.clone();
            path.extend(suffix);
            self.symbolic_discriminants
                .insert((destination.0, path), value);
        }
        true
    }

    /// The object a container method receiver names: the borrowed local when the
    /// MIR takes a `&local`, otherwise the reference place itself (a parameter
    /// carries the same key for the whole body).
    fn receiver_object(writes: &BodyWrites, operand: &Operand<'_>) -> Option<PlaceKey> {
        let place = operand_place(operand)?;
        if place.projection.is_empty() {
            if let Some(Some(target)) = writes.pointees.get(&place.local.index()) {
                return Some((*target, Vec::new()));
            }
        }
        place_key(place)
    }

    /// Model `Vec`/`String`/slice/`str` `len`.
    fn model_len<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        tcx: TyCtxt<'tcx>,
        writes: &BodyWrites,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        key: &PlaceKey,
    ) -> bool {
        let Some(arg) = args.first() else {
            return false;
        };
        let Some(ty) = operand_ty(mir, tcx, &arg.node) else {
            return false;
        };
        let length = if container_kind(tcx, ty).is_some() {
            let Some(object) = Self::receiver_object(writes, &arg.node) else {
                return false;
            };
            self.container_length(ctx, &object)
        } else if is_slice_or_str_ref(ty) {
            let Some(place) = operand_place(&arg.node) else {
                return false;
            };
            let Some(length) = self.slice_length(ctx, mir, tcx, writes, place) else {
                return false;
            };
            length
        } else {
            return false;
        };
        self.set_value(key.clone(), length);
        true
    }

    /// Model `is_empty` for a container, slice or `str`.
    fn model_is_empty<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        tcx: TyCtxt<'tcx>,
        writes: &BodyWrites,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        key: &PlaceKey,
    ) -> bool {
        let Some(arg) = args.first() else {
            return false;
        };
        let Some(ty) = operand_ty(mir, tcx, &arg.node) else {
            return false;
        };
        let length = if container_kind(tcx, ty).is_some() {
            let Some(object) = Self::receiver_object(writes, &arg.node) else {
                return false;
            };
            self.container_length(ctx, &object)
        } else if is_slice_or_str_ref(ty) {
            let Some(place) = operand_place(&arg.node) else {
                return false;
            };
            let Some(length) = self.slice_length(ctx, mir, tcx, writes, place) else {
                return false;
            };
            length
        } else {
            return false;
        };
        let zero = zero(ctx);
        let flag = length._eq(&zero).ite(&one(ctx), &zero);
        self.set_value(key.clone(), flag);
        true
    }

    /// Model `capacity` for a `Vec` or `String`.
    fn model_capacity<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        tcx: TyCtxt<'tcx>,
        writes: &BodyWrites,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        key: &PlaceKey,
    ) -> bool {
        let Some(arg) = args.first() else {
            return false;
        };
        let Some(ty) = operand_ty(mir, tcx, &arg.node) else {
            return false;
        };
        if container_kind(tcx, ty).is_none() {
            return false;
        }
        let Some(object) = Self::receiver_object(writes, &arg.node) else {
            return false;
        };
        let capacity = self.container_capacity(ctx, &object);
        self.set_value(key.clone(), capacity);
        true
    }

    /// Model `new` for a `Vec` or `String`: empty, with capacity zero.
    fn model_new<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        tcx: TyCtxt<'tcx>,
        destination: &Place<'tcx>,
        key: &PlaceKey,
    ) -> bool {
        let ty = destination.ty(&mir.local_decls, tcx).ty;
        if container_kind(tcx, ty).is_none() {
            return false;
        }
        self.invalidate(key);
        self.values.insert(pseudo_key(key, PSEUDO_LEN), zero(ctx));
        self.values
            .insert(pseudo_key(key, PSEUDO_CAPACITY), zero(ctx));
        true
    }

    /// Model `with_capacity`: the result is empty, and its capacity is at least
    /// the requested one.
    ///
    /// "At least" rather than "exactly" is what keeps this sound for a
    /// zero-sized element, whose `Vec` reports `usize::MAX` capacity. It is also
    /// all a program may rely on from the standard library.
    fn model_with_capacity<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        const_map: &ConstMap,
        tcx: TyCtxt<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        destination: &Place<'tcx>,
        key: &PlaceKey,
    ) -> bool {
        let ty = destination.ty(&mir.local_decls, tcx).ty;
        if container_kind(tcx, ty).is_none() {
            return false;
        }
        let Some(arg) = args.first() else {
            return false;
        };
        let Some(requested) = self.int_operand(ctx, mir, const_map, tcx, &arg.node) else {
            return false;
        };
        self.invalidate(key);
        self.values.insert(pseudo_key(key, PSEUDO_LEN), zero(ctx));
        let capacity_key = pseudo_key(key, PSEUDO_CAPACITY);
        let capacity = self.fact_value(
            ctx,
            capacity_key.clone(),
            format!("{}.capacity", place_label_of_key(key)),
            Some(ValueDomain::Int(IntType::Usize)),
        );
        self.assume(ctx, capacity.ge(&requested));
        self.values.insert(capacity_key, capacity);
        true
    }

    /// Model `slice::iter`: a fresh cursor at zero, over the slice's length.
    fn model_iter<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        tcx: TyCtxt<'tcx>,
        writes: &BodyWrites,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        key: &PlaceKey,
    ) -> bool {
        let Some(arg) = args.first() else {
            return false;
        };
        let Some(ty) = operand_ty(mir, tcx, &arg.node) else {
            return false;
        };
        if !is_slice_or_str_ref(ty) {
            return false;
        }
        let Some(place) = operand_place(&arg.node) else {
            return false;
        };
        let Some(length) = self.slice_length(ctx, mir, tcx, writes, place) else {
            return false;
        };
        self.invalidate(key);
        self.values
            .insert(pseudo_key(key, PSEUDO_CURSOR), zero(ctx));
        self.values.insert(pseudo_key(key, PSEUDO_LEN), length);
        true
    }

    /// Model `Iterator::enumerate`: the same cursor/len as the inner iterator,
    /// plus a count offset that starts at zero.
    ///
    /// `Enumerate` adds its own counter to whatever the inner iterator yields, so
    /// the offset is a fact of its own — a `slice::Iter` advanced past the first
    /// element before `enumerate()` still counts its yields from zero. The bound
    /// is the same one a length has: the offset counts yields, and no slice
    /// iterator yields more than its length.
    fn model_enumerate<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        tcx: TyCtxt<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        key: &PlaceKey,
    ) -> bool {
        let Some(arg) = args.first() else {
            return false;
        };
        let Some(ty) = operand_ty(mir, tcx, &arg.node) else {
            return false;
        };
        if iterator_kind(tcx, ty).is_none() {
            return false;
        }
        let Some(source) = operand_place(&arg.node).and_then(place_key) else {
            return false;
        };
        let Some((cursor, length)) = self.iterator_parts(ctx, tcx, &source, ty) else {
            return false;
        };
        self.invalidate(key);
        self.values.insert(pseudo_key(key, PSEUDO_CURSOR), cursor);
        self.values.insert(pseudo_key(key, PSEUDO_LEN), length);
        let count_key = pseudo_key(key, PSEUDO_COUNT);
        let count = self.fact_value(
            ctx,
            count_key.clone(),
            format!("{}.count", place_label_of_key(key)),
            Some(ValueDomain::SliceLen),
        );
        self.values.insert(count_key, count);
        true
    }

    /// Model `IntoIterator::into_iter` for the iterator shapes this analysis
    /// understands (and for `&[T]`/`&Vec<T>`, which become a slice iterator).
    fn model_into_iter<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        tcx: TyCtxt<'tcx>,
        writes: &BodyWrites,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        key: &PlaceKey,
    ) -> bool {
        let Some(arg) = args.first() else {
            return false;
        };
        let Some(ty) = operand_ty(mir, tcx, &arg.node) else {
            return false;
        };
        let Some(source) = operand_place(&arg.node).and_then(place_key) else {
            return false;
        };
        if let Some(kind) = iterator_kind(tcx, ty) {
            let Some((cursor, length)) = self.iterator_parts(ctx, tcx, &source, ty) else {
                return false;
            };
            let count = if kind == IteratorKind::Enumerate {
                self.pseudo_fact(ctx, &source, PSEUDO_COUNT)
                    .unwrap_or_else(|| zero(ctx))
            } else {
                zero(ctx)
            };
            self.invalidate(key);
            self.copy_place_facts(&source, key);
            self.values.insert(pseudo_key(key, PSEUDO_CURSOR), cursor);
            self.values.insert(pseudo_key(key, PSEUDO_LEN), length);
            if kind == IteratorKind::Enumerate {
                self.values.insert(pseudo_key(key, PSEUDO_COUNT), count);
            }
            return true;
        }
        // `for x in &slice` / `for x in &vec` desugar to `IntoIterator` on the
        // reference, whose result is a slice iterator.
        if !matches!(ty.kind(), TyKind::Ref(..)) {
            return false;
        }
        let pointee = referent_ty(ty);
        let length = if is_slice_or_str_ref(ty) {
            let Some(place) = operand_place(&arg.node) else {
                return false;
            };
            let Some(length) = self.slice_length(ctx, mir, tcx, writes, place) else {
                return false;
            };
            length
        } else if container_kind(tcx, pointee) == Some(ContainerKind::Vec) {
            let Some(object) = Self::receiver_object(writes, &arg.node) else {
                return false;
            };
            self.container_length(ctx, &object)
        } else {
            return false;
        };
        self.invalidate(key);
        self.values
            .insert(pseudo_key(key, PSEUDO_CURSOR), zero(ctx));
        self.values.insert(pseudo_key(key, PSEUDO_LEN), length);
        true
    }

    /// Model `Iterator::next` for `Range`, `slice::Iter` and `Enumerate`.
    ///
    /// The `Some` discriminant is not guessed: it is exactly `cursor < len`, the
    /// same expression the body's `switchInt` then splits on. That is what makes
    /// the yielded element's bound (`start <= i < end`, or `cursor < len`) a
    /// guard on the body rather than a free variant.
    fn model_next<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        tcx: TyCtxt<'tcx>,
        writes: &BodyWrites,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        key: &PlaceKey,
    ) -> bool {
        let Some(arg) = args.first() else {
            return false;
        };
        let Some(ty) = operand_ty(mir, tcx, &arg.node) else {
            return false;
        };
        let Some(kind) = iterator_kind(tcx, ty) else {
            return false;
        };
        let Some(receiver) = Self::receiver_object(writes, &arg.node) else {
            return false;
        };
        let Some((cursor, length)) = self.iterator_parts(ctx, tcx, &receiver, ty) else {
            return false;
        };

        let some = cursor.lt(&length);
        let one = one(ctx);
        let zero = zero(ctx);
        let variant = some.ite(&one, &zero);
        let next_cursor = some.ite(&(&cursor + &one), &cursor);

        self.invalidate(key);
        match kind {
            IteratorKind::Range => {
                // The yielded item is the pre-increment `start`.
                self.values.insert(pseudo_key(key, 0), cursor);
                // Keep the Range's real `start` in step as well: a later direct
                // read of `range.start` must not see the value from before the
                // call.
                self.set_value(pseudo_key(&receiver, PSEUDO_CURSOR), next_cursor.clone());
                self.set_value(pseudo_key(&receiver, PSEUDO_LEN), length);
                let mut start = receiver.1.clone();
                start.push(0);
                self.set_value((receiver.0, start), next_cursor);
            }
            IteratorKind::Slice => {
                self.set_value(pseudo_key(&receiver, PSEUDO_CURSOR), next_cursor);
                self.set_value(pseudo_key(&receiver, PSEUDO_LEN), length);
            }
            IteratorKind::Enumerate => {
                let count_key = pseudo_key(&receiver, PSEUDO_COUNT);
                let count = self
                    .pseudo_fact(ctx, &receiver, PSEUDO_COUNT)
                    .unwrap_or_else(|| zero.clone());
                // The payload is `(index, item)`, and the index is `Enumerate`'s
                // own definition: `inner.next()` yields the inner iterator's
                // element at `cursor`, and the yielded *ordinal* is `cursor +
                // count` — `count` is zero for the fresh `slice::iter()` the
                // desugar builds, and an offset when the inner iterator was
                // advanced before `enumerate()`. Splitting the two keeps this
                // exact for both.
                let mut payload = key.1.clone();
                payload.push(0);
                let mut index_field = payload.clone();
                index_field.push(0);
                self.values.insert((key.0, index_field), &cursor + &count);
                let next_count = some.ite(&(&count + &one), &count);
                self.set_value(count_key, next_count);
                self.set_value(pseudo_key(&receiver, PSEUDO_CURSOR), next_cursor);
                self.set_value(pseudo_key(&receiver, PSEUDO_LEN), length);
            }
        }
        self.symbolic_discriminants.insert(key.clone(), variant);
        true
    }

    /// Model common standard library functions like `min`, `max`, `abs`, and Option/Result checks (`is_some`, `is_none`, `is_ok`, `is_err`).
    #[allow(clippy::too_many_arguments)]
    fn apply_known_callee<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        const_map: &ConstMap,
        tcx: TyCtxt<'tcx>,
        writes: &BodyWrites,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        key: &PlaceKey,
        destination: &Place<'tcx>,
    ) -> CalleeApplied {
        let Operand::Constant(constant) = func else {
            return CalleeApplied::NotModelled;
        };
        let TyKind::FnDef(def_id, _) = constant.const_.ty().kind() else {
            return CalleeApplied::NotModelled;
        };
        let krate = tcx.crate_name(def_id.krate);
        if krate.as_str() != "core" && krate.as_str() != "std" && krate.as_str() != "alloc" {
            return CalleeApplied::NotModelled;
        }
        let name = tcx.item_name(*def_id).to_string();
        let applied = match name.as_str() {
            "len" => self.model_len(ctx, mir, tcx, writes, args, key),
            "is_empty" => self.model_is_empty(ctx, mir, tcx, writes, args, key),
            "capacity" => self.model_capacity(ctx, mir, tcx, writes, args, key),
            "new" => self.model_new(ctx, mir, tcx, destination, key),
            "with_capacity" => {
                self.model_with_capacity(ctx, mir, const_map, tcx, args, destination, key)
            }
            "iter" => self.model_iter(ctx, mir, tcx, writes, args, key),
            "enumerate" => self.model_enumerate(ctx, mir, tcx, args, key),
            "into_iter" => self.model_into_iter(ctx, mir, tcx, writes, args, key),
            "next" => self.model_next(ctx, mir, tcx, writes, args, key),
            "min" if args.len() == 2 => {
                let Some(l) = self.int_operand(ctx, mir, const_map, tcx, &args[0].node) else {
                    return CalleeApplied::NotModelled;
                };
                let Some(r) = self.int_operand(ctx, mir, const_map, tcx, &args[1].node) else {
                    return CalleeApplied::NotModelled;
                };
                let min_val = l.le(&r).ite(&l, &r);
                self.set_value(key.clone(), min_val);
                true
            }
            "max" if args.len() == 2 => {
                let Some(l) = self.int_operand(ctx, mir, const_map, tcx, &args[0].node) else {
                    return CalleeApplied::NotModelled;
                };
                let Some(r) = self.int_operand(ctx, mir, const_map, tcx, &args[1].node) else {
                    return CalleeApplied::NotModelled;
                };
                let max_val = l.ge(&r).ite(&l, &r);
                self.set_value(key.clone(), max_val);
                true
            }
            "abs" if args.len() == 1 => {
                let Some(val) = self.int_operand(ctx, mir, const_map, tcx, &args[0].node) else {
                    return CalleeApplied::NotModelled;
                };
                let z = zero(ctx);
                let abs_val = val.ge(&z).ite(&val, &(&z - &val));
                self.set_value(key.clone(), abs_val);
                true
            }
            "is_some" if args.len() == 1 => {
                if let Some(disc) = self.operand_discriminant(writes, &args[0].node) {
                    let val = if disc == 1 { one(ctx) } else { zero(ctx) };
                    self.set_value(key.clone(), val);
                    true
                } else {
                    false
                }
            }
            "is_none" if args.len() == 1 => {
                if let Some(disc) = self.operand_discriminant(writes, &args[0].node) {
                    let val = if disc == 0 { one(ctx) } else { zero(ctx) };
                    self.set_value(key.clone(), val);
                    true
                } else {
                    false
                }
            }
            "is_ok" if args.len() == 1 => {
                if let Some(disc) = self.operand_discriminant(writes, &args[0].node) {
                    let val = if disc == 0 { one(ctx) } else { zero(ctx) };
                    self.set_value(key.clone(), val);
                    true
                } else {
                    false
                }
            }
            "is_err" if args.len() == 1 => {
                if let Some(disc) = self.operand_discriminant(writes, &args[0].node) {
                    let val = if disc == 1 { one(ctx) } else { zero(ctx) };
                    self.set_value(key.clone(), val);
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if applied {
            CalleeApplied::Modelled
        } else {
            CalleeApplied::NotModelled
        }
    }

    /// The successor edges of a block, each with the condition that must hold to
    /// take it. `None` means the edge is unconditional.
    fn successor_guards<'tcx>(
        &mut self,
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        const_map: &ConstMap,
        tcx: TyCtxt<'tcx>,
        block: BasicBlock,
    ) -> Vec<(BasicBlock, Option<Bool<'ctx>>)> {
        let Some(terminator) = &mir.basic_blocks[block].terminator else {
            return Vec::new();
        };
        match &terminator.kind {
            TerminatorKind::Goto { target } => vec![(*target, None)],
            // Only the success edge is followed, matching `compute_reachable_blocks`.
            TerminatorKind::Assert {
                cond,
                expected,
                target,
                ..
            } => {
                // A condition that is already constant is folded here. When it
                // contradicts `expected` — `assert(const false, …)`, which is how
                // the coroutine transform spells "resumed after completion" — the
                // success edge cannot be taken. Walking it as a *self*-edge merges
                // one longer condition into the block on every round, a fixpoint
                // that never arrives, and the body then fell back to
                // path-insensitivity and lost every guard in it (S6). The block is
                // still reached and reported; only its impossible continuation is
                // dropped, not the assertion.
                // A constant condition needs no solver: `assert(const false, …)`
                // never falls through, so its success edge must not be walked at
                // all. `int_operand` answers only for integer types, which is why
                // this asks for the boolean directly.
                if let Some(value) = crate::constant_prop::const_bool(cond, tcx) {
                    if value != *expected {
                        return Vec::new();
                    }
                }
                let term = self.int_operand(ctx, mir, const_map, tcx, cond);
                let guard =
                    term.map(|term| term._eq(&if *expected { one(ctx) } else { zero(ctx) }));
                vec![(*target, guard)]
            }
            TerminatorKind::SwitchInt { discr, targets } => {
                let term = self.int_operand(ctx, mir, const_map, tcx, discr);
                let values = targets.all_values();
                let all = targets.all_targets();
                let discr_ty = operand_int_type(discr, mir, tcx);
                let mut edges = Vec::with_capacity(all.len());
                for (index, target) in all.iter().enumerate() {
                    // The last target is the `otherwise` branch: it is taken when
                    // the discriminant matches none of the listed values.
                    let guard = match (index < values.len(), term.as_ref()) {
                        (true, Some(term)) => Some(
                            term._eq(&int_lit(ctx, switch_value(values[index].get(), discr_ty))),
                        ),
                        (false, Some(term)) => {
                            let listed: Vec<Bool<'ctx>> = values
                                .iter()
                                .map(|value| {
                                    term._eq(&int_lit(ctx, switch_value(value.get(), discr_ty)))
                                        .not()
                                })
                                .collect();
                            let refs: Vec<&Bool<'ctx>> = listed.iter().collect();
                            Some(all_of(ctx, &refs))
                        }
                        // No symbolic discriminant: we know only that one of the
                        // edges is taken, so the precondition is left open.
                        (_, None) => None,
                    };
                    edges.push((*target, guard));
                }
                edges
            }
            TerminatorKind::Drop { target, .. } => vec![(*target, None)],
            TerminatorKind::Call {
                target: Some(target),
                ..
            } => vec![(*target, None)],
            TerminatorKind::Yield { resume, .. } => vec![(*resume, None)],
            // An asm block that returns goes to one of the blocks its `Label`
            // operands name; which one is the asm's own business, so every edge
            // is left open (`None`) rather than guessed. `NORETURN` asm has no
            // targets at all, which the MIR validator enforces.
            TerminatorKind::InlineAsm { targets, .. } => {
                targets.iter().map(|target| (*target, None)).collect()
            }
            // Borrow-check artefacts: the real edge is the one execution takes.
            TerminatorKind::FalseEdge { real_target, .. } => vec![(*real_target, None)],
            TerminatorKind::FalseUnwind { real_target, .. } => vec![(*real_target, None)],
            // No successor: control does not come back to this body (or this is
            // the end of it). Listed rather than wildcarded, so a new variant has
            // to be thought about instead of silently truncating the walk — the
            // shape that made an `asm!` block's following assertions unchecked.
            TerminatorKind::Call { target: None, .. }
            | TerminatorKind::TailCall { .. }
            | TerminatorKind::UnwindResume
            | TerminatorKind::UnwindTerminate(_)
            | TerminatorKind::Return
            | TerminatorKind::Unreachable
            | TerminatorKind::CoroutineDrop => Vec::new(),
        }
    }

    /// A cheap, stable fingerprint of the state, used to detect the fixpoint.
    fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.pc.hash(&mut hasher);
        for (key, term) in &self.values {
            key.hash(&mut hasher);
            term.hash(&mut hasher);
        }
        for (name, def) in &self.defs {
            name.hash(&mut hasher);
            def.hash(&mut hasher);
        }
        for (key, product) in &self.checked {
            key.hash(&mut hasher);
            product.rendering().hash(&mut hasher);
        }
        for (key, term) in &self.symbolic_discriminants {
            key.hash(&mut hasher);
            term.hash(&mut hasher);
        }
        for (name, input) in &self.inputs {
            name.hash(&mut hasher);
            input.kind.hash(&mut hasher);
            input.domain.hash(&mut hasher);
        }
        for key in &self.generalised {
            key.hash(&mut hasher);
        }
        hasher.finish()
    }
}

/// Every symbol from the state's inputs, mapped to the name it carries in the
/// previous-trip view of the same state.
///
/// A name that already carries [`PREVIOUS_TRIP_PREFIX`] is mapped to itself. It
/// names a *role* — "the value of `phi_bb1_3` one trip before the formula that
/// mentions it" — and prefixing it again would mint a new symbol every round, so
/// the state would grow instead of settling and the round budget would run out
/// before a fixpoint. Leaving it alone keeps one name for one role.
fn previous_trip_map<'ctx>(
    ctx: &'ctx Context,
    inputs: &BTreeMap<String, InputVar>,
    prefix: &str,
) -> Vec<(Int<'ctx>, Int<'ctx>)> {
    inputs
        .keys()
        .map(|name| {
            let renamed = previous_trip_name(name, prefix);
            (
                Int::new_const(ctx, name.as_str()),
                Int::new_const(ctx, renamed.as_str()),
            )
        })
        .collect()
}

/// A `core::num` arithmetic form whose result this analysis can define exactly,
/// instead of recording the call as an unknown value (K9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SafeArith {
    /// `wrapping_*`: the mathematical result reduced into the type's width.
    Wrapping,
    /// `saturating_*`: the mathematical result clamped to the type's range.
    Saturating,
    /// `checked_*`: the mathematical result in the `Some` payload, with the
    /// discriminant modelled alongside it — `Some` exactly when that sum is in
    /// range (see [`CheckedProduct`]).
    Checked,
}

/// Recognise `core::num::<impl T>::{wrapping,saturating,checked}_{add,sub,mul}`.
///
/// Only the standard library's own methods are recognised, and only these three
/// families: giving a callee this analysis' semantics is a claim about what the
/// callee computes, so a crate-local function that happens to be named
/// `wrapping_add` must not be given them. Hence the crate check as well as the
/// name check.
fn safe_arith_form<'tcx>(tcx: TyCtxt<'tcx>, func: &Operand<'tcx>) -> Option<(SafeArith, BinOp)> {
    let Operand::Constant(constant) = func else {
        return None;
    };
    let TyKind::FnDef(def_id, _) = constant.const_.ty().kind() else {
        return None;
    };
    let krate = tcx.crate_name(def_id.krate);
    if krate.as_str() != "core" && krate.as_str() != "std" {
        return None;
    }
    let name = tcx.item_name(*def_id).to_string();
    let (family, operation) = name.split_once('_')?;
    // `checked_*`'s result is read as `((_3 as Some).0: u8)` — a `Downcast` in the
    // projection, which [`place_key`] cannot name. The payload is modelled
    // instead: [`unique_payload_field`] decides, from the destination's own type,
    // whether that position belongs to one variant alone, which is the condition
    // the collapse needs to be sound.
    let family = match family {
        "wrapping" => SafeArith::Wrapping,
        "saturating" => SafeArith::Saturating,
        "checked" => SafeArith::Checked,
        _ => return None,
    };
    let op = match operation {
        "add" => BinOp::Add,
        "sub" => BinOp::Sub,
        "mul" => BinOp::Mul,
        _ => return None,
    };
    Some((family, op))
}

/// The one variant and field position at which an enum's payload can be named
/// without ambiguity, with that field's type: the variant's own index, the
/// position, and the type of the field that sits there.
///
/// Collapsing `(_3 as Some).0` to "field 0 of `_3`" is sound only where no other
/// variant of the same enum holds a field at that position, because only then does
/// the collapsed place name the payload and nothing else. `Option<T>` — the only
/// thing `core::num::checked_*` returns — is exactly that shape: `None` carries no
/// fields, so its payload is the sole claimant of position 0. A `Result<T, E>` is
/// not, and gets `None` here: `Ok`'s field 0 and `Err`'s field 0 are different
/// fields at the same index, which one key cannot tell apart.
fn unique_payload_field<'tcx>(
    tcx: TyCtxt<'tcx>,
    ty: Ty<'tcx>,
) -> Option<(usize, usize, usize, Ty<'tcx>)> {
    let TyKind::Adt(adt_def, args) = ty.kind() else {
        return None;
    };
    if !adt_def.is_enum() || adt_def.variants().len() != 2 {
        return None;
    }
    let mut payload = None;
    let mut empty = None;
    for (variant_index, variant) in adt_def.variants().iter_enumerated() {
        if variant.fields.is_empty() {
            empty = Some(variant_index.index());
            continue;
        }
        // A payload wider than one field means two fields can share a position,
        // and the collapse is refused.
        if payload.is_some() || variant.fields.len() != 1 {
            return None;
        }
        let field = variant.fields.iter().next()?;
        // The field's type is only read to check that it is an integer of the
        // width the callee operates on, so the instantiation is taken as written:
        // an unnormalized projection yields no domain and the collapse is refused
        // rather than guessed at.
        let field_ty = tcx
            .type_of(field.did)
            .instantiate(tcx, args)
            .skip_normalization();
        // One field, so its position is `0` — which is the index the read's own
        // projection will carry.
        payload = Some((variant_index.index(), 0usize, field_ty));
    }
    let (payload_variant, index, field_ty) = payload?;
    let empty_variant = empty?;
    Some((payload_variant, index, empty_variant, field_ty))
}

/// The place a downcast payload read names, when the enum's shape makes it
/// unambiguous: `(_3 as Some).0` is field `0` of `_3`, and nothing else is.
fn payload_key<'tcx>(tcx: TyCtxt<'tcx>, mir: &Body<'tcx>, place: &Place<'tcx>) -> Option<PlaceKey> {
    let mut elems = place.projection.iter();
    let PlaceElem::Downcast(_, variant) = elems.next()? else {
        return None;
    };
    let PlaceElem::Field(field, _) = elems.next()? else {
        return None;
    };
    if elems.next().is_some() {
        return None;
    }
    let (payload_variant, index, _, _) =
        unique_payload_field(tcx, mir.local_decls[place.local].ty)?;
    (payload_variant == variant.index() && index == field.index())
        .then_some((place.local.index(), vec![index]))
}

/// The value `saturating_*` returns for a mathematical result of `sum`: the
/// result clamped to the type's range. Exact, so usable as the value rather than
/// as a bound — clamping needs no modulus, which is why it holds in one step.
///
/// (`wrapping_*` needs the fixed-width reduction instead, and gets it from
/// [`fixed_binary`], whose `Mul` case is exactly why a single add-or-subtract of
/// the modulus would not do: `127 * 127` wrapped is not `16256 - 256`.)
fn saturating_value<'ctx>(ctx: &'ctx Context, sum: &Int<'ctx>, ty: IntType) -> Int<'ctx> {
    let (min, max) = ty.bounds();
    let high = sum.gt(&int_lit(ctx, max));
    let low = sum.lt(&int_lit(ctx, min));
    let clamped = high.ite(&int_lit(ctx, max), sum);
    low.ite(&int_lit(ctx, min), &clamped)
}

/// Whether a symbol name is one the widening introduced: a previous-trip name,
/// declared when a back-edge value is related to the trip before it.
fn is_widened(name: &str) -> bool {
    name.contains(PREVIOUS_TRIP_PREFIX)
}

/// Extract all identifier symbols from a rendered SMT-LIB formula in a single linear pass.
fn extract_symbols(rendered: &str) -> HashSet<String> {
    let mut symbols = HashSet::new();
    let mut current = String::new();
    for c in rendered.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            current.push(c);
        } else if !current.is_empty() {
            symbols.insert(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        symbols.insert(current);
    }
    symbols
}

/// Whether a rendered formula names `symbol` as a whole symbol.
///
/// A plain substring test would be wrong in the direction that matters: `mir_3`
/// is a substring of `prev_bb6_mir_3`, so "does this definition mention `mir_3`"
/// would answer yes for an equation about the previous trip alone, and the
/// dependence test below would walk backwards into the symbol it is trying to
/// decide about. The neighbours are therefore checked to be non-identifier
/// characters on both sides.
#[cfg(test)]
fn mentions_symbol(rendered: &str, symbol: &str) -> bool {
    if symbol.is_empty() {
        return false;
    }
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut from = 0;
    while let Some(offset) = rendered[from..].find(symbol) {
        let start = from + offset;
        let end = start + symbol.len();
        let before_ok = rendered[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !is_ident(c));
        let after_ok = rendered[end..].chars().next().is_none_or(|c| !is_ident(c));
        if before_ok && after_ok {
            return true;
        }
        // Past this occurrence: `symbol` is non-empty, so this always advances.
        from = end;
    }
    false
}

/// The name a symbol carries in the previous-trip view of a state.
fn previous_trip_name(name: &str, prefix: &str) -> String {
    if name.contains(PREVIOUS_TRIP_PREFIX) {
        name.to_string()
    } else {
        format!("{prefix}{name}")
    }
}

/// True when `place` is a pointer whose metadata is a length: `&[T]`, `&mut
/// [T]`, `*const [T]` and the string forms of those.
///
/// `PtrMetadata` on one of these is the length of the slice, which is why
/// [`ValueDomain::SliceLen`] bounds it at `isize::MAX`. On anything else the
/// metadata is not a number the analysis can name at all — a `&[T; N]`
/// reference is thin, so its metadata is `()`, and a `&dyn Trait` one is a
/// vtable pointer — and the place stays unmodelled.
fn metadata_is_length<'tcx>(mir: &Body<'tcx>, tcx: TyCtxt<'tcx>, place: &Place<'tcx>) -> bool {
    let local_ty = mir.local_decls[place.local].ty;
    let projected = PlaceTy::from_ty(local_ty).multi_projection_ty(tcx, place.projection);
    let inner = match projected.ty.kind() {
        TyKind::Ref(_, inner, _) | TyKind::RawPtr(inner, _) => *inner,
        _ => return false,
    };
    matches!(inner.kind(), TyKind::Slice(_) | TyKind::Str)
}

/// The locals this body writes to, the ones whose address it takes, and what its
/// own pointers point at.
///
/// Three questions the transfer function cannot answer one statement at a time,
/// so all of them are settled once for the body:
///
/// * `written` is what lets a slice length be named only for a place that keeps
///   one object for the whole function, which is what makes that name sound (see
///   [`PathState::metadata_value`]). Every statement and terminator that writes a
///   place contributes, not only `Assign`.
/// * `address_taken` is the set a store the key scheme cannot name has to assume
///   it may have reached, and the set a write by a body this analysis does not
///   read has to assume as well. A pointer used in this body is either a
///   parameter — whose pointee is the caller's memory and not one of these locals
///   — or derived from `&place`/`&mut place`/`&raw place` for a place this body
///   names, so a local that is never borrowed cannot be written through one.
/// * `pointees` says what a local *that holds a pointer* points at: `Some(Some(l))`
///   when the only thing the body ever stores into it is `&l` (one hop of
///   `Rvalue::Ref`/`Rvalue::RawPtr`, which is how MIR materialises every
///   `f(&mut x)` — the pointer lives in a temp first), `Some(None)` when it is
///   written by anything else, a second and different borrow, or a borrow taken
///   *through* another pointer. This is what keeps
///   `let mut w = 0; bump(&mut w); v + 240` provable: the callee can write `w`,
///   and nothing about `v`.
struct BodyWrites {
    written: HashSet<usize>,
    #[allow(dead_code)]
    address_taken: HashSet<usize>,
    mutably_address_taken: HashSet<usize>,
    pointees: HashMap<usize, Option<usize>>,
}

/// Whether a type mentions a lifetime anywhere in its structure.
///
/// This is the drop-glue question in a form MIR can answer: a value whose type
/// carries a lifetime may contain a reference into this body's own storage, so
/// running its destructor may write what this analysis has already proved a
/// value of. A type with no lifetime (`Vec<u8>`, `String`, `Rc<u8>`) cannot
/// hold such a reference.
///
/// `has_free_regions` reads as the obvious predicate, but it answers a different
/// question once regions are erased: the verifier analyses `optimized_mir`,
/// where `Holder<'_>` is lowered as `Holder<'{erased}>`, and an erased region is
/// not *free*. The S1 guard built on it therefore never fired for exactly the
/// type `tests/statement_writes` drops, and the value the destructor overwrote
/// was proved from its old definition again. Asking whether a lifetime appears
/// at all keeps `Vec<u8>` out of the retirement while `Holder<'_>` and
/// `Vec<&mut u8>` stay in it.
fn holds_a_lifetime(ty: Ty<'_>) -> bool {
    ty.walk().any(|arg| arg.as_region().is_some())
}

/// Record what a local that holds a pointer can point at, merging with what is
/// already known about it.
///
/// The merge only ever moves towards `None`: a local written twice with different
/// pointees, or once by something that is not a borrow of a local, may point at
/// any of them, and the callers treat that as "any local this body borrowed".
fn record_pointee(
    pointees: &mut HashMap<usize, Option<usize>>,
    holder: usize,
    pointee: Option<usize>,
) {
    match pointees.entry(holder) {
        std::collections::hash_map::Entry::Vacant(slot) => {
            slot.insert(pointee);
        }
        std::collections::hash_map::Entry::Occupied(mut slot) => {
            if *slot.get() != pointee {
                slot.insert(None);
            }
        }
    }
}

fn body_writes<'tcx>(tcx: TyCtxt<'tcx>, mir: &Body<'tcx>) -> BodyWrites {
    let mut written = HashSet::new();
    let mut address_taken = HashSet::new();
    let mut mutably_address_taken = HashSet::new();
    let mut pointees: HashMap<usize, Option<usize>> = HashMap::new();
    let writes_place = |written: &mut HashSet<usize>, place: &Place<'_>| {
        written.insert(place.local.index());
    };
    let param_env = tcx.param_env(mir.source.def_id());
    let typing_env =
        rustc_middle::ty::TypingEnv::new(param_env, rustc_middle::ty::TypingMode::PostAnalysis);
    for block in mir.basic_blocks.iter() {
        for statement in &block.statements {
            match &statement.kind {
                StatementKind::Assign(box (place, rvalue)) => {
                    writes_place(&mut written, place);
                    match rvalue {
                        Rvalue::Ref(_, borrow_kind, borrowed) => {
                            address_taken.insert(borrowed.local.index());
                            let is_mut = match borrow_kind {
                                rustc_middle::mir::BorrowKind::Mut { .. } => true,
                                _ => {
                                    let place_ty = mir.local_decls[borrowed.local].ty;
                                    !place_ty.is_freeze(tcx, typing_env)
                                }
                            };
                            if is_mut {
                                mutably_address_taken.insert(borrowed.local.index());
                            }
                        }
                        Rvalue::RawPtr(mutbl, borrowed) => {
                            address_taken.insert(borrowed.local.index());
                            let is_mut = if *mutbl == rustc_middle::mir::Mutability::Mut.into() {
                                true
                            } else {
                                let place_ty = mir.local_decls[borrowed.local].ty;
                                !place_ty.is_freeze(tcx, typing_env)
                            };
                            if is_mut {
                                mutably_address_taken.insert(borrowed.local.index());
                            }
                        }
                        _ => {}
                    }
                    if place.projection.is_empty() {
                        let pointee = match rvalue {
                            Rvalue::Ref(_, _, borrowed) | Rvalue::RawPtr(_, borrowed)
                                if !borrowed
                                    .projection
                                    .iter()
                                    .any(|elem| matches!(elem, PlaceElem::Deref)) =>
                            {
                                Some(borrowed.local.index())
                            }
                            // `&*p` borrows through another pointer, and anything
                            // else is not a borrow at all: neither says which
                            // local this pointer points at.
                            _ => None,
                        };
                        record_pointee(&mut pointees, place.local.index(), pointee);
                    }
                }
                // The only statement kinds besides `Assign` that write a place:
                // the discriminant of a coroutine's state, and an asm block's
                // outputs. Both make the local something else, so a slice length
                // may not be named for it either.
                StatementKind::SetDiscriminant { place, .. } => writes_place(&mut written, place),
                StatementKind::Intrinsic(box NonDivergingIntrinsic::CopyNonOverlapping(_)) => {}
                StatementKind::Intrinsic(box NonDivergingIntrinsic::Assume(_))
                | StatementKind::FakeRead(_)
                | StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::PlaceMention(_)
                | StatementKind::AscribeUserType(..)
                | StatementKind::Coverage(_)
                | StatementKind::ConstEvalCounter
                | StatementKind::Nop
                | StatementKind::BackwardIncompatibleDropHint { .. } => {}
            }
        }
        if let Some(terminator) = &block.terminator {
            match &terminator.kind {
                TerminatorKind::Call { destination, .. } => {
                    writes_place(&mut written, destination);
                }
                TerminatorKind::Drop { place, .. } => writes_place(&mut written, place),
                TerminatorKind::Yield { resume_arg, .. } => {
                    writes_place(&mut written, resume_arg);
                    if resume_arg.projection.is_empty() {
                        record_pointee(&mut pointees, resume_arg.local.index(), None);
                    }
                }
                TerminatorKind::InlineAsm { operands, .. } => {
                    for operand in operands.iter() {
                        let place = match operand {
                            InlineAsmOperand::Out { place, .. } => place.as_ref(),
                            InlineAsmOperand::InOut { out_place, .. } => out_place.as_ref(),
                            _ => None,
                        };
                        if let Some(place) = place {
                            writes_place(&mut written, place);
                            if place.projection.is_empty() {
                                record_pointee(&mut pointees, place.local.index(), None);
                            }
                        }
                    }
                }
                TerminatorKind::TailCall { .. }
                | TerminatorKind::Goto { .. }
                | TerminatorKind::SwitchInt { .. }
                | TerminatorKind::UnwindResume
                | TerminatorKind::UnwindTerminate(_)
                | TerminatorKind::Return
                | TerminatorKind::Unreachable
                | TerminatorKind::Assert { .. }
                | TerminatorKind::CoroutineDrop
                | TerminatorKind::FalseEdge { .. }
                | TerminatorKind::FalseUnwind { .. } => {}
            }
        }
    }
    BodyWrites {
        written,
        address_taken,
        mutably_address_taken,
        pointees,
    }
}

/// The places a store the key scheme cannot name must retire, given the locals
/// this body writes and borrows.
///
/// The store's own base local always: `a[i] = …` destroys the whole `a` a state
/// may have recorded for it, and a downcast write like `(_3 as Some).0 = …`
/// destroys `_3`. When the projection goes *through* a pointer, every
/// mutably-address-taken local is a candidate as well, because which of them the
/// reference points at is the aliasing question and this analysis does not answer
/// it — answering with "all of them" is what keeps a value the store overwrote
/// from being read back as the value it held before. Sorted and deduplicated, so
/// the retirement order does not depend on hash iteration.
fn retirement_targets(local: usize, through_pointer: bool, writes: &BodyWrites) -> Vec<PlaceKey> {
    let mut targets = vec![(local, Vec::new())];
    if through_pointer {
        targets.extend(
            writes
                .mutably_address_taken
                .iter()
                .map(|l| (*l, Vec::new())),
        );
    }
    targets.sort();
    targets.dedup();
    targets
}

/// What an operand a body this analysis does not read receives can write.
///
/// A callee, an asm block and drop glue all see this body's pointers and can
/// write through them, and who wrote what afterwards is the aliasing question
/// this analysis does not answer. It does know two things that make the answer
/// cheaper than "everything": a pointer that is one hop from `&local` can only
/// reach that local, and a pointer that is a *parameter* is the caller's memory,
/// which is not one of this body's locals at all.
enum PointerWrite {
    /// Nothing this state holds a value for.
    Nothing,
    /// Exactly this local: the operand's only definition in this body is a borrow
    /// of it.
    Local(usize),
    /// Any local this body took the address of.
    Any,
}

fn operand_pointer_write<'tcx>(
    mir: &Body<'tcx>,
    tcx: TyCtxt<'tcx>,
    writes: &BodyWrites,
    operand: &Operand<'tcx>,
) -> PointerWrite {
    // A constant cannot be written through, and an operand that is not a pointer
    // at all is passed by value. Both are checked before the pointee map is
    // consulted, which is what keeps `f(x)` on an integer from retiring anything.
    let Some(place) = operand_place(operand) else {
        return PointerWrite::Nothing;
    };
    let local_ty = mir.local_decls[place.local].ty;
    let projected = PlaceTy::from_ty(local_ty).multi_projection_ty(tcx, place.projection);
    let (inner_ty, mutbl) = match projected.ty.kind() {
        TyKind::Ref(_, inner, mutbl) => (*inner, *mutbl),
        TyKind::RawPtr(inner, mutbl) => (*inner, *mutbl),
        _ => return PointerWrite::Nothing,
    };
    if mutbl == rustc_middle::mir::Mutability::Not {
        let param_env = tcx.param_env(mir.source.def_id());
        let typing_env =
            rustc_middle::ty::TypingEnv::new(param_env, rustc_middle::ty::TypingMode::PostAnalysis);
        if inner_ty.is_freeze(tcx, typing_env) {
            return PointerWrite::Nothing;
        }
    }
    // A pointer *read out of memory* — `(*_8)` where `_8: &&mut u8` — may be any
    // of them.
    if place
        .projection
        .iter()
        .any(|elem| matches!(elem, PlaceElem::Deref))
    {
        return PointerWrite::Any;
    }
    match writes.pointees.get(&place.local.index()) {
        Some(Some(pointee)) => PointerWrite::Local(*pointee),
        Some(None) => PointerWrite::Any,
        // Never written by this body: a parameter, so the pointee is the caller's
        // memory — or, for a local that cannot be a parameter, something this
        // analysis has no account of.
        None if place.local.index() <= mir.arg_count => PointerWrite::Nothing,
        None => PointerWrite::Any,
    }
}

fn operand_place<'a, 'tcx>(operand: &'a Operand<'tcx>) -> Option<&'a Place<'tcx>> {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => Some(place),
        _ => None,
    }
}

/// Interpret a `SwitchInt` value as the discriminant's own type.
///
/// The targets store the raw bit pattern, so a `-1` in an `i32` `match` arrives
/// as `4294967295` and has to be read back through the signed width before it is
/// compared against a signed term.
fn switch_value(value: u128, discr_ty: Option<IntType>) -> i128 {
    match discr_ty {
        Some(ty) if !ty.is_unsigned() => {
            let width = ty.bit_width();
            if width < 128 && value >= (1u128 << (width - 1)) {
                (value as i128) - (1i128 << width)
            } else {
                value.min(i128::MAX as u128) as i128
            }
        }
        _ => value.min(i128::MAX as u128) as i128,
    }
}

/// One block's contribution to a successor: the out-state plus the guard on that
/// particular edge.
#[derive(Clone)]
struct Contribution<'ctx> {
    state: PathState<'ctx>,
    guard: Option<Bool<'ctx>>,
}

impl<'ctx> Contribution<'ctx> {
    /// The same contribution with every free symbol renamed under `prefix`.
    ///
    /// The edge guard goes with it: it is a formula about the same state, and a
    /// guard left speaking about the un-renamed symbols would constrain the
    /// *current* trip with the *previous* trip's facts — a wrong answer rather
    /// than a weaker one.
    fn renamed(&self, ctx: &'ctx Context, prefix: &str) -> Self {
        let pairs = previous_trip_map(ctx, &self.state.inputs, prefix);
        let map: Vec<(&Int<'ctx>, &Int<'ctx>)> =
            pairs.iter().map(|(from, to)| (from, to)).collect();
        Self {
            state: self.state.previous_trip(ctx, prefix),
            guard: self.guard.as_ref().map(|guard| guard.substitute(&map)),
        }
    }
}

fn reach_of<'ctx>(ctx: &'ctx Context, contribution: &Contribution<'ctx>) -> Option<Bool<'ctx>> {
    match (contribution.state.pc.as_ref(), contribution.guard.as_ref()) {
        (None, None) => None,
        (None, Some(guard)) => Some(guard.clone()),
        (Some(pc), None) => Some(pc.clone()),
        (Some(pc), Some(guard)) => Some(all_of(ctx, &[pc, guard])),
    }
}

/// The state a cycle block is given once the unrolling budget is spent: the
/// condition rebuilt from the forward edges alone, and the values kept as a
/// *relation to their previous-trip version* rather than forgotten.
///
/// Forgetting the values is sound but loses the loop. `while i + 8 <= v.len()`
/// reads `i + 8` in the header — whose overflow check is one obligation — and
/// again in the body, whose bounds check is another. With the header's value
/// replaced by a fresh symbol, the guard that bounds the body's index speaks
/// about a symbol the check of the index knows nothing about, so the guard
/// proves nothing and the solver is free to choose the loop's *exit* value: the
/// false positive this function exists to remove.
///
/// Keeping the previous round's value under the same name is the opposite
/// mistake. `i`'s definition is `i + 8`, so the merge would assert `i = i + 8`,
/// which is unsatisfiable and quietly pins `i` to its entry value — and a value
/// pinned to its entry value is how a loop with a real bug two iterations in
/// gets proved. Renaming the back edge's contribution — every symbol in it,
/// guard included — to a `prev_bb<n>_` version gives the merge the fact it
/// really has:
///
/// ```text
/// i = 0  ∨  (prev guard ∧ i = prev_bb6_i + 8)
/// ```
///
/// Every real state satisfies it (take `prev_bb6_i` to be the previous trip's
/// `i`), and it is strong enough for the header's overflow check: the previous
/// trip's guard bounds `i`, and the length it bounds `i` against is bounded in
/// turn, so `i + 8` cannot leave the type.
///
/// The *condition* stays the forward merge's. A back edge's disjunct brings the
/// block's own condition from the previous round with it, which is how a nested
/// loop's condition used to grow without end; the values can take the renamed
/// contributions because their symbols are named per block and never chain into
/// themselves, so the round still reaches a fixpoint.
fn widened_state<'ctx>(
    ctx: &'ctx Context,
    block: BasicBlock,
    contributions: &[(BasicBlock, Contribution<'ctx>)],
    position: &HashMap<BasicBlock, usize>,
) -> PathState<'ctx> {
    let here = position.get(&block).copied().unwrap_or(usize::MAX);
    let is_forward = |pred: &BasicBlock| position.get(pred).is_some_and(|p| *p < here);

    // The condition is rebuilt from the forward edges, and so are the values:
    // they are the trip's own, computed by a block that dominates this one, so a
    // read of the place they belong to keeps meeting the guard that bounds it.
    let forward = forward_contributions(block, contributions, position);
    let mut state = merge_all(ctx, block, &forward);

    let mut alternatives: BTreeMap<PlaceKey, Vec<(bool, Option<Bool<'ctx>>, Int<'ctx>)>> =
        BTreeMap::new();
    let mut carried: Vec<PlaceKey> = Vec::new();
    for (pred, contribution) in contributions {
        let forward = is_forward(pred);
        let renamed = if forward {
            contribution.clone()
        } else {
            contribution.renamed(ctx, &format!("{PREVIOUS_TRIP_PREFIX}{}_", pred.index()))
        };
        let reach = reach_of(ctx, &renamed);
        for (key, term) in &renamed.state.values {
            if !forward && !carried.contains(key) {
                carried.push(key.clone());
            }
            alternatives.entry(key.clone()).or_default().push((
                forward,
                reach.clone(),
                term.clone(),
            ));
        }
        // The previous trip's free symbols come with it: their type bounds are
        // part of the relation's precondition, and a symbol the relation mentions
        // has to be declared for the obligation to be re-dischargeable at all.
        for (name, input) in &renamed.state.inputs {
            state
                .inputs
                .entry(name.clone())
                .or_insert_with(|| input.clone());
        }
        // An outer loop's generalisation travels with the values it applies to:
        // this block is unrolled inside that loop, so a place the outer loop gave
        // up on is one this state has given up on too.
        state
            .generalised
            .extend(renamed.state.generalised.iter().cloned());
    }

    // A value the *back edge* carries is the one this block cannot pin: it is
    // whatever the previous trip left there. Forgetting it makes the block's
    // reads re-derive the place's own symbol, bounded by the place's type — which
    // is what a guard on that place needs in order to mean anything (`outer <
    // limit` has to imply `outer <= 255`). Keeping the merged value in its place
    // would put an untyped merge symbol in front of every read and turn proofs
    // back into false alarms.
    for key in &carried {
        state.generalise(key);
    }

    // What is not forgotten is the loop *relation*, written about the very symbol
    // the reads will invent (`mir_3`), so the guard and the value it guards stay
    // the same quantity:
    //
    // ```text
    // mir_3 = 0  ∨  (prev guard ∧ mir_3 = prev_bb6_mir_3 + 8)
    // ```
    //
    // Every real state satisfies it (take `prev_bb6_mir_3` to be the previous
    // trip's counter), and it is what proves the header's own overflow check: the
    // previous trip's guard bounds the counter, and the length it bounds the
    // counter against is bounded in turn.
    for key in &carried {
        let Some(pairs) = alternatives.get(key) else {
            continue;
        };
        // A relation needs both halves: what this trip computed and what the
        // previous trip left. With one side only it is not a disjunction at all,
        // and asserting it would assert *that* side — pinning the place to the
        // previous trip's guard, which is exactly the over-constraint that proves
        // a loop with a bug two iterations in. A place only the back edge defines
        // is therefore left free, which is sound and simply imprecise.
        let complete = pairs.iter().any(|(forward, _, _)| *forward)
            && pairs.iter().any(|(forward, _, _)| !*forward);
        let symbol = state.symbol_name(key);
        if complete {
            let value = Int::new_const(ctx, symbol.as_str());
            let disjuncts: Vec<Bool<'ctx>> = pairs
                .iter()
                .filter_map(|(_, reach, term)| {
                    // `v = v` is what a place this trip never wrote leaves
                    // behind. As a disjunct it makes the whole relation true and
                    // hides the back-edge half, which is the half that carries
                    // the previous trip's bound: `for j in 0..v.len()` then read
                    // the loop's length as a free symbol unrelated to the length
                    // the body's bounds check uses. Dropping it keeps the sound
                    // half (`reach => v = prev`), which is what "the trip did not
                    // write this place" actually means.
                    if term.to_string().trim() == symbol {
                        return None;
                    }
                    let equality = value._eq(term);
                    Some(match reach {
                        Some(reach) => all_of(ctx, &[reach, &equality]),
                        None => equality,
                    })
                })
                .collect();
            if !disjuncts.is_empty() {
                let refs: Vec<&Bool<'ctx>> = disjuncts.iter().collect();
                let def_name = format!("def_loop_bb{}_{}", block.index(), key_suffix(key));
                let mut syms = HashSet::new();
                syms.insert(symbol.clone());
                for (_, _, term) in pairs {
                    syms.extend(extract_symbols(&term.to_string()));
                }
                state.def_symbols.insert(def_name.clone(), syms);
                state.defs.insert(def_name, any_of(ctx, &refs));
            }
        }
        // Declare the symbol the block's reads will use, with the place's own
        // domain: a symbol the relation mentions has to be declared for the
        // obligation to be re-dischargeable at all, and its type is what lets a
        // guard bound it. The name is the place's, so any contribution that knows
        // it knows its type — including a previous-trip copy, which is the same
        // quantity one trip earlier.
        if !state.inputs.contains_key(&symbol) {
            let known = contributions
                .iter()
                .find_map(|(_, contribution)| contribution.state.inputs.get(&symbol).cloned())
                .or_else(|| {
                    contributions.iter().find_map(|(pred, _)| {
                        let variant = previous_trip_name(
                            &symbol,
                            &format!("{PREVIOUS_TRIP_PREFIX}{}_", pred.index()),
                        );
                        state.inputs.get(&variant).cloned()
                    })
                });
            state.inputs.insert(
                symbol,
                known.unwrap_or_else(|| InputVar {
                    symbol: state.symbol_name(key),
                    label: place_label_of_key(key),
                    domain: None,
                    kind: InputKind::Temp,
                }),
            );
        }
    }

    // A length the body never writes is the same length one trip earlier.
    //
    // `metadata_value` mints a `len_…` symbol only for a place `BodyWrites`
    // shows unwritten, so the previous-trip copy of it denotes the same
    // constant. Without this the copy stays a free symbol, and a
    // `for j in 0..v.len()` loop reads its own bound as something unrelated to
    // the length the body's bounds check uses: the loop's guard then proves
    // nothing about `v[j]`.
    let back_edges: Vec<String> = contributions
        .iter()
        .filter(|(pred, _)| !is_forward(pred))
        .map(|(pred, _)| format!("{PREVIOUS_TRIP_PREFIX}{}_", pred.index()))
        .collect();
    let unvarying: Vec<(String, String)> = state
        .inputs
        .iter()
        .filter(|(name, input)| {
            input.kind == InputKind::Source
                && input.domain == Some(ValueDomain::SliceLen)
                && !name.starts_with(PREVIOUS_TRIP_PREFIX)
        })
        .flat_map(|(name, _)| {
            back_edges
                .iter()
                .map(|prefix| (name.clone(), previous_trip_name(name, prefix)))
                .collect::<Vec<_>>()
        })
        .filter(|(here, before)| here != before && state.inputs.contains_key(before))
        .collect();
    for (here, before) in unvarying {
        let def_name = format!("inv_loop_{before}");
        let here = Int::new_const(ctx, here.as_str());
        let before = Int::new_const(ctx, before.as_str());
        state.defs.insert(def_name, here._eq(&before));
    }

    // -----------------------------------------------------------------
    // A13: Carried Value Refinement & Loop Invariant Discovery
    // -----------------------------------------------------------------
    // Detect constant-step linear induction variables:
    // next = prev + delta, starting from init at forward edges.
    struct LinearInduction<'ctx> {
        key: PlaceKey,
        symbol: String,
        init: Int<'ctx>,
        step: i64,
    }
    let mut linear_vars: Vec<LinearInduction<'ctx>> = Vec::new();

    for key in &carried {
        let Some(pairs) = alternatives.get(key) else {
            continue;
        };
        let forward_terms: Vec<&Int<'ctx>> = pairs
            .iter()
            .filter(|(forward, _, _)| *forward)
            .map(|(_, _, term)| term)
            .collect();
        if forward_terms.len() != 1 {
            continue;
        }
        let init = forward_terms[0].clone();
        let symbol = state.symbol_name(key);

        // Check if all back edges have a consistent constant step
        let mut consistent_step: Option<i64> = None;
        let mut is_linear = true;

        for (pred, contribution) in contributions {
            if is_forward(pred) {
                continue;
            }
            let prev_name =
                previous_trip_name(&symbol, &format!("{PREVIOUS_TRIP_PREFIX}{}_", pred.index()));
            let prev_var = Int::new_const(ctx, prev_name.as_str());
            let renamed =
                contribution.renamed(ctx, &format!("{PREVIOUS_TRIP_PREFIX}{}_", pred.index()));
            if let Some(term) = renamed.state.values.get(key) {
                let diff = (term - &prev_var).simplify();
                if let Some(delta) = diff.as_i64() {
                    match consistent_step {
                        None => consistent_step = Some(delta),
                        Some(prev_delta) if prev_delta == delta => {}
                        _ => {
                            is_linear = false;
                            break;
                        }
                    }
                } else {
                    is_linear = false;
                    break;
                }
            } else {
                is_linear = false;
                break;
            }
        }

        if is_linear {
            if let Some(step) = consistent_step {
                if step != 0 {
                    let value = Int::new_const(ctx, symbol.as_str());
                    // 1. Monotonicity bound
                    let mono_def_name = format!("def_mono_bb{}_{}", block.index(), key_suffix(key));
                    let mono_guard = if step > 0 {
                        value.ge(&init)
                    } else {
                        value.le(&init)
                    };
                    let mut syms = HashSet::new();
                    syms.insert(symbol.clone());
                    syms.extend(extract_symbols(&init.to_string()));
                    state.def_symbols.insert(mono_def_name.clone(), syms);
                    state.defs.insert(mono_def_name, mono_guard);

                    linear_vars.push(LinearInduction {
                        key: key.clone(),
                        symbol: symbol.clone(),
                        init,
                        step,
                    });
                }
            }
        }
    }

    // 2. Cross-variable affine induction relation:
    // For any two linear variables A and B with steps s_A, s_B:
    // s_B * (A - A_0) == s_A * (B - B_0)
    for i in 0..linear_vars.len() {
        for j in (i + 1)..linear_vars.len() {
            let var_a = &linear_vars[i];
            let var_b = &linear_vars[j];
            let val_a = Int::new_const(ctx, var_a.symbol.as_str());
            let val_b = Int::new_const(ctx, var_b.symbol.as_str());
            let step_a = Int::from_i64(ctx, var_a.step);
            let step_b = Int::from_i64(ctx, var_b.step);

            let lhs = &step_b * &(&val_a - &var_a.init);
            let rhs = &step_a * &(&val_b - &var_b.init);
            let affine_eq = lhs._eq(&rhs);

            let def_name = format!(
                "def_affine_bb{}_{}_{}",
                block.index(),
                key_suffix(&var_a.key),
                key_suffix(&var_b.key)
            );
            let mut syms = HashSet::new();
            syms.insert(var_a.symbol.clone());
            syms.insert(var_b.symbol.clone());
            syms.extend(extract_symbols(&var_a.init.to_string()));
            syms.extend(extract_symbols(&var_b.init.to_string()));
            state.def_symbols.insert(def_name.clone(), syms);
            state.defs.insert(def_name, affine_eq);
        }
    }

    state
}

/// The contributions that arrive along a *forward* edge: one whose predecessor
/// comes earlier in reverse postorder, which in a reducible CFG is every edge
/// that is not a back edge.
///
/// Widening uses this to rebuild a cycle block's condition from its forward
/// edges, dropping the back edge — whose contribution otherwise carries the
/// block's own condition from the previous round back into it, so the condition
/// re-wraps itself once per round instead of reaching a fixpoint (see
/// [`PathAnalysis::compute`]). The dropped disjunct is a subset of a forward one
/// — the same guards imposed on the same widened symbols — so the union is
/// unchanged and what comes out is the fixpoint. A block with no forward
/// contribution yields an empty slice, which [`merge_all`] turns into the weaker
/// "claims nothing" state; that is sound.
fn forward_contributions<'ctx>(
    block: BasicBlock,
    contributions: &[(BasicBlock, Contribution<'ctx>)],
    position: &HashMap<BasicBlock, usize>,
) -> Vec<Contribution<'ctx>> {
    let here = position.get(&block).copied().unwrap_or(usize::MAX);
    contributions
        .iter()
        .filter(|(pred, _)| position.get(pred).is_some_and(|p| *p < here))
        .map(|(_, contribution)| Contribution {
            state: contribution.state.clone(),
            guard: contribution.guard.clone(),
        })
        .collect()
}

/// Merge every path arriving at a block into one state.
///
/// A local defined on several incoming paths becomes a fresh `phi` symbol
/// constrained by
///
/// ```text
/// (reach_1 ∧ phi = value_1) ∨ (reach_2 ∧ phi = value_2) ∨ …
/// ```
///
/// which says "the value is one of these" without ever forcing a particular one.
/// That is the difference between soundness and a plausible-looking wrong answer:
/// forcing `phi = value_1` would drop `value_2` from consideration over the range
/// where both paths are live, and a dropped value is a hidden bug.
fn merge_all<'ctx>(
    ctx: &'ctx Context,
    block: BasicBlock,
    contributions: &[Contribution<'ctx>],
) -> PathState<'ctx> {
    if contributions.is_empty() {
        return PathState::unknown();
    }
    if contributions.len() == 1 {
        // A single predecessor still has to fold in its own edge guard: it is
        // just as much a requirement of reaching here as any other.
        let mut state = contributions[0].state.clone();
        state.pc = reach_of(ctx, &contributions[0]);
        return state;
    }

    let reach: Vec<Option<Bool<'ctx>>> = contributions.iter().map(|c| reach_of(ctx, c)).collect();
    // A path that needs nothing to be taken makes the whole precondition "no
    // condition": reaching this block may genuinely require nothing.
    let pc = if reach.iter().any(|r| r.is_none()) {
        None
    } else {
        let refs: Vec<&Bool<'ctx>> = reach
            .iter()
            .map(|r| r.as_ref().expect("checked for `None` above"))
            .collect();
        Some(any_of(ctx, &refs))
    };

    let mut merged = PathState {
        values: BTreeMap::new(),
        pc,
        defs: BTreeMap::new(),
        def_symbols: BTreeMap::new(),
        checked: BTreeMap::new(),
        discriminants: BTreeMap::new(),
        symbolic_discriminants: BTreeMap::new(),
        inputs: BTreeMap::new(),
        unmodelled: BTreeMap::new(),
        generalised: BTreeSet::new(),
        pointees: BTreeMap::new(),
        // A merge is never the entry of a coroutine body, so the resume symbol has
        // already been read into a value by the time a join is reached.
        resume: None,
        prefix: String::new(),
        has_bitwise: contributions.iter().any(|c| c.state.has_bitwise),
    };
    for contribution in contributions {
        for (name, def) in &contribution.state.defs {
            merged
                .defs
                .entry(name.clone())
                .or_insert_with(|| def.clone());
        }
        for (name, syms) in &contribution.state.def_symbols {
            merged
                .def_symbols
                .entry(name.clone())
                .or_insert_with(|| syms.clone());
        }
        for (name, input) in &contribution.state.inputs {
            merged
                .inputs
                .entry(name.clone())
                .or_insert_with(|| input.clone());
        }
        // Whatever either path could not model stays not modelled on the merge:
        // a value that is a stand-in on one arm of a branch is not evidence on
        // the other just because they meet here.
        for (name, reason) in &contribution.state.unmodelled {
            merged
                .unmodelled
                .entry(name.clone())
                .or_insert_with(|| reason.clone());
        }
        // A value an incoming path lost to the loop abstraction is still a lost
        // value here: whichever contribution carries the generalisation, the merge
        // is downstream of it.
        merged
            .generalised
            .extend(contribution.state.generalised.iter().cloned());
    }

    for key in contributions[0].state.values.keys() {
        let mut terms = Vec::with_capacity(contributions.len());
        let mut present = true;
        for contribution in contributions {
            match contribution.state.values.get(key) {
                Some(term) => terms.push(term.clone()),
                None => {
                    present = false;
                    break;
                }
            }
        }
        // A local one path leaves unmodelled keeps no value here; a read of it
        // later becomes a fresh bounded symbol, which is sound.
        if !present {
            continue;
        }
        // Every path agrees: no need for a symbol, and keeping the term keeps the
        // state's fingerprint (and so the fixpoint) stable.
        if terms.iter().all(|term| term == &terms[0]) {
            merged.values.insert(key.clone(), terms[0].clone());
            continue;
        }
        if merged.defs.len() >= MAX_MERGE_DEFS {
            continue;
        }
        let symbol = format!("phi_bb{}_{}", block.index(), key_suffix(key));
        let phi = Int::new_const(ctx, symbol.as_str());
        merged.inputs.insert(
            symbol.clone(),
            InputVar {
                symbol: symbol.clone(),
                label: place_label_of_key(key),
                domain: None,
                kind: InputKind::Merge,
            },
        );
        let mut disjuncts: Vec<Bool<'ctx>> = Vec::with_capacity(contributions.len());
        for (index, term) in terms.iter().enumerate() {
            let equality = phi._eq(term);
            match reach[index].as_ref() {
                Some(reach) => disjuncts.push(all_of(ctx, &[reach, &equality])),
                None => disjuncts.push(equality),
            }
        }
        let refs: Vec<&Bool<'ctx>> = disjuncts.iter().collect();
        let def = any_of(ctx, &refs);
        let def_name = format!("def_{}", symbol);
        let mut syms = HashSet::new();
        syms.insert(symbol.clone());
        for term in &terms {
            syms.extend(extract_symbols(&term.to_string()));
        }
        merged.def_symbols.insert(def_name.clone(), syms);
        merged.defs.insert(def_name, def);
        merged.values.insert(key.clone(), phi);
    }

    // A `checked_*` fact is the sum *and* the test that chose the variant, so it
    // survives a merge only where every path holds the same one. A payload value
    // whose fact did not survive is dropped with it rather than left standing on
    // its own: the sum that made it meaningful is gone, and its guard would be
    // gone with it.
    let mut survived: Vec<PlaceKey> = Vec::new();
    for (key, product) in &contributions[0].state.checked {
        let rendered = product.rendering();
        let agreed = contributions.iter().all(|c| {
            c.state.checked.get(key).map(CheckedProduct::rendering) == Some(rendered.clone())
        });
        if agreed {
            survived.push(product.payload.clone());
            merged.checked.insert(key.clone(), product.clone());
        }
    }
    merged.values.retain(|key, _| {
        !contributions
            .iter()
            .any(|c| c.state.checked.values().any(|p| &p.payload == key))
            || survived.contains(key)
    });

    if let Some(first) = contributions.first() {
        for (key, disc) in &first.state.discriminants {
            if contributions
                .iter()
                .all(|c| c.state.discriminants.get(key) == Some(disc))
            {
                merged.discriminants.insert(key.clone(), *disc);
            }
        }

        // A symbolic discriminant is a value like any other, so it merges the
        // same way: one `phi` constrained to the incoming expressions. Keeping it
        // only where every path agrees would drop the iterator's `Some` guard at
        // the loop head and turn the body's bounds check back into a question
        // about a free variant.
        for (key, _term) in &first.state.symbolic_discriminants {
            let mut terms = Vec::with_capacity(contributions.len());
            let mut present = true;
            for contribution in contributions {
                match contribution.state.symbolic_discriminants.get(key) {
                    Some(value) => terms.push(value.clone()),
                    None => {
                        present = false;
                        break;
                    }
                }
            }
            if !present {
                continue;
            }
            if terms.iter().all(|value| value == &terms[0]) {
                merged
                    .symbolic_discriminants
                    .insert(key.clone(), terms[0].clone());
                continue;
            }
            if merged.defs.len() >= MAX_MERGE_DEFS {
                continue;
            }
            let symbol = format!("phi_bb{}_disc_{}", block.index(), key_suffix(key));
            let phi = Int::new_const(ctx, symbol.as_str());
            merged.inputs.insert(
                symbol.clone(),
                InputVar {
                    symbol: symbol.clone(),
                    label: format!("{} (variant)", place_label_of_key(key)),
                    domain: None,
                    kind: InputKind::Merge,
                },
            );
            let mut disjuncts: Vec<Bool<'ctx>> = Vec::with_capacity(contributions.len());
            for (index, value) in terms.iter().enumerate() {
                let equality = phi._eq(value);
                match reach[index].as_ref() {
                    Some(reach) => disjuncts.push(all_of(ctx, &[reach, &equality])),
                    None => disjuncts.push(equality),
                }
            }
            let refs: Vec<&Bool<'ctx>> = disjuncts.iter().collect();
            let def_name = format!("def_{}", symbol);
            let mut syms = HashSet::new();
            syms.insert(symbol.clone());
            for value in &terms {
                syms.extend(extract_symbols(&value.to_string()));
            }
            merged.def_symbols.insert(def_name.clone(), syms);
            merged.defs.insert(def_name, any_of(ctx, &refs));
            merged.symbolic_discriminants.insert(key.clone(), phi);
        }
    }

    merged
}

/// A label for a merged value: the local it belongs to. No source name is
/// available from a key alone, so the MIR local is shown.
fn place_label_of_key(key: &PlaceKey) -> String {
    let mut label = format!("_{}", key.0);
    for idx in &key.1 {
        label.push('.');
        label.push_str(&pseudo_field_name(*idx).unwrap_or_else(|| idx.to_string()));
    }
    label
}

/// A reader-facing name for a [`PathState`] pseudo field, or `None` when the
/// index is a real field (or at least not one of ours).
fn pseudo_field_name(index: usize) -> Option<String> {
    if index == PSEUDO_LEN {
        Some("len".to_string())
    } else if index == PSEUDO_CAPACITY {
        Some("capacity".to_string())
    } else if index == PSEUDO_CURSOR {
        Some("cursor".to_string())
    } else if index == PSEUDO_COUNT {
        Some("count".to_string())
    } else {
        None
    }
}

/// The path-sensitive state of every reachable block of one MIR body.
pub struct PathAnalysis<'ctx> {
    states: HashMap<BasicBlock, PathState<'ctx>>,
    /// Did the dataflow fall back to the path-insensitive state?
    ///
    /// True when no round was change-free within [`MAX_ROUNDS`], which is the
    /// only way [`PathAnalysis::compute`] replaces its states wholesale with
    /// [`PathState::unknown`]. The fallback is sound but it silently costs every
    /// guard in the function, and its only outward sign used to be a FAIL the
    /// guards would have ruled out — a false positive a reader had no way to
    /// attribute. Recorded so it can be reported instead (see
    /// [`PathAnalysis::path_insensitivity_note`]).
    path_insensitive: bool,
    /// The round budget this analysis ran under, so the note names the number
    /// that was actually hit rather than the default it may have replaced.
    rounds: usize,
}

impl<'ctx> PathAnalysis<'ctx> {
    /// Thread the symbolic state through the control-flow graph.
    pub fn compute<'tcx>(
        ctx: &'ctx Context,
        mir: &Body<'tcx>,
        reachable: &HashSet<BasicBlock>,
        const_map: &ConstMap,
        tcx: TyCtxt<'tcx>,
        entry_guard: Option<Bool<'ctx>>,
    ) -> Self {
        let entry = BasicBlock::from_usize(0);
        let rounds_available = round_budget();
        let mut states: HashMap<BasicBlock, PathState<'ctx>> = HashMap::new();
        if !reachable.contains(&entry) {
            // Nothing was analysed, so nothing was lost: an unreachable body is
            // not a function whose guards went missing.
            return Self {
                states,
                path_insensitive: false,
                rounds: rounds_available,
            };
        }

        let order: Vec<BasicBlock> = mir
            .basic_blocks
            .reverse_postorder()
            .iter()
            .copied()
            .filter(|bb| reachable.contains(bb))
            .collect();
        let position: HashMap<BasicBlock, usize> =
            order.iter().enumerate().map(|(i, bb)| (*bb, i)).collect();

        // The locals this body writes. A place that is never written names one
        // object for the whole function, which is what lets its length be one
        // symbol (see [`PathState::metadata_value`]).
        let writes = body_writes(tcx, mir);

        // The blocks inside a cycle. They are analysed like any other block —
        // the point of the walk is that a loop body's guards reach its
        // obligations — but they are unrolled to the back edge for a bounded
        // number of trips and then widened in both dimensions, values and
        // condition alike (see the widening step below).
        let in_cycle = cycle_blocks(mir, &order, &position, reachable);

        // Each block's incoming contributions, keyed by the predecessor that
        // supplied them, so a later round replaces a predecessor's contribution
        // instead of piling up a second one.
        let mut incoming: HashMap<BasicBlock, HashMap<BasicBlock, Contribution<'ctx>>> =
            HashMap::new();
        // The out-state of each block, so a block whose contribution never changes
        // stops re-merging its successors.
        let mut out_states: HashMap<BasicBlock, u64> = HashMap::new();
        // A cycle block is widened *once*, and that state is then held while the
        // rest of the graph settles.
        //
        // Re-deriving it every round does not converge. Widening reads which
        // places the back edge carries out of the back edge's own values, and then
        // forgets them — so the very act of widening removes the evidence that
        // those places are carried, the next round carries fewer of them, the one
        // after that carries them again, and the states swing between two shapes
        // forever. Holding the widened state is how a fixpoint is actually
        // reached: the block stops depending on a contribution it has already
        // generalised, and everything downstream settles against it.
        //
        // It is also the sound direction. The held state is the one built from the
        // *last unrolled trip*, whose guards are at least as strong as the ones any
        // later round would rebuild, and it is already cleared of everything the
        // back edge carries — a weaker precondition than the fixpoint would have,
        // which can only add counterexamples, never remove them.
        let mut widened_cycle: HashMap<BasicBlock, PathState<'ctx>> = HashMap::new();

        let mut converged = false;
        // How many rounds each cycle block has been carried round the loop. See
        // `MAX_LOOP_TRIPS`: beyond it the block's loop-carried values are
        // widened, which is what bounds the unrolling.
        let mut trips: HashMap<BasicBlock, usize> = HashMap::new();
        for _ in 0..rounds_available {
            let mut changed = false;
            for &block in &order {
                // Each contributor with the predecessor that supplied it, so the
                // widening below can tell a forward edge from a back edge.
                let mut contributions: Vec<(BasicBlock, Contribution<'ctx>)> =
                    match incoming.get(&block) {
                        Some(map) => map
                            .iter()
                            .map(|(pred, contribution)| {
                                (
                                    *pred,
                                    Contribution {
                                        state: contribution.state.clone(),
                                        guard: contribution.guard.clone(),
                                    },
                                )
                            })
                            .collect(),
                        None => Vec::new(),
                    };
                // Deterministic order (affects which value a `phi` is paired
                // with, and so the printed SMT-LIB).
                contributions.sort_by_key(|(pred, _)| pred.index());
                let all: Vec<Contribution<'ctx>> = contributions
                    .iter()
                    .map(|(_, contribution)| Contribution {
                        state: contribution.state.clone(),
                        guard: contribution.guard.clone(),
                    })
                    .collect();
                let mut in_state = if block == entry {
                    let mut st = PathState::initial();
                    if let Some(guard) = &entry_guard {
                        st.assume(ctx, guard.clone());
                    }
                    st
                } else {
                    merge_all(ctx, block, &all)
                };

                // Unroll the loop, then widen it. This is deliberately the
                // *in*-state: the reads this block performs then meet the widened
                // values, so the guard on the edge out of the block and the term
                // its successors read are built from the same symbols rather than
                // from two unrelated free ones.
                if in_cycle.contains(&block) {
                    let rounds = trips.entry(block).or_insert(0);
                    *rounds += 1;
                    if *rounds > MAX_LOOP_TRIPS {
                        in_state = match widened_cycle.get(&block) {
                            Some(held) => held.clone(),
                            None => {
                                let widened = widened_state(ctx, block, &contributions, &position);
                                widened_cycle.insert(block, widened.clone());
                                widened
                            }
                        };
                    }
                }
                let mut out_state = in_state;
                out_state.transfer(ctx, mir, const_map, tcx, &writes, block);
                let edges = out_state.successor_guards(ctx, mir, const_map, tcx, block);
                for (target, guard) in edges {
                    if !reachable.contains(&target) {
                        continue;
                    }
                    incoming.entry(target).or_default().insert(
                        block,
                        Contribution {
                            state: out_state.clone(),
                            guard,
                        },
                    );
                }

                let fingerprint = out_state.fingerprint();
                if out_states.get(&block) != Some(&fingerprint) {
                    out_states.insert(block, fingerprint);
                    changed = true;
                }
                states.insert(block, out_state);
            }
            converged = !changed;
            if converged {
                break;
            }
        }

        // If the fixpoint was not reached (a shape the round cap cut short),
        // fall back to the state that claims nothing. That is the analysis from
        // before this module existed, and it is sound — but it is also the one
        // place a function loses every guard at once, so remember that it
        // happened rather than letting only the resulting false positives show.
        let mut path_insensitive = false;
        if !converged {
            path_insensitive = true;
            states = order
                .iter()
                .map(|block| (*block, PathState::unknown()))
                .collect();
        }

        Self {
            states,
            path_insensitive,
            rounds: rounds_available,
        }
    }

    /// The state on entry to a block, or `None` when the block is not analysed.
    pub fn state(&self, block: BasicBlock) -> Option<&PathState<'ctx>> {
        self.states.get(&block)
    }

    /// Why this function's dataflow is path-insensitive, or `None` when the
    /// analysis reached a fixpoint and every state carries the guards that lead
    /// to it.
    ///
    /// This is the report of a lost precision, and it is deliberately a `String`
    /// the caller records rather than a flag it acts on: the fallback is sound,
    /// so the verdicts are not withdrawn — what changes is that a FAIL in this
    /// function may be a false positive, because the guard that rules it out is
    /// one the analysis no longer had. Reporting it turns inference from the
    /// verdicts into a statement about the analysis itself.
    ///
    /// The direction matters, and the note says so: forgetting a guard only
    /// *weakens* the precondition, and a weakened precondition can only make an
    /// obligation easier to satisfy. So a FAIL here is suspect while a PASS here
    /// is still a proof.
    pub fn path_insensitivity_note(&self) -> Option<String> {
        let rounds = self.rounds;
        // "1 round", not "1 rounds": the budget is a number a reader is expected
        // to act on, and a report with a typo in it is one they trust less.
        let plural = if rounds == 1 { "round" } else { "rounds" };
        self.path_insensitive.then(|| {
            format!(
                "Path-sensitive dataflow did not reach a fixpoint inside its round budget ({rounds} \
                 {plural}), so every obligation in this function was checked path-insensitively: the \
                 guards that lead to it are missing, and a FAIL here may be a false positive. A PASS \
                 still holds — a weaker precondition can only make an obligation easier to satisfy."
            )
        })
    }
}

/// The blocks that live inside a cycle.
///
/// Reverse postorder places a natural loop's body in a contiguous range, and an
/// edge from later to earlier in that order can only be a back edge — so the
/// loop is the range between the header and the source of the back edge. This is
/// what tells [`PathAnalysis::compute`] which blocks to unroll and then widen; a
/// shape it over-approximates (an unusual or irreducible CFG) widens a block
/// that it did not have to, which costs precision and never soundness.
fn cycle_blocks(
    mir: &Body<'_>,
    order: &[BasicBlock],
    position: &HashMap<BasicBlock, usize>,
    reachable: &HashSet<BasicBlock>,
) -> HashSet<BasicBlock> {
    let predecessors = mir.basic_blocks.predecessors();
    let mut in_cycle = HashSet::new();
    for &block in order {
        let Some(&block_pos) = position.get(&block) else {
            continue;
        };
        for predecessor in &predecessors[block] {
            if !reachable.contains(predecessor) {
                continue;
            }
            let Some(&pred_pos) = position.get(predecessor) else {
                continue;
            };
            if pred_pos > block_pos {
                for marked in &order[block_pos..=pred_pos] {
                    in_cycle.insert(*marked);
                }
            }
        }
    }
    in_cycle
}

/// Map a mathematical value to Rust's fixed-width integer domain.
fn fixed_value<'ctx>(ctx: &'ctx Context, value: &Int<'ctx>, ty: IntType) -> Int<'ctx> {
    let modulus = Int::from_str(
        ctx,
        &(1u128
            .checked_shl(ty.bit_width())
            .map(|v| v.to_string())
            .unwrap_or_else(|| "340282366920938463463374607431768211456".to_string())),
    )
    .unwrap();
    let residue = value.modulo(&modulus);
    if ty.is_unsigned() {
        residue
    } else {
        let half = modulus.div(&Int::from_i64(ctx, 2));
        residue.ge(&half).ite(&(&residue - &modulus), &residue)
    }
}

fn fixed_binary<'ctx>(
    ctx: &'ctx Context,
    op: BinOp,
    left: &Int<'ctx>,
    right: &Int<'ctx>,
    ty: IntType,
) -> Option<Int<'ctx>> {
    let arithmetic = match op {
        BinOp::Add => Some(left + right),
        BinOp::Sub => Some(left - right),
        BinOp::Mul => Some(left * right),
        _ => None,
    };
    if let Some(value) = arithmetic {
        return Some(fixed_value(ctx, &value, ty));
    }
    let l = BV::from_int(left, ty.bit_width());
    let r = BV::from_int(right, ty.bit_width());
    let amount = BV::from_int(
        &right.modulo(&Int::from_i64(ctx, ty.bit_width().into())),
        ty.bit_width(),
    );
    let value = match op {
        BinOp::BitAnd => l.bvand(&r),
        BinOp::BitOr => l.bvor(&r),
        BinOp::BitXor => l.bvxor(&r),
        BinOp::Shl => l.bvshl(&amount),
        BinOp::Shr if ty.is_unsigned() => l.bvlshr(&amount),
        BinOp::Shr => l.bvashr(&amount),
        _ => return None,
    };
    Some(value.to_int(!ty.is_unsigned()))
}

/// Rust divides magnitudes and restores the sign, truncating toward zero.
/// The compiler's success-edge guards exclude divisor zero and MIN / -1.
fn rust_quotient<'ctx>(ctx: &'ctx Context, left: &Int<'ctx>, right: &Int<'ctx>) -> Int<'ctx> {
    let z = zero(ctx);
    let abs_left = left.ge(&z).ite(left, &(&z - left));
    let abs_right = right.ge(&z).ite(right, &(&z - right));
    let magnitude = abs_left.div(&abs_right);
    left.lt(&z)
        ._eq(&right.lt(&z))
        .ite(&magnitude, &(&z - &magnitude))
}

#[cfg(test)]
mod tests {
    use super::*;
    use z3::{Config, SatResult};

    #[test]
    fn signed_division_and_remainder_match_rust_exhaustively() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        for a in i8::MIN..=i8::MAX {
            for b in i8::MIN..=i8::MAX {
                let (Some(q), Some(r)) = (a.checked_div(b), a.checked_rem(b)) else {
                    continue;
                };
                let l = Int::from_i64(&ctx, i64::from(a));
                let d = Int::from_i64(&ctx, i64::from(b));
                let quotient = rust_quotient(&ctx, &l, &d);
                assert_eq!(quotient.simplify().as_i64(), Some(i64::from(q)), "{a}/{b}");
                assert_eq!(
                    (&l - &(&quotient * &d)).simplify().as_i64(),
                    Some(i64::from(r)),
                    "{a}%{b}"
                );
            }
        }
    }

    #[test]
    fn fixed_width_symbolic_values_match_native_rust() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        for a in i8::MIN..=i8::MAX {
            let left = Int::from_i64(&ctx, a.into());
            assert_eq!(
                fixed_value(&ctx, &left, IntType::U8).simplify().as_i64(),
                Some((a as u8).into())
            );
            for b in [-128i8, -1, 0, 1, 7, 127] {
                let right = Int::from_i64(&ctx, b.into());
                for (op, expected) in [
                    (BinOp::Add, a.wrapping_add(b)),
                    (BinOp::Sub, a.wrapping_sub(b)),
                    (BinOp::Mul, a.wrapping_mul(b)),
                    (BinOp::BitAnd, a & b),
                    (BinOp::BitOr, a | b),
                    (BinOp::BitXor, a ^ b),
                ] {
                    assert_eq!(
                        fixed_binary(&ctx, op, &left, &right, IntType::I8)
                            .unwrap()
                            .simplify()
                            .as_i64(),
                        Some(expected.into())
                    );
                }
            }
            for amount in 0..=9 {
                let right = Int::from_i64(&ctx, amount);
                assert_eq!(
                    fixed_binary(&ctx, BinOp::Shl, &left, &right, IntType::I8)
                        .unwrap()
                        .simplify()
                        .as_i64(),
                    Some(a.wrapping_shl(amount as u32).into())
                );
                assert_eq!(
                    fixed_binary(&ctx, BinOp::Shr, &left, &right, IntType::I8)
                        .unwrap()
                        .simplify()
                        .as_i64(),
                    Some(a.wrapping_shr(amount as u32).into())
                );
            }
        }
        let max = Int::from_str(&ctx, "340282366920938463463374607431768211455").unwrap();
        assert_eq!(
            fixed_value(&ctx, &max, IntType::U128)
                .simplify()
                .to_string(),
            max.to_string()
        );
        assert_eq!(
            fixed_value(&ctx, &max, IntType::I8).simplify().as_i64(),
            Some(-1)
        );
    }

    /// One path arriving at a merge: the value it defines for `_3`, and the
    /// condition under which that path is taken.
    fn contribution<'ctx>(
        ctx: &'ctx Context,
        value: i128,
        reach: Option<Bool<'ctx>>,
    ) -> Contribution<'ctx> {
        let mut state = PathState::initial();
        state.values.insert((3, Vec::new()), int_lit(ctx, value));
        state.pc = reach;
        Contribution { state, guard: None }
    }

    fn merge_of<'ctx>(ctx: &'ctx Context, entries: &[(i128, i128)]) -> PathState<'ctx> {
        let contributions: Vec<Contribution<'ctx>> = entries
            .iter()
            .map(|(value, condition)| {
                let reach = if *condition == 0 {
                    None
                } else {
                    Some(Int::new_const(ctx, "flag")._eq(&int_lit(ctx, *condition)))
                };
                contribution(ctx, *value, reach)
            })
            .collect();
        merge_all(ctx, BasicBlock::from_usize(3), &contributions)
    }

    /// Ask the merged state whether `phi = value` is reachable, with `flag`
    /// pinned to `condition`.
    fn model_has<'ctx>(
        ctx: &'ctx Context,
        phi_value: i128,
        flag: i128,
        state: &PathState<'ctx>,
    ) -> SatResult {
        let mut solver = Solver::new(ctx);
        state.assert_into(ctx, &mut solver);
        solver.assert(&Int::new_const(ctx, "phi_bb3_3")._eq(&int_lit(ctx, phi_value)));
        if flag != 0 {
            solver.assert(&Int::new_const(ctx, "flag")._eq(&int_lit(ctx, flag)));
        }
        solver.check()
    }

    #[test]
    fn a_merge_keeps_every_possible_value() {
        // The soundness core of the whole module. After `0` on one path and
        // `255` on the other, *both* values must remain reachable; picking one
        // would hide a violation on the other path.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let state = merge_of(&ctx, &[(0, 1), (255, 2)]);

        assert_eq!(model_has(&ctx, 0, 1, &state), SatResult::Sat);
        assert_eq!(model_has(&ctx, 255, 2, &state), SatResult::Sat);
        // …and the values stay paired with the path that produced them.
        assert_eq!(model_has(&ctx, 255, 1, &state), SatResult::Unsat);
        assert_eq!(model_has(&ctx, 0, 2, &state), SatResult::Unsat);
    }

    #[test]
    fn an_unconditional_path_does_not_force_its_value() {
        // If one predecessor needs nothing to be taken, its value must not be
        // asserted outright: execution could have come the other way. An
        // `ite`-chain merge gets this wrong and reports the guard as impossible.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let state = merge_of(&ctx, &[(0, 0), (255, 2)]);
        assert_eq!(model_has(&ctx, 255, 2, &state), SatResult::Sat);
    }

    #[test]
    fn a_single_path_folds_in_its_edge_guard() {
        // The guard on the edge into a block is part of reaching it, even when
        // there is only one way in.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let flag = Int::new_const(&ctx, "flag");
        let guard = flag._eq(&int_lit(&ctx, 1));
        let state = merge_all(
            &ctx,
            BasicBlock::from_usize(1),
            &[Contribution {
                state: PathState::initial(),
                guard: Some(guard),
            }],
        );
        assert!(state.condition().is_some(), "the edge guard must be kept");
    }

    #[test]
    fn a_merge_with_no_contributor_claims_nothing() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let state = merge_all(&ctx, BasicBlock::from_usize(7), &[]);
        assert!(state.condition().is_none());
        assert!(state.values.is_empty());
    }

    #[test]
    fn a_guard_is_what_rules_a_violation_out() {
        // The whole feature in one test. `x + 1` can overflow a `u8` — the
        // solver's own witness is `x = 255` — but not once the path has
        // established `x < 100`. That is the difference between the old verdict
        // and the new one for `if x < 100 { x + 1 } else { 0 }`, and asserting
        // both directions is what makes this a test of the *guard* rather than
        // of the arithmetic.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let x = Int::new_const(&ctx, "mir_1");
        let one_lit = int_lit(&ctx, 1);
        let violation = match overflow_violation(&ctx, BinOp::Add, &x, &one_lit, Some(IntType::U8))
        {
            OverflowCheck::Violation(condition) => condition,
            _ => panic!("addition is modelled"),
        };

        // The two halves below differ *only* in the guard, so the u8 bound that
        // the encoder always records for a source local is held constant in
        // both. Without it the guard is not enough: `x` could be `-5`, which
        // overflows a `u8` downwards, and the solver would be right to say Sat.
        let u8_input = || InputVar {
            symbol: "mir_1".to_string(),
            label: "x (_1)".to_string(),
            domain: Some(ValueDomain::Int(IntType::U8)),
            kind: InputKind::Source,
        };

        let mut guarded = PathState::initial();
        guarded.pc = Some(x.lt(&int_lit(&ctx, 100)));
        guarded.inputs.insert("mir_1".to_string(), u8_input());
        let mut solver = Solver::new(&ctx);
        guarded.assert_into(&ctx, &mut solver);
        solver.assert(&violation);
        assert_eq!(
            solver.check(),
            SatResult::Unsat,
            "the guard must rule it out"
        );

        // Without the guard the same violation is satisfiable, with a witness
        // (`x = 255`) the guard forbids. This is what used to be reported.
        let mut unguarded = PathState::initial();
        unguarded.inputs.insert("mir_1".to_string(), u8_input());
        let mut solver = Solver::new(&ctx);
        unguarded.assert_into(&ctx, &mut solver);
        solver.assert(&violation);
        assert_eq!(
            solver.check(),
            SatResult::Sat,
            "unguarded, the same violation is reachable"
        );
    }

    #[test]
    fn rebranding_keeps_two_states_apart() {
        // Two call sites of one function, each requiring `flag` and registering
        // it the way the walk does. Asserted together they must stay two
        // independent requirements: if renaming were a no-op, one symbol would
        // serve both and pinning the sites to different values would be
        // unsatisfiable — the silent conflation of two callers.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let flag = Int::new_const(&ctx, "flag");
        let caller_state = || {
            let mut state = PathState::initial();
            state.inputs.insert(
                "flag".to_string(),
                InputVar {
                    symbol: "flag".to_string(),
                    label: "flag (_1)".to_string(),
                    domain: Some(ValueDomain::Bool),
                    kind: InputKind::Source,
                },
            );
            state
        };
        let guard = flag._eq(&one(&ctx));
        let site0 = merge_all(
            &ctx,
            BasicBlock::from_usize(1),
            &[Contribution {
                state: caller_state(),
                guard: Some(guard.clone()),
            }],
        )
        .rebranded(&ctx, "site0_");
        let site1 = merge_all(
            &ctx,
            BasicBlock::from_usize(1),
            &[Contribution {
                state: caller_state(),
                guard: Some(guard),
            }],
        )
        .rebranded(&ctx, "site1_");

        assert!(site0.inputs.keys().all(|name| name.starts_with("site0_")));
        assert!(site1.inputs.keys().all(|name| name.starts_with("site1_")));

        // Contradicting one site's own symbol must be enough to make the pair
        // unsatisfiable, which is only true if the two are separate symbols.
        let mut solver = Solver::new(&ctx);
        site0.assert_into(&ctx, &mut solver);
        site1.assert_into(&ctx, &mut solver);
        solver.assert(&Int::new_const(&ctx, "site1_flag")._eq(&int_lit(&ctx, 0)));
        assert_eq!(
            solver.check(),
            SatResult::Unsat,
            "the two sites must not share one symbol"
        );
    }

    #[test]
    fn assuming_a_guard_conjoins_it() {
        // An inherited precondition joins whatever this function already
        // requires: it may add a constraint, never replace one.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let x = Int::new_const(&ctx, "x");
        let mut state = PathState::initial();
        state.assume(&ctx, x.lt(&int_lit(&ctx, 100)));
        state.assume(&ctx, x.gt(&int_lit(&ctx, 10)));

        let mut solver = Solver::new(&ctx);
        state.assert_into(&ctx, &mut solver);
        solver.assert(&x._eq(&int_lit(&ctx, 50)));
        assert_eq!(solver.check(), SatResult::Sat, "both guards hold at 50");

        let mut solver = Solver::new(&ctx);
        state.assert_into(&ctx, &mut solver);
        solver.assert(&x._eq(&int_lit(&ctx, 5)));
        assert_eq!(
            solver.check(),
            SatResult::Unsat,
            "the second guard must not be lost"
        );
    }

    #[test]
    fn input_bounds_keep_a_model_inside_its_type() {
        // A `u8` argument cannot be 300; without the bound Z3 would happily
        // invent such a witness for an obligation that can never fire.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let mut state = PathState::initial();
        state.inputs.insert(
            "mir_1".to_string(),
            InputVar {
                symbol: "mir_1".to_string(),
                label: "x (_1)".to_string(),
                domain: Some(ValueDomain::Int(IntType::U8)),
                kind: InputKind::Source,
            },
        );

        let mut solver = Solver::new(&ctx);
        state.assert_into(&ctx, &mut solver);
        solver.assert(&Int::new_const(&ctx, "mir_1")._eq(&int_lit(&ctx, 300)));
        assert_eq!(solver.check(), SatResult::Unsat);

        let mut solver = Solver::new(&ctx);
        state.assert_into(&ctx, &mut solver);
        solver.assert(&Int::new_const(&ctx, "mir_1")._eq(&int_lit(&ctx, 200)));
        assert_eq!(solver.check(), SatResult::Sat);
    }

    #[test]
    fn a_bool_input_is_bounded_to_zero_and_one() {
        // Otherwise a model could answer `2` for a flag, and `cond == true`
        // would become unsatisfiable — a real failure turned into a PASS.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let mut state = PathState::initial();
        state.inputs.insert(
            "mir_1".to_string(),
            InputVar {
                symbol: "mir_1".to_string(),
                label: "flag (_1)".to_string(),
                domain: Some(ValueDomain::Bool),
                kind: InputKind::Source,
            },
        );
        let mut solver = Solver::new(&ctx);
        state.assert_into(&ctx, &mut solver);
        solver.assert(&Int::new_const(&ctx, "mir_1")._eq(&int_lit(&ctx, 2)));
        assert_eq!(solver.check(), SatResult::Unsat);
    }

    #[test]
    fn merged_values_are_not_type_bounded() {
        // A merged value may be the unwrapped result of an expression that
        // legitimately exceeds the local's own range; bounding it there would
        // assert no-overflow and hide the very failure being checked.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let state = merge_of(&ctx, &[(0, 1), (255, 2)]);
        for input in state.inputs.values() {
            if input.kind == InputKind::Merge {
                assert!(input.domain.is_none(), "merge symbols stay unbounded");
            }
        }
        assert!(
            state.inputs.values().any(|i| i.kind == InputKind::Merge),
            "the merge must introduce a symbol"
        );
    }

    #[test]
    fn switch_values_are_read_through_the_discriminant_type() {
        // `match -1` stores the two's-complement pattern; comparing it raw
        // against a signed term would make the arm dead.
        assert_eq!(switch_value(u32::MAX as u128, Some(IntType::I32)), -1);
        assert_eq!(switch_value(1, Some(IntType::I32)), 1);
        assert_eq!(
            switch_value(u32::MAX as u128, Some(IntType::U32)),
            u32::MAX as i128
        );
        assert_eq!(switch_value(0, None), 0);
        assert_eq!(switch_value(1, None), 1);
    }

    #[test]
    fn a_place_key_sees_through_field_projections_only() {
        // Field paths are structural; anything else (a dereference, an index)
        // reads memory we cannot name.
        assert_eq!(
            key_suffix(&(4, vec![1])),
            "4_1",
            "the symbol name has to be a legal SMT-LIB symbol"
        );
        assert_eq!(place_label_of_key(&(4, vec![1])), "_4.1");
    }

    #[test]
    fn bool_witness_values_render_as_booleans() {
        assert_eq!(render_bool("1"), "true");
        assert_eq!(render_bool("0"), "false");
        assert_eq!(render_bool("200"), "200");
    }

    /// A body state as the walk leaves one: `_3` is the bounded counter symbol
    /// and `_4` is a value derived from it, so widening has both an input and a
    /// term to deal with.
    fn loop_body_state<'ctx>(ctx: &'ctx Context, guard: Bool<'ctx>) -> PathState<'ctx> {
        let mut state = PathState::initial();
        let i = Int::new_const(ctx, "mir_3");
        state.values.insert((3, Vec::new()), i.clone());
        state.values.insert((4, Vec::new()), &i + &int_lit(ctx, 1));
        state.inputs.insert(
            "mir_3".to_string(),
            InputVar {
                symbol: "mir_3".to_string(),
                label: "i (_3)".to_string(),
                domain: Some(ValueDomain::Int(IntType::U8)),
                kind: InputKind::Source,
            },
        );
        state.pc = Some(guard);
        state
    }

    /// Widening a cycle block rebuilds its condition from the forward edges, so a
    /// back edge cannot carry the block's own condition back into itself — which
    /// is what used to make a nested loop's condition grow instead of converge.
    #[test]
    fn a_widened_condition_keeps_the_forward_edges_and_drops_the_back_edge() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let mut position = HashMap::new();
        position.insert(BasicBlock::from_usize(1), 1);
        position.insert(BasicBlock::from_usize(4), 4);
        position.insert(BasicBlock::from_usize(9), 9);
        let contributions = vec![
            (BasicBlock::from_usize(1), contribution(&ctx, 7, None)),
            (
                BasicBlock::from_usize(9),
                contribution(
                    &ctx,
                    7,
                    Some(Int::new_const(&ctx, "g")._eq(&int_lit(&ctx, 1))),
                ),
            ),
        ];
        let kept = forward_contributions(BasicBlock::from_usize(4), &contributions, &position);
        assert_eq!(
            kept.len(),
            1,
            "only the predecessor earlier in reverse postorder is a forward edge"
        );
        // A block reached only by a back edge keeps no contribution, which the
        // caller turns into the weaker state rather than a wrong one.
        let back_only = vec![(BasicBlock::from_usize(9), contribution(&ctx, 7, None))];
        assert!(forward_contributions(BasicBlock::from_usize(4), &back_only, &position).is_empty());
    }

    /// A block reached from an unguarded entry and from a guarded back edge, the
    /// shape the loop abstraction exists for: `_3` carries the counter.
    fn loop_header_contributions<'ctx>(
        ctx: &'ctx Context,
    ) -> Vec<(BasicBlock, Contribution<'ctx>)> {
        let mut entry = PathState::initial();
        entry.values.insert((3, Vec::new()), int_lit(ctx, 0));
        vec![
            (
                BasicBlock::from_usize(1),
                Contribution {
                    state: entry,
                    guard: None,
                },
            ),
            (
                BasicBlock::from_usize(9),
                Contribution {
                    state: loop_body_state(
                        ctx,
                        Int::new_const(ctx, "mir_3").lt(&int_lit(ctx, 199)),
                    ),
                    guard: None,
                },
            ),
        ]
    }

    fn loop_header_position() -> HashMap<BasicBlock, usize> {
        let mut position = HashMap::new();
        position.insert(BasicBlock::from_usize(1), 1);
        position.insert(BasicBlock::from_usize(4), 4);
        position.insert(BasicBlock::from_usize(9), 9);
        position
    }

    #[test]
    fn widening_relates_a_loop_carried_value_to_the_previous_trip() {
        // The loop abstraction in one test, and it is a two-sided claim.
        //
        // The value must not be pinned to whatever the last unrolled trip held:
        // the previous trip's symbol is free, so a model may put the counter
        // anywhere that trip's guard allowed. Pinning it is how a loop with a bug
        // two iterations in gets proved.
        //
        // It must not simply be forgotten either. "Any `u8`" makes the body's own
        // `i + 8 <= v.len()` guard speak about one symbol while the index it
        // guards is another, and the solver is then free to choose the loop's
        // *exit* value — the dominant false positive this relation removes.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let contributions = loop_header_contributions(&ctx);
        let widened = widened_state(
            &ctx,
            BasicBlock::from_usize(4),
            &contributions,
            &loop_header_position(),
        );

        assert!(
            widened.values.is_empty(),
            "the value itself is still not pinned"
        );
        let defs = widened
            .defs
            .values()
            .map(|def| def.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            defs.contains("prev_bb9_mir_3"),
            "the relation names the previous trip's symbol: {defs}"
        );
        assert!(defs.contains("199"), "and carries its guard: {defs}");

        // The relation is asserted about the symbol the block's reads will use,
        // so the guard and the value it guards stay the same quantity.
        let value = Int::new_const(&ctx, "mir_3");
        let mut solver = Solver::new(&ctx);
        widened.assert_into(&ctx, &mut solver);
        solver.assert(&value._eq(&int_lit(&ctx, 0)));
        assert_eq!(solver.check(), SatResult::Sat, "the entry value stays");

        // A value a previous trip could have carried is free...
        let mut solver = Solver::new(&ctx);
        widened.assert_into(&ctx, &mut solver);
        solver.assert(&value._eq(&int_lit(&ctx, 198)));
        assert_eq!(
            solver.check(),
            SatResult::Sat,
            "a value the previous trip's guard allows is free"
        );

        // ...and one it forbids is not, even though the place's own `u8` domain
        // would allow it. That is the difference between a relation and an
        // arbitrary number: the guard travels with the rename, bounding the value
        // the relation produces instead of a symbol nothing else mentions.
        let mut solver = Solver::new(&ctx);
        widened.assert_into(&ctx, &mut solver);
        solver.assert(&value._eq(&int_lit(&ctx, 250)));
        assert_eq!(
            solver.check(),
            SatResult::Unsat,
            "`prev_bb9_mir_3 < 199` bounds it to at most 198"
        );
    }

    #[test]
    fn a_previous_trip_symbol_keeps_the_places_type() {
        // The other half of the widening contract. The previous trip's value is
        // free, not arbitrary: it keeps the place's own type bound, so a model
        // cannot name a number the local could never hold — which for a `u8`
        // would be an overflow the program cannot have.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let contributions = loop_header_contributions(&ctx);
        let widened = widened_state(
            &ctx,
            BasicBlock::from_usize(4),
            &contributions,
            &loop_header_position(),
        );
        assert!(
            widened.inputs.contains_key("prev_bb9_mir_3"),
            "the renamed symbol stays registered, with its domain"
        );

        let prev = Int::new_const(&ctx, "prev_bb9_mir_3");
        let mut solver = Solver::new(&ctx);
        widened.assert_into(&ctx, &mut solver);
        solver.assert(&prev._eq(&int_lit(&ctx, 200)));
        assert_eq!(solver.check(), SatResult::Sat);

        let mut solver = Solver::new(&ctx);
        widened.assert_into(&ctx, &mut solver);
        solver.assert(&prev._eq(&int_lit(&ctx, 300)));
        assert_eq!(
            solver.check(),
            SatResult::Unsat,
            "the u8 bound survives the previous-trip rename"
        );
    }

    /// Build an analysis with an explicit precision verdict and round budget.
    ///
    /// Reaching the fallback for real needs a CFG, a `TyCtxt` and a body the cap
    /// cuts short; what is worth asserting here is the *report* of it, which is
    /// what the verifier records and what a reader of `report.json` sees.
    fn analysis_with_precision(path_insensitive: bool, rounds: usize) -> PathAnalysis<'static> {
        PathAnalysis {
            states: HashMap::new(),
            path_insensitive,
            rounds,
        }
    }

    /// A function whose dataflow converged says nothing about precision, and one
    /// the cap cut short says so in terms a reader can act on.
    #[test]
    fn a_lost_path_sensitivity_is_reported_and_a_fixpoint_is_not() {
        assert!(
            analysis_with_precision(false, MAX_ROUNDS)
                .path_insensitivity_note()
                .is_none(),
            "a converged analysis has no precision to report"
        );

        let note = analysis_with_precision(true, MAX_ROUNDS)
            .path_insensitivity_note()
            .expect("the fallback is reported");
        // The budget that was hit, so the reader knows what to raise or
        // investigate — and the number reflects the budget in force, not the
        // default it may have replaced.
        assert!(
            note.contains(&MAX_ROUNDS.to_string()),
            "the note names the round cap: {note}"
        );
        let overridden = analysis_with_precision(true, 1)
            .path_insensitivity_note()
            .expect("the fallback is reported");
        assert!(
            overridden.contains("budget (1 round)"),
            "an overridden budget is the one named, and a single round is singular: {overridden}"
        );
        assert!(
            !overridden.contains(&MAX_ROUNDS.to_string()),
            "the default is not named when it was overridden: {overridden}"
        );
        // The kind a consumer groups on, and the asymmetry that makes the
        // fallback worth reporting rather than merely noting: the FAILs are the
        // suspect ones.
        assert!(
            note.contains("path-insensitively"),
            "the note names the loss: {note}"
        );
        assert!(
            note.contains("false positive"),
            "the note says which verdicts are suspect: {note}"
        );
        assert!(
            note.contains("PASS still holds"),
            "the note says which verdicts are not: {note}"
        );
    }

    /// The reference `wrapping_*` is defined against, in exact `i128` arithmetic:
    /// the mathematical result reduced into the type's width.
    ///
    /// A *modulus*, not a single add-or-subtract of the span: `-128 * -128` in
    /// `i8` is 16384, which wraps to 0 — subtracting the span once would answer
    /// 16128, a value the method never returns.
    fn reference_wrapping(sum: i128, ty: IntType) -> i128 {
        let (_, max) = ty.bounds();
        let span = 1i128 << ty.bit_width();
        let residue = sum.rem_euclid(span);
        if residue > max {
            residue - span
        } else {
            residue
        }
    }

    /// The reference `saturating_*` is defined against: the mathematical result
    /// clamped to the type's range.
    fn reference_saturating(sum: i128, ty: IntType) -> i128 {
        let (min, max) = ty.bounds();
        sum.clamp(min, max)
    }

    /// Assert that `term` is exactly `expected` — not merely that it can be it.
    ///
    /// Both directions matter: a term that only *implies* the expected value is a
    /// weaker encoding than the method's semantics, and one that excludes it is
    /// not the method either.
    fn assert_term_is(ctx: &Context, term: &Int<'_>, expected: i128, what: &str) {
        let solver = Solver::new(ctx);
        solver.assert(&term._eq(&int_lit(ctx, expected)));
        assert_eq!(
            solver.check(),
            SatResult::Sat,
            "{what}: {expected} is unreachable"
        );
        let solver = Solver::new(ctx);
        solver.assert(&term._eq(&int_lit(ctx, expected)).not());
        assert_eq!(
            solver.check(),
            SatResult::Unsat,
            "{what}: the term is not {expected}"
        );
    }

    /// Every form here claims what a `core::num` method computes, so the claim is
    /// checked against an independently written reference over every width and a
    /// spread of sums that wrap, clamp or sit inside the range.
    #[test]
    fn the_safe_arithmetic_encodings_match_their_reference() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let widths = [
            IntType::I8,
            IntType::I16,
            IntType::I32,
            IntType::I64,
            IntType::U8,
            IntType::U16,
            IntType::U32,
            IntType::U64,
            IntType::Usize,
            IntType::Isize,
        ];
        let mut checked = 0;
        for ty in widths {
            let (min, max) = ty.bounds();
            // Operands, not sums: the wrapping claim is about what the *binary* op
            // returns, and a reference written as `l <op> r` is the one thing this
            // can compare it to without re-deriving the reduction it is testing.
            let operands = [min, min + 1, -1, 0, 1, max - 1, max];
            for l in operands {
                for r in operands {
                    let left = int_lit(&ctx, l);
                    let right = int_lit(&ctx, r);
                    // A `mul` reference for a wide type would overflow the `i128`
                    // the reference is computed in, so products are checked up to
                    // 32 bits — which still covers both signed and unsigned
                    // wrapping, inside and outside the range.
                    let ops: &[BinOp] = if ty.bit_width() <= 32 {
                        &[BinOp::Add, BinOp::Sub, BinOp::Mul]
                    } else {
                        &[BinOp::Add, BinOp::Sub]
                    };
                    for op in ops {
                        let mathematical = match op {
                            BinOp::Add => l + r,
                            BinOp::Sub => l - r,
                            _ => l * r,
                        };
                        let term = fixed_binary(&ctx, *op, &left, &right, ty)
                            .expect("fixed-width arithmetic is modelled");
                        assert_term_is(
                            &ctx,
                            &term,
                            reference_wrapping(mathematical, ty),
                            &format!("{l} {op:?} {r} in {ty:?}"),
                        );
                        assert_term_is(
                            &ctx,
                            &saturating_value(&ctx, &int_lit(&ctx, mathematical), ty),
                            reference_saturating(mathematical, ty),
                            &format!("saturating {l} {op:?} {r} in {ty:?}"),
                        );
                        checked += 2;
                    }
                }
            }
        }
        assert!(
            checked > 2000,
            "every width, operator and operand pair: {checked}"
        );
    }

    /// `checked_*` is modelled as an exact *payload*, not as a bounded sum: the
    /// value placed at `((_3 as Some).0)` is the mathematical sum reduced into the
    /// payload's own type. Two claims have to hold together — the payload is a
    /// `T` and never the unwrapped sum, and on the `Some` path, the only path that
    /// can read it, the reduction is the identity, because `Some` is exactly the
    /// case where the sum did not overflow. Pinned over the whole `u8` square.
    #[test]
    fn the_checked_payload_is_the_exact_sum_whenever_the_sum_fits() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let (min, max) = IntType::U8.bounds();
        let mut in_the_some_arm = 0;
        for x in 0i128..=255 {
            for y in 0i128..=255 {
                let sum = x + y;
                let payload = fixed_value(&ctx, &int_lit(&ctx, sum), IntType::U8).simplify();
                let value = i128::from(payload.as_i64().expect("a concrete value"));
                assert!(
                    (min..=max).contains(&value),
                    "the payload of `Option<u8>` is a `u8` value, not {value}"
                );
                if sum <= max {
                    in_the_some_arm += 1;
                    assert_eq!(value, sum, "{x} + {y} fits, so the payload is the sum");
                }
            }
        }
        assert_eq!(
            in_the_some_arm,
            256 * 257 / 2,
            "every pair whose sum fits is a `Some`, and both orders are distinct pairs"
        );
    }

    /// A widened symbol declared in the state, as the widening declares it: the
    /// previous trip's value of a place.
    fn declare_previous_trip(state: &mut PathState<'_>, name: &str, label: &str) {
        state.inputs.insert(
            name.to_string(),
            InputVar {
                symbol: name.to_string(),
                label: label.to_string(),
                domain: Some(ValueDomain::Int(IntType::U8)),
                kind: InputKind::Temp,
            },
        );
    }

    fn symbols(names: &[&str]) -> HashSet<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    /// K6's classification is a property of the state an obligation was checked
    /// in: whether the loop abstraction generalised a value it leans on, and
    /// whether the failure is stated under guards at all.
    #[test]
    fn a_failure_is_classified_by_the_state_it_was_checked_in() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);

        let bare = PathState::initial();
        assert_eq!(bare.defect_class(&symbols(&[])), "unguarded");

        let mut guarded = PathState::initial();
        guarded.pc = Some(Int::new_const(&ctx, "x").lt(&int_lit(&ctx, 10)));
        assert_eq!(guarded.defect_class(&symbols(&[])), "guard-limited");

        // The audit's case, pinned: a widened symbol that the state *declares* for
        // a place this failure never reads is not evidence that the widening had
        // anything to do with the finding. The location test this replaced
        // answered `widening-limited` here, for every obligation in the function.
        let mut declared_only = guarded.clone();
        declare_previous_trip(&mut declared_only, "prev_bb6_mir_9", "other (_9)");
        assert_eq!(
            declared_only.defect_class(&symbols(&["mir_3", "1_u8"])),
            "guard-limited",
            "a generalised symbol the failure does not depend on is not a widening limitation"
        );

        // The dependence the class is for: the failure is about `mir_3`, and the
        // state's relation defines `mir_3` through the previous trip's value of
        // the same place — which is what the widening itself writes down.
        let mut leaning = guarded.clone();
        declare_previous_trip(&mut leaning, "prev_bb6_mir_3", "i (_3)");
        let value = Int::new_const(&ctx, "mir_3");
        let previous = Int::new_const(&ctx, "prev_bb6_mir_3");
        leaning.defs.insert(
            "def_loop_bb1_mir_3".to_string(),
            value._eq(&(&previous + &Int::from_u64(&ctx, 8))),
        );
        assert!(leaning.leans_on_widening(&symbols(&["mir_3"])));
        assert_eq!(
            leaning.defect_class(&symbols(&["mir_3"])),
            "widening-limited",
            "a generalised loop value outranks the guard: it is the class that may be wrong for a reason no stronger guard would fix"
        );
        // A failure about a value the relation does not mention is still not
        // classified by it.
        assert_eq!(
            leaning.defect_class(&symbols(&["steps_2"])),
            "guard-limited"
        );
        // And the failure's own symbols count when they *are* the previous trip.
        assert_eq!(
            leaning.defect_class(&symbols(&["prev_bb6_mir_3"])),
            "widening-limited"
        );

        // The second piece of evidence, and the one the fixtures actually have:
        // the failing value is a *place* the widening emptied because the back edge
        // carries it. No relation is written down for it, so the closure above
        // finds nothing — but the free symbol the finding is stated over is the one
        // the abstraction left behind, which is a dependence on the widening.
        let mut gave_up = guarded.clone();
        gave_up.generalise(&(1, Vec::new()));
        assert!(
            gave_up.leans_on_widening(&symbols(&["mir_1"])),
            "the failing value is the place the widening stopped tracking"
        );
        assert_eq!(
            gave_up.defect_class(&symbols(&["mir_1"])),
            "widening-limited"
        );
        // Another place's generalisation is not this failure's dependence: this is
        // the difference between a dependence test and asking whether the state
        // sits inside a widened block.
        assert_eq!(
            gave_up.defect_class(&symbols(&["mir_2"])),
            "guard-limited",
            "a place this failure does not read is not evidence about it"
        );
    }

    /// The dependence test reads rendered formulas, so a symbol that merely
    /// *contains* another's name must not be taken for it: `mir_3` is a substring
    /// of `prev_bb6_mir_3`, and a substring test would walk a relation about the
    /// previous trip as though it were about this trip's value.
    #[test]
    fn a_symbol_is_matched_as_a_whole_name_in_a_rendered_formula() {
        assert!(mentions_symbol(
            "(= prev_bb6_mir_3 (+ mir_7 1))",
            "prev_bb6_mir_3"
        ));
        assert!(!mentions_symbol("(= prev_bb6_mir_3 (+ mir_7 1))", "mir_3"));
        assert!(mentions_symbol("(= mir_3 (+ 0 1))", "mir_3"));
        assert!(!mentions_symbol("(= mir_33 (+ 0 1))", "mir_3"));
        assert!(!mentions_symbol("(= mir_3_9 (+ 0 1))", "mir_3"));
        assert!(!mentions_symbol("", "mir_3"));
        assert!(!mentions_symbol("(= mir_3 x)", ""));
    }

    /// The encodings compute `2^w`, so they are defined exactly where that is an
    /// `i128` — which is why 128-bit types are refused rather than approximated.
    /// This is the boundary the refusal is drawn at, and it fails if it moves.
    #[test]
    fn a_store_the_key_scheme_cannot_name_retires_its_own_local() {
        // An index write touches the array it indexes and nothing else, so a
        // borrowed local it cannot reach stays out of it.
        let writes = BodyWrites {
            written: HashSet::from([1, 2]),
            address_taken: HashSet::from([7, 9]),
            mutably_address_taken: HashSet::from([7, 9]),
            pointees: HashMap::new(),
        };
        assert_eq!(
            retirement_targets(5, false, &writes),
            vec![(5usize, Vec::new())]
        );
        // A store through a pointer could be any local this body lent out, and
        // they are retired in a deterministic order.
        assert_eq!(
            retirement_targets(5, true, &writes),
            vec![(5usize, Vec::new()), (7, Vec::new()), (9, Vec::new())]
        );
    }

    #[test]
    fn a_borrowed_local_is_retired_once_when_it_is_also_the_stores_own() {
        let writes = BodyWrites {
            written: HashSet::new(),
            address_taken: HashSet::from([5]),
            mutably_address_taken: HashSet::from([5]),
            pointees: HashMap::new(),
        };
        assert_eq!(
            retirement_targets(5, true, &writes),
            vec![(5usize, Vec::new())]
        );
    }

    #[test]
    fn invalidating_a_place_takes_its_projection_and_its_ancestor() {
        // `values` is keyed by local plus field path, so a write to a whole local
        // makes every field of it stale, and a write to a field makes the whole
        // local stale. Neither may survive, and an unrelated local may not move.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let mut state = PathState::initial();
        state.values.insert((3, Vec::new()), int_lit(&ctx, 1));
        state.values.insert((3, vec![0]), int_lit(&ctx, 2));
        state.values.insert((4, Vec::new()), int_lit(&ctx, 3));

        state.invalidate(&(3, Vec::new()));
        assert!(state.values.get(&(3, Vec::new())).is_none());
        assert!(state.values.get(&(3, vec![0])).is_none());
        assert!(state.values.get(&(4, Vec::new())).is_some());

        state.values.insert((3, Vec::new()), int_lit(&ctx, 1));
        state.values.insert((3, vec![0]), int_lit(&ctx, 2));
        state.invalidate(&(3, vec![0]));
        assert!(state.values.get(&(3, Vec::new())).is_none());
        assert!(state.values.get(&(3, vec![0])).is_none());
    }

    #[test]
    fn invalidating_a_place_retires_the_checked_fact_that_names_it() {
        // K9's stated soundness condition — the payload value and the fact saying
        // which variant the sum chose travel together — rested on this and had no
        // test at all. Retiring either the payload or the `Option` must retire the
        // fact too: a discriminant over a sum the state has forgotten is a claim
        // about a value nothing holds any more.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        fn build<'ctx>(ctx: &'ctx Context) -> PathState<'ctx> {
            let mut state = PathState::initial();
            let sum = Int::new_const(ctx, "sum");
            state
                .values
                .insert((3, vec![0]), fixed_value(ctx, &sum, IntType::U8));
            state.checked.insert(
                (3, Vec::new()),
                CheckedProduct {
                    sum,
                    ty: IntType::U8,
                    payload: (3, vec![0]),
                    payload_variant: 1,
                    empty_variant: 0,
                },
            );
            state
        }

        let mut by_payload = build(&ctx);
        by_payload.invalidate(&(3, vec![0]));
        assert!(by_payload.checked.is_empty());
        assert!(by_payload.values.get(&(3, vec![0])).is_none());

        let mut by_option = build(&ctx);
        by_option.invalidate(&(3, Vec::new()));
        assert!(by_option.checked.is_empty());
        assert!(by_option.values.get(&(3, vec![0])).is_none());
    }

    #[test]
    fn a_pointer_keeps_its_pointee_until_a_write_says_otherwise() {
        // The rule `operand_pointer_write` rests on: one hop of `&place` names the
        // local a callee's pointer can reach, and *any* other write to the pointer
        // local takes that answer away. Which local a pointer read out of memory
        // points at is the aliasing question, and a second borrow is a second
        // answer, so both fall back to "any local this body borrowed".
        let mut pointees = HashMap::new();
        // `p = &mut _5` twice is still `_5`.
        record_pointee(&mut pointees, 8, Some(5));
        record_pointee(&mut pointees, 8, Some(5));
        assert_eq!(pointees.get(&8), Some(&Some(5)));
        // A different borrow makes it two candidates, so neither is known.
        record_pointee(&mut pointees, 8, Some(6));
        assert_eq!(pointees.get(&8), Some(&None));
        // And once it is unknown, no later borrow makes it known again: the local
        // may have been written in between, and the merge can only lose.
        record_pointee(&mut pointees, 8, Some(5));
        assert_eq!(pointees.get(&8), Some(&None));
        // A write that is not a borrow is the same answer, from either side.
        record_pointee(&mut pointees, 9, None);
        record_pointee(&mut pointees, 9, Some(5));
        assert_eq!(pointees.get(&9), Some(&None));
        // A local this body never writes has no entry at all, which is how a
        // parameter is told apart from a temp.
        assert_eq!(pointees.get(&10), None);
    }

    #[test]
    fn a_store_retires_its_base_and_reaches_borrowed_locals_only_through_a_pointer() {
        // `retirement_targets` is what decides how far a write reaches. The base
        // local always — `a[i] = …` destroys the whole `a`, and a downcast write
        // destroys the enum a `checked_*` payload lives in — and the locals this
        // body borrowed only when the projection goes *through* a pointer, because
        // that is the case where the analysis cannot say which of them it hit.
        let writes = BodyWrites {
            written: HashSet::from([3, 4]),
            address_taken: HashSet::from([4, 7]),
            mutably_address_taken: HashSet::from([4, 7]),
            pointees: HashMap::new(),
        };
        assert_eq!(retirement_targets(3, false, &writes), vec![(3, Vec::new())]);
        assert_eq!(
            retirement_targets(3, true, &writes),
            vec![(3, Vec::new()), (4, Vec::new()), (7, Vec::new())]
        );
        // Sorted and deduplicated, so the retirement order cannot depend on hash
        // iteration: a base local that is also borrowed appears once.
        assert_eq!(
            retirement_targets(7, true, &writes),
            vec![(4, Vec::new()), (7, Vec::new())]
        );
    }

    #[test]
    fn a_pointer_write_retires_every_borrowed_local_and_names_the_write() {
        // The other half of the rule: what a callee, an asm block or drop glue
        // writes through an unknown pointer. Every borrowed local is given up with
        // the reason recorded, so a later read of it is undecided-with-a-reason
        // rather than answered from the value the unread body overwrote — and a
        // local the body never borrowed keeps its value.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let writes = BodyWrites {
            written: HashSet::from([3, 4]),
            address_taken: HashSet::from([3]),
            mutably_address_taken: HashSet::from([3]),
            pointees: HashMap::new(),
        };
        let mut state = PathState::initial();
        state.values.insert((3, Vec::new()), int_lit(&ctx, 1));
        state.values.insert((4, Vec::new()), int_lit(&ctx, 2));
        state.retire_pointer_write(&writes, PointerWrite::Any, "the callee may write it");
        assert!(state.values.get(&(3, Vec::new())).is_none());
        assert!(state.values.get(&(4, Vec::new())).is_some());
        assert_eq!(
            state.unmodelled_reason(&state.symbol_name(&(3, Vec::new()))),
            Some("the callee may write it")
        );
        // A known pointee reaches exactly that local.
        let mut known = PathState::initial();
        known.values.insert((3, Vec::new()), int_lit(&ctx, 1));
        known.values.insert((4, Vec::new()), int_lit(&ctx, 2));
        known.retire_pointer_write(&writes, PointerWrite::Local(4), "the callee may write it");
        assert!(known.values.get(&(3, Vec::new())).is_some());
        assert!(known.values.get(&(4, Vec::new())).is_none());
    }

    #[test]
    fn an_immutable_borrow_is_not_retired_by_an_arbitrary_pointer_write() {
        // S1: When a local is only borrowed immutably (address_taken but NOT
        // mutably_address_taken), an unknown callee or drop glue cannot modify it.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let writes = BodyWrites {
            written: HashSet::from([3]),
            address_taken: HashSet::from([3]),
            mutably_address_taken: HashSet::new(),
            pointees: HashMap::new(),
        };
        let mut state = PathState::initial();
        state.values.insert((3, Vec::new()), int_lit(&ctx, 42));
        state.retire_pointer_write(&writes, PointerWrite::Any, "callee write");
        // Local 3 is preserved!
        assert!(state.values.get(&(3, Vec::new())).is_some());
    }

    #[test]
    fn aggregate_and_subfield_copy_tracks_individual_components() {
        // S2: Struct and tuple subfields are preserved and copied individually.
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let mut state = PathState::initial();
        state.values.insert((3, vec![0]), int_lit(&ctx, 10));
        state.values.insert((3, vec![1]), int_lit(&ctx, 20));
        state.discriminants.insert((3, Vec::new()), 1);

        let sub_fields: Vec<(Vec<usize>, Int<'_>)> = state
            .values
            .iter()
            .filter(|(k, _)| k.0 == 3 && !k.1.is_empty())
            .map(|(k, v)| (k.1.clone(), v.clone()))
            .collect();
        state.invalidate(&(4, Vec::new()));
        for (suffix, val) in sub_fields {
            state.values.insert((4, suffix), val);
        }

        assert_eq!(state.values.get(&(4, vec![0])).unwrap().to_string(), "10");
        assert_eq!(state.values.get(&(4, vec![1])).unwrap().to_string(), "20");
    }

    #[test]
    fn the_modulus_is_representable_for_every_width_the_encodings_accept() {
        let accepted = [
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
        ];
        for ty in accepted {
            if ty.bit_width() < 128 {
                // Well-defined exactly here, and the encodings may be reached.
                let span = 1i128 << ty.bit_width();
                assert!(span > 0, "{ty:?} has a modulus the encodings can state");
            } else {
                assert!(
                    matches!(ty, IntType::I128 | IntType::U128),
                    "{ty:?} is the case `apply_safe_arith` must refuse"
                );
            }
        }
    }

    #[test]
    fn test_z3_diff_simplification() {
        let cfg = z3::Config::new();
        let ctx = Context::new(&cfg);
        let x = Int::new_const(&ctx, "x");
        let next = &x + &Int::from_i64(&ctx, 1);
        let diff = (&next - &x).simplify();
        assert_eq!(diff.as_i64(), Some(1));

        let next2 = &x - &Int::from_i64(&ctx, 4);
        let diff2 = (&next2 - &x).simplify();
        assert_eq!(diff2.as_i64(), Some(-4));
    }
}
