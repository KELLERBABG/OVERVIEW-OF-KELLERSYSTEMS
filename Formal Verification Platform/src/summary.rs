//! Interprocedural summaries for local helper functions (S7).
//!
//! Every function is analysed on its own body, so its parameters arrive
//! unconstrained: `fn inc(x: u8) -> u8 { x + 1 }` reports an `unguarded` overflow
//! even when every caller in the crate passes a value the addition cannot
//! overflow. That is the boundary A8 pinned in `tests/precond_probe` — the callee
//! judged without its call sites — and this module is the half of it that can be
//! closed without guessing.
//!
//! The summary is a **precondition derived from the helper's own body**, and every
//! restriction below exists to keep assuming it *sound* rather than merely quiet:
//!
//! * **leaf and cycle-free** — a helper that calls no local function cannot be
//!   part of a recursion cycle, so the set of call sites the scan can see is the
//!   whole set. (A stricter rule than "cycle-free" on purpose: it is decidable
//!   from one body instead of a call graph that would itself have to be trusted.)
//! * **small** — a body over [`SMALL_BLOCKS`] is past the point where a body-level
//!   precondition reads as a fact about the helper; the caller should write one.
//! * **private** — an exported function has callers this analysis cannot see, so
//!   "every call site satisfies it" would be a claim about the crate's users. A
//!   private one is reachable only from this crate, which is what the scan reads.
//! * **address never taken** — a private function whose `fn` item is passed as a
//!   value or cast to a pointer can be called by code the call-terminator scan
//!   never sees. Any mention of the item outside a direct call disqualifies it.
//! * **every call site passes a constant that satisfies the bound** — the
//!   precondition is *checked* before it is assumed. A site whose argument this
//!   analysis does not know abandons the summary instead of assuming the bound
//!   holds there: the body then keeps its `unguarded` finding, which is the
//!   honest report and the behaviour that existed before this module.
//!
//! When all of that holds the helper's body is analysed under its derived
//! precondition — the same entry guard `#[requires]` uses — and each call site is
//! recorded as a checked obligation (`kind: "helper-precondition"`), so the
//! assumption and the proof of it are in the same run and the same report.
//!
//! What this is *not*: a contract the user wrote, or a proof about the helper in
//! isolation. The bound is exactly the one its callers were shown to satisfy, and
//! a caller outside the crate or through a pointer moves the helper back to the
//! unguarded reading rather than silently keeping the assumption.

use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_middle::mir::visit::Visitor;
use rustc_middle::mir::{
    BinOp, Body, Local, Location, Operand, Place, Rvalue, StatementKind, Terminator,
    TerminatorKind, VarDebugInfoContents,
};
use rustc_middle::ty::{Ty, TyCtxt, TyKind, Visibility};
use std::collections::{HashMap, HashSet};
use z3::ast::{Bool, Int};
use z3::Context;

use crate::constant_prop::ConstMap;

/// The largest body that still reads as a helper rather than as a unit of its own.
pub const SMALL_BLOCKS: usize = 32;

/// One bound on one parameter, and the type range it was derived against.
///
/// `low`/`high` are the parts of the bound that are narrower than the parameter's
/// own type: a `u8` addition bounds above only, a subtraction below only. Both
/// `None` never reaches the caller — such an operation is not a precondition at
/// all and [`derive_bounds`] drops it.
#[derive(Clone, Debug)]
pub struct ParamBound {
    /// The callee's parameter local (`_1` is 1). The call site's argument is at
    /// `local - 1`, because argument 0 is that same parameter.
    pub local: usize,
    pub name: String,
    pub low: Option<i128>,
    pub high: Option<i128>,
}

impl ParamBound {
    /// Whether a value the caller passes satisfies this bound.
    pub fn holds(&self, value: i128) -> bool {
        self.low.is_none_or(|low| value >= low) && self.high.is_none_or(|high| value <= high)
    }

    /// The bound as a reader would write it, for the console and the report.
    pub fn describe(&self) -> String {
        match (self.low, self.high) {
            (Some(low), Some(high)) => format!("{low} <= {} <= {high}", self.name),
            (Some(low), None) => format!("{} >= {low}", self.name),
            (None, Some(high)) => format!("{} <= {high}", self.name),
            (None, None) => format!("{} unrestricted", self.name),
        }
    }
}

/// A helper whose precondition was derived and shown to hold at every call site.
#[derive(Clone, Debug)]
pub struct HelperSummary {
    pub function: String,
    pub bounds: Vec<ParamBound>,
    /// How many call sites were checked. Always at least one: a helper nothing
    /// calls has no call sites to check, so nothing guarantees the bound.
    pub sites: usize,
}

/// Every helper this crate's own call sites decide the precondition of.
pub fn build(tcx: TyCtxt<'_>) -> HashMap<DefId, HelperSummary> {
    let def_ids: Vec<DefId> = tcx
        .mir_keys(())
        .iter()
        .map(|local| local.to_def_id())
        .filter(|def_id| {
            matches!(
                tcx.def_kind(*def_id),
                DefKind::Fn | DefKind::AssocFn | DefKind::Closure
            )
        })
        .filter(|def_id| tcx.is_mir_available(*def_id))
        .filter(|def_id| !crate::reachability::is_panic_internal(&tcx.def_path_str(*def_id)))
        .collect();
    if def_ids.is_empty() {
        return HashMap::new();
    }

    let mut candidates: HashMap<DefId, Vec<ParamBound>> = HashMap::new();
    for &def_id in &def_ids {
        if let Some(bounds) = derive_bounds(tcx, def_id) {
            candidates.insert(def_id, bounds);
        }
    }
    if candidates.is_empty() {
        return HashMap::new();
    }

    // Every body is scanned for direct calls to a candidate and for any other
    // mention of one, so "every call site" is answered from the MIR rather than
    // assumed from the shape of the helper.
    // The scan reads *pre-inlining* MIR on purpose: `optimized_mir` has already
    // inlined a small helper into its callers, so the direct calls this summary is
    // about no longer exist there and the helper would look uncalled. The bound
    // itself is derived from the optimized body, which is the body the analysis
    // walks, so the guard and the arithmetic are stated over the same MIR.
    let mut scan = Scan {
        tcx,
        candidates: &candidates,
        caller: def_ids[0],
        sites: HashMap::new(),
        taken: HashSet::new(),
    };
    for &def_id in &def_ids {
        scan.caller = def_id;
        scan.visit_body(tcx.optimized_mir(def_id));
    }

    let mut summaries = HashMap::new();
    for (&def_id, bounds) in candidates.iter() {
        // The item is used somewhere other than as the target of a direct call,
        // so there is a caller this scan does not describe.
        if scan.taken.contains(&def_id) {
            continue;
        }
        let Some(sites) = scan.sites.get(&def_id) else {
            continue;
        };
        if sites.is_empty() {
            continue;
        }
        // Checked, not assumed: a site that does not pass a constant or bounded value
        // satisfying the bound leaves the helper reported on its own body.
        let every_site_checked = sites.iter().all(|site| {
            bounds.iter().all(|bound| {
                let index = match bound.local.checked_sub(1) {
                    Some(i) => i,
                    None => return false,
                };
                if let Some(Some(value)) = site.values.get(index) {
                    if bound.holds(*value) {
                        return true;
                    }
                }
                if let Some(Some((low, high))) = site.ranges.get(index) {
                    let low_ok = bound.low.is_none_or(|l| *low >= l);
                    let high_ok = bound.high.is_none_or(|h| *high <= h);
                    if low_ok && high_ok {
                        return true;
                    }
                }
                false
            })
        });
        if !every_site_checked {
            continue;
        }
        summaries.insert(
            def_id,
            HelperSummary {
                function: tcx.def_path_str(def_id),
                bounds: bounds.clone(),
                sites: sites.len(),
            },
        );
    }
    summaries
}

/// The derived precondition as an entry guard over the helper's own parameters.
///
/// The terms are built from the *same* initial state the body's walk starts from,
/// so the symbol named here is the one the body's arithmetic is checked against —
/// a guard over a differently named symbol would prove nothing about the body and
/// would look exactly like a proof that did.
pub fn entry_guard<'ctx, 'tcx>(
    summary: &HelperSummary,
    ctx: &'ctx Context,
    mir: &Body<'tcx>,
    const_map: &ConstMap,
    tcx: TyCtxt<'tcx>,
) -> Option<Bool<'ctx>> {
    let mut state = crate::path_state::PathState::initial();
    let mut terms: Vec<Bool<'ctx>> = Vec::new();
    for bound in &summary.bounds {
        let operand = Operand::Copy(Place::from(Local::from_usize(bound.local)));
        let value = state.int_operand(ctx, mir, const_map, tcx, &operand)?;
        if let Some(low) = bound.low {
            terms.push(value.ge(&Int::from_str(ctx, &low.to_string())?));
        }
        if let Some(high) = bound.high {
            terms.push(value.le(&Int::from_str(ctx, &high.to_string())?));
        }
    }
    if terms.is_empty() {
        return None;
    }
    let refs: Vec<&Bool<'ctx>> = terms.iter().collect();
    Some(crate::path_state::all_of(ctx, &refs))
}

/// A constant argument, exactly as written, including the unsigned half above
/// `i128::MAX` (which is not a value this analysis checks a bound against).
pub fn literal_i128<'tcx>(operand: &Operand<'tcx>, tcx: TyCtxt<'tcx>) -> Option<i128> {
    crate::constant_prop::integer_literal(operand, tcx)?
        .parse::<i128>()
        .ok()
}

// -----------------------------------------------------------------------------
// Deriving the precondition from the body
// -----------------------------------------------------------------------------

/// Why a helper cannot carry a summary derived this way, when the reason is that
/// it is not one: no integer parameter arithmetic, or a shape that makes "every
/// call site" untrue.
fn derive_bounds(tcx: TyCtxt<'_>, def_id: DefId) -> Option<Vec<ParamBound>> {
    let mir = tcx.optimized_mir(def_id);
    if mir.arg_count == 0 || mir.basic_blocks.len() > SMALL_BLOCKS {
        return None;
    }
    if !matches!(tcx.visibility(def_id), Visibility::Restricted(_)) {
        return None;
    }
    if has_local_callee(&mir) {
        return None;
    }

    let mut bounds: Vec<ParamBound> = Vec::new();
    for block in mir.basic_blocks.iter() {
        for statement in &block.statements {
            let StatementKind::Assign(box (_, rvalue)) = &statement.kind else {
                continue;
            };
            let Rvalue::BinaryOp(op, box (lhs, rhs)) = rvalue else {
                continue;
            };
            let Some((local, low, high)) = derive_op_bound(op, lhs, rhs, &mir, tcx) else {
                continue;
            };
            if low.is_none() && high.is_none() {
                continue;
            }
            let bound = ParamBound {
                local,
                name: parameter_name(&mir, local),
                low,
                high,
            };
            if !bounds.iter().any(|seen| {
                seen.local == bound.local && seen.low == bound.low && seen.high == bound.high
            }) {
                bounds.push(bound);
            }
        }
    }
    if bounds.is_empty() {
        return None;
    }
    Some(bounds)
}

/// Derive parameter bounds for an operation: Add, Sub, Mul, Div, Rem (S7).
fn derive_op_bound<'tcx>(
    op: &BinOp,
    lhs: &Operand<'tcx>,
    rhs: &Operand<'tcx>,
    mir: &Body<'tcx>,
    tcx: TyCtxt<'tcx>,
) -> Option<(usize, Option<i128>, Option<i128>)> {
    match op {
        // A debug build spells the checked arithmetic `AddWithOverflow` and tests
        // the flag it returns; the values the operation needs to stay in the type
        // are the same either way, which is what this bound is.
        BinOp::Add | BinOp::AddWithOverflow => {
            let (local, constant) = if let (Some(l), Some(c)) =
                (parameter_local(lhs, mir), literal_i128(rhs, tcx))
            {
                (l, c)
            } else if let (Some(c), Some(l)) =
                (literal_i128(lhs, tcx), parameter_local(rhs, mir))
            {
                (l, c)
            } else {
                return None;
            };
            let ty = mir.local_decls[Local::from_usize(local)].ty;
            let range = int_range(ty, tcx)?;
            let low = range.min.checked_sub(constant)?;
            let high = range.max.checked_sub(constant)?;
            let low = (low > range.min).then_some(low);
            let high = (high < range.max).then_some(high);
            Some((local, low, high))
        }
        BinOp::Sub | BinOp::SubWithOverflow => {
            let local = parameter_local(lhs, mir)?;
            let constant = literal_i128(rhs, tcx)?;
            let ty = mir.local_decls[Local::from_usize(local)].ty;
            let range = int_range(ty, tcx)?;
            let low = range.min.checked_add(constant)?;
            let high = range.max.checked_add(constant)?;
            let low = (low > range.min).then_some(low);
            let high = (high < range.max).then_some(high);
            Some((local, low, high))
        }
        BinOp::Mul | BinOp::MulWithOverflow => {
            let (local, constant) = if let (Some(l), Some(c)) =
                (parameter_local(lhs, mir), literal_i128(rhs, tcx))
            {
                (l, c)
            } else if let (Some(c), Some(l)) =
                (literal_i128(lhs, tcx), parameter_local(rhs, mir))
            {
                (l, c)
            } else {
                return None;
            };
            if constant == 0 {
                return None;
            }
            let ty = mir.local_decls[Local::from_usize(local)].ty;
            let range = int_range(ty, tcx)?;
            if constant > 0 {
                let low = if ty.is_signed() {
                    let l = range.min / constant;
                    (l > range.min).then_some(l)
                } else {
                    None
                };
                let high = range.max / constant;
                let high = (high < range.max).then_some(high);
                Some((local, low, high))
            } else {
                let low = range.max / constant;
                let high = range.min / constant;
                let low = (low > range.min).then_some(low);
                let high = (high < range.max).then_some(high);
                Some((local, low, high))
            }
        }
        BinOp::Div | BinOp::Rem => {
            // Divisor parameter cannot be zero: for unsigned types, local >= 1
            if let Some(local) = parameter_local(rhs, mir) {
                let ty = mir.local_decls[Local::from_usize(local)].ty;
                if !ty.is_signed() {
                    return Some((local, Some(1), None));
                }
            }
            // lhs / -1 panics when lhs is MIN (for signed types)
            if let (Some(local), Some(-1)) = (parameter_local(lhs, mir), literal_i128(rhs, tcx)) {
                let ty = mir.local_decls[Local::from_usize(local)].ty;
                if ty.is_signed() {
                    let range = int_range(ty, tcx)?;
                    let low = range.min.checked_add(1)?;
                    return Some((local, Some(low), None));
                }
            }
            None
        }
        _ => None,
    }
}

/// A bare integer argument of the body, if the operand is one.
fn parameter_local<'tcx>(operand: &Operand<'tcx>, mir: &Body<'tcx>) -> Option<usize> {
    let (Operand::Copy(place) | Operand::Move(place)) = operand else {
        return None;
    };
    if !place.projection.is_empty() {
        return None;
    }
    let local = place.local.as_usize();
    if local == 0 || local > mir.arg_count {
        return None;
    }
    let ty = mir.local_decls[place.local].ty;
    matches!(ty.kind(), TyKind::Int(_) | TyKind::Uint(_)).then_some(local)
}

struct IntRange {
    min: i128,
    max: i128,
}

/// The range of an integer type this analysis can state a bound in.
fn int_range<'tcx>(ty: Ty<'tcx>, tcx: TyCtxt<'tcx>) -> Option<IntRange> {
    if !matches!(ty.kind(), TyKind::Int(_) | TyKind::Uint(_)) {
        return None;
    }
    let bits = ty.primitive_size(tcx).bits();
    // 128-bit values do not fit the `i128` this bounds arithmetic in; a bound at
    // the top of the range is not worth the ambiguity of one that wrapped there.
    if bits == 0 || bits >= 128 {
        return None;
    }
    let bits = bits as u32;
    Some(if ty.is_signed() {
        let max = (1i128 << (bits - 1)) - 1;
        IntRange { min: -max - 1, max }
    } else {
        IntRange {
            min: 0,
            max: (1i128 << bits) - 1,
        }
    })
}

fn parameter_name(mir: &Body<'_>, local: usize) -> String {
    for var in &mir.var_debug_info {
        if let VarDebugInfoContents::Place(place) = &var.value {
            if place.projection.is_empty() && place.local.as_usize() == local {
                return var.name.as_str().to_string();
            }
        }
    }
    format!("_{local}")
}

/// Whether the body calls a function from this crate, which is what makes it a
/// leaf (and therefore cycle-free).
fn has_local_callee(mir: &Body<'_>) -> bool {
    mir.basic_blocks.iter().any(|block| {
        let Some(terminator) = &block.terminator else {
            return false;
        };
        match &terminator.kind {
            TerminatorKind::Call { func, .. } | TerminatorKind::TailCall { func, .. } => func
                .const_fn_def()
                .map(|(callee, _)| callee.is_local())
                .unwrap_or(false),
            _ => false,
        }
    })
}

// -----------------------------------------------------------------------------
// The call-site scan
// -----------------------------------------------------------------------------

/// One argument list seen at one direct call.
struct SiteRecord {
    /// Each argument's value when it is written as a constant, `None` otherwise.
    values: Vec<Option<i128>>,
    /// Known bounds (min, max) for non-constant arguments (e.g. from caller parameter bounds or types).
    ranges: Vec<Option<(i128, i128)>>,
}

/// Walks every body for the two things "every call site" needs: the direct calls
/// of a candidate, and any *other* mention of one (a `fn` item used as a value,
/// which is a call route this scan cannot see).
struct Scan<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    caller: DefId,
    candidates: &'a HashMap<DefId, Vec<ParamBound>>,
    sites: HashMap<DefId, Vec<SiteRecord>>,
    taken: HashSet<DefId>,
}

impl<'tcx> Visitor<'tcx> for Scan<'_, 'tcx> {
    fn visit_terminator(&mut self, terminator: &Terminator<'tcx>, location: Location) {
        match &terminator.kind {
            TerminatorKind::Call { func, args, .. }
            | TerminatorKind::TailCall { func, args, .. } => {
                if let Some((callee, _)) = func.const_fn_def() {
                    if self.candidates.contains_key(&callee) {
                        let caller_mir = self.tcx.optimized_mir(self.caller);
                        let mut values = Vec::new();
                        let mut ranges = Vec::new();
                        for arg in args {
                            let val = literal_i128(&arg.node, self.tcx);
                            let range = if val.is_some() {
                                None
                            } else if let Some(local) = parameter_local(&arg.node, caller_mir) {
                                let ty = caller_mir.local_decls[Local::from_usize(local)].ty;
                                let ty_range = int_range(ty, self.tcx);
                                if let Some(caller_bounds) = self.candidates.get(&self.caller) {
                                    if let Some(b) = caller_bounds.iter().find(|b| b.local == local) {
                                        let min = b.low.or(ty_range.as_ref().map(|r| r.min)).unwrap_or(i128::MIN);
                                        let max = b.high.or(ty_range.as_ref().map(|r| r.max)).unwrap_or(i128::MAX);
                                        Some((min, max))
                                    } else {
                                        ty_range.map(|r| (r.min, r.max))
                                    }
                                } else {
                                    ty_range.map(|r| (r.min, r.max))
                                }
                            } else {
                                None
                            };
                            values.push(val);
                            ranges.push(range);
                        }
                        self.sites.entry(callee).or_default().push(SiteRecord {
                            values,
                            ranges,
                        });
                    }
                }
                // The callee operand is accounted for above; the arguments are
                // values, and a `fn` item among them is an address this scan
                // cannot follow.
                for arg in args {
                    self.visit_operand(&arg.node, location);
                }
            }
            _ => self.super_terminator(terminator, location),
        }
    }

    fn visit_operand(&mut self, operand: &Operand<'tcx>, location: Location) {
        // A function item is always a constant operand, so the body is not needed
        // to see one referenced: any mention outside a direct call is a call route
        // this scan cannot follow.
        if let Operand::Constant(constant) = operand {
            if let TyKind::FnDef(def_id, _) = constant.const_.ty().kind() {
                if self.candidates.contains_key(def_id) {
                    self.taken.insert(*def_id);
                }
            }
        }
        self.super_operand(operand, location);
    }
}

#[cfg(test)]
mod tests {
    use super::ParamBound;

    #[test]
    fn a_bound_holds_across_closed_and_half_open_ranges() {
        let upper = ParamBound {
            local: 1,
            name: "x".to_string(),
            low: None,
            high: Some(254),
        };
        assert!(upper.holds(0));
        assert!(upper.holds(254));
        assert!(!upper.holds(255));

        let both = ParamBound {
            low: Some(1),
            high: Some(9),
            ..upper.clone()
        };
        assert!(!both.holds(0));
        assert!(both.holds(5));
        assert!(!both.holds(10));
    }

    #[test]
    fn a_bound_is_described_the_way_a_reader_writes_it() {
        let bound = ParamBound {
            local: 2,
            name: "n".to_string(),
            low: None,
            high: Some(254),
        };
        assert_eq!(bound.describe(), "n <= 254");
        let both = ParamBound {
            low: Some(1),
            ..bound
        };
        assert_eq!(both.describe(), "1 <= n <= 254");
    }

    #[test]
    fn mul_and_div_bounds_hold_for_ranges() {
        let mul_bound = ParamBound {
            local: 1,
            name: "x".to_string(),
            low: None,
            high: Some(127),
        };
        assert!(mul_bound.holds(0));
        assert!(mul_bound.holds(127));
        assert!(!mul_bound.holds(128));

        let div_bound = ParamBound {
            local: 1,
            name: "y".to_string(),
            low: Some(1),
            high: None,
        };
        assert!(!div_bound.holds(0));
        assert!(div_bound.holds(1));
        assert!(div_bound.holds(255));
    }
}
