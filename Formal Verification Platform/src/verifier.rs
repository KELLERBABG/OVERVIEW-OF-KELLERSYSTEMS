use rustc_driver::Compilation;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::{DefId, LOCAL_CRATE};
use rustc_middle::mir::{
    AssertKind, BasicBlock, BinOp, Body, Operand, Rvalue, StatementKind, TerminatorKind,
};
use rustc_middle::ty::{TyCtxt, TyKind};
use std::collections::{BTreeSet, HashSet};
use std::sync::{Arc, Mutex};
use z3::ast::{Ast, Int};
use z3::{Config, Context, SatResult, Solver};

use crate::constant_prop::{eval_operand, operand_int_type, propagate_constants, ConstMap};
use crate::constraint::{
    build_bounds_constraints, build_divzero_constraints, build_neg_overflow_constraints,
    build_overflow_constraints, extract_witness, is_user_assertion, overflow_detected, Encoding,
    SymCtx, WitnessVar,
};
use crate::path_state::{operand_label, PathAnalysis, PathState};
use crate::reachability::{compute_reachable_blocks, is_panic_internal};
use crate::report::{is_non_assertion_kind, print_summary, quiet, write_smt_obligation};
use crate::types::IntType;
use crate::types::{Coverage, FunctionEvidence, SharedReport, VerificationEvent, Witness};

pub struct KellerVerifier {
    pub report: Arc<Mutex<SharedReport>>,
}

impl rustc_driver::Callbacks for KellerVerifier {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        tcx: TyCtxt<'tcx>,
    ) -> Compilation {
        analyze_mir(tcx, Arc::clone(&self.report));
        Compilation::Continue
    }
}

/// Analyze all MIR bodies in the crate, checking assertions with Z3.
/// The solver configuration for this run.
///
/// The per-obligation budget (`--solver-timeout`) is not only about speed. It is
/// also the difference between a verdict and a hang: with no budget, a
/// pathological obligation never returns, and a pipeline that never returns
/// reports nothing at all. With a budget, the solver answers `unknown`, which
/// becomes the WARN verdict — the one that must never be mistaken for a pass, and
/// the reason `--deny-warn` exists.
fn set_z3_global_param(key: &str, val: &str) {
    if let (Ok(k), Ok(v)) = (std::ffi::CString::new(key), std::ffi::CString::new(val)) {
        unsafe {
            z3_sys::Z3_global_param_set(k.as_ptr(), v.as_ptr());
        }
    }
}

fn solver_timeout_budget() -> Option<u64> {
    match std::env::var("KELLER_SOLVER_TIMEOUT_MS") {
        Ok(raw) => match raw.parse::<u64>() {
            Ok(0) => None,
            Ok(ms) => Some(ms),
            Err(_) => Some(crate::report::DEFAULT_SOLVER_TIMEOUT_MS),
        },
        Err(_) => Some(crate::report::DEFAULT_SOLVER_TIMEOUT_MS),
    }
}

fn solver_config() -> Config {
    let mut cfg = Config::new();
    set_z3_global_param("model.compact", "false");
    // Ensure deterministic solver heuristics across runs and platforms (K10 / A14)
    set_z3_global_param("sat.random_seed", "0");
    set_z3_global_param("smt.random_seed", "0");
    // Absent, unparsable, or a positive number: a budget is always in force, so no
    // default path can wedge a CI job. `0` is the one value that means "no limit",
    // and it is what `--solver-timeout 0` arrives as — a caller saying so on
    // purpose rather than a default nobody chose.
    let budget = solver_timeout_budget();
    if let Some(ms) = budget {
        cfg.set_timeout_msec(ms);
    }
    // Hard ceiling on solver memory per instance (default 1024 MB).
    // Prevents Z3 from exhausting host RAM on complex obligations and forcing a system freeze/reboot.
    let mem_limit_mb = match std::env::var("KELLER_SOLVER_MEMORY_MB") {
        Ok(raw) => match raw.parse::<u64>() {
            Ok(0) => None,
            Ok(mb) => Some(mb),
            Err(_) => Some(1024),
        },
        Err(_) => Some(1024),
    };
    if let Some(mb) = mem_limit_mb {
        set_z3_global_param("memory_max_size", &mb.to_string());
    }
    cfg
}

/// How many finding lines this worker may print.
///
/// `--max-findings` reaches the worker through the environment because the
/// per-verdict lines are printed here, not at the merge: capping them after the
/// run would let a large crate fill the log before the summary ever ran. The cap
/// is a listing cap only — the gate and `report.json` see every finding — and the
/// note printed once, when it is reached, says so.
fn finding_line_cap() -> Option<usize> {
    std::env::var("KELLER_MAX_FINDINGS")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
}

/// Print one finding line, or the single note that says the listing is capped.
fn print_finding(cap: Option<usize>, printed: &mut usize, line: String) {
    match cap {
        Some(limit) if *printed >= limit => {
            if *printed == limit {
                eprintln!(
                    "[Keller] Finding lines capped at {limit} by --max-findings. The gate still judged every finding and report.json is complete; the rest is in the summary."
                );
            }
        }
        _ => eprintln!("{line}"),
    }
    *printed += 1;
}

/// Where a function was defined, as `(file, line)`, when the compiler can say.
///
/// A finding that names a file and a line is one a reader can act on, and it is
/// the only form a platform can annotate on a pull request, so this is worth two
/// rustc calls per function. A synthetic span (macro-generated, or from a file the
/// compiler cannot name) yields an empty file rather than a placeholder: an
/// annotation pointing at a path that does not exist is worse than none.
fn source_location(tcx: TyCtxt<'_>, def_id: DefId) -> (String, Option<u32>) {
    let span = tcx.def_span(def_id);
    if span.from_expansion() {
        return (String::new(), None);
    }
    let location = tcx.sess.source_map().lookup_char_pos(span.lo());
    let name = location
        .file
        .name
        .prefer_remapped_unconditionally()
        .to_string();
    // Cargo runs the wrapper with the verified project as the working directory,
    // so trimming that prefix keeps report.json and the SARIF log portable
    // between the machine that produced them and the one reading them.
    let name = std::env::current_dir()
        .ok()
        .and_then(|cwd| {
            let prefix = format!("{}{}", cwd.display(), std::path::MAIN_SEPARATOR);
            name.strip_prefix(&prefix).map(str::to_string)
        })
        .unwrap_or(name);
    // Forward slashes whatever the host: `report.json` is meant to be readable and
    // comparable across machines, and a SARIF location is a URI.
    let name = name.replace('\\', "/");
    (name, Some(location.line as u32))
}

fn analyze_mir(tcx: TyCtxt<'_>, shared_report: Arc<Mutex<SharedReport>>) {
    let started = std::time::Instant::now();
    if tcx.data_layout.pointer_size().bits() != usize::BITS as u64 {
        eprintln!("[Keller-Pipeline] Cross-width target is unsupported: target pointer width differs from verifier host.");
        std::process::exit(crate::EXIT_SETUP);
    }
    let cfg = solver_config();
    let mut function_count = 0;
    let mut coverage = Coverage::default();
    let mut folded_passes = 0;
    // Findings whose line the worker printed, against the `--max-findings` cap.
    let line_cap = finding_line_cap();
    let mut printed_findings = 0usize;
    // Obligations are filed per crate so parallel workers cannot clobber them.
    let crate_name = tcx.crate_name(LOCAL_CRATE).to_string();
    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let cache_enabled = std::env::var("KELLER_NO_CACHE")
        .map(|v| v != "1")
        .unwrap_or(true);
    let mut verification_cache = if cache_enabled {
        crate::cache::VerificationCache::load_for_crate(&current_dir, &crate_name)
    } else {
        crate::cache::VerificationCache::default()
    };

    // S7: the helpers this crate's own call sites decide the precondition of.
    // Built once, from the MIR alone, before any function is analysed, so a
    // helper's verdict does not depend on the order its callers are walked in.
    let summaries = crate::summary::build(tcx);

    for &local_def_id in tcx.mir_keys(()) {
        let def_id = local_def_id.to_def_id();
        if !matches!(
            tcx.def_kind(def_id),
            DefKind::Fn | DefKind::AssocFn | DefKind::Closure
        ) {
            continue;
        }
        if !tcx.is_mir_available(def_id) {
            continue;
        }

        let mir = tcx.optimized_mir(def_id);
        let function_name = tcx.def_path_str(def_id);
        if is_panic_internal(&function_name) {
            continue;
        }
        let (file, line) = source_location(tcx, def_id);

        let contracts = crate::contract::extract_fn_contracts(tcx, def_id);
        let fingerprint = crate::cache::compute_mir_fingerprint(&mir, &contracts);

        // A summarised helper's verdict depends on other functions' call sites, so
        // it is not a function of this MIR alone: its cache entry is neither read
        // nor written while the summary is in force.
        let cacheable = cache_enabled && !summaries.contains_key(&def_id);
        if cacheable {
            if let Some(cached) = verification_cache.get(&function_name, fingerprint) {
                // An entry written with emission off carries no artifacts, but
                // its events still demand files. Treat that as a miss rather
                // than publishing a manifest that indexes nothing.
                let artifacts_ok = !crate::report::obligations_enabled()
                    || cached.obligations.len()
                        == crate::report::expected_obligation_artifacts(&cached.events);
                if artifacts_ok {
                    if let Err(err) = crate::report::restore_smt_obligations(
                        &crate_name,
                        &cached
                            .obligations
                            .iter()
                            .map(|obligation| {
                                (obligation.artifact.clone(), obligation.contents.clone())
                            })
                            .collect::<Vec<_>>(),
                    ) {
                        eprintln!("[Keller-Pipeline] {err}");
                        std::process::exit(crate::EXIT_SETUP);
                    }
                    for event in &cached.events {
                        shared_report.lock().unwrap().events.push(event.clone());
                    }
                    if let Some(evidence) = &cached.evidence {
                        shared_report
                            .lock()
                            .unwrap()
                            .evidence
                            .push(evidence.clone());
                    }
                    coverage.supported_assertions += cached.supported_assertions;
                    coverage.unsupported_assertions += cached.unsupported_assertions;
                    // A cached function contributes exactly the assertions it was
                    // analysed with. Restoring `supported + unsupported` keeps the
                    // receipt's `reachable == supported + unsupported` invariant true
                    // on a cache hit; before this, a cached run reported zero
                    // reachable assertions beside a non-zero supported count, and the
                    // merged report refused the receipt as inconsistent.
                    coverage.reachable_assertions +=
                        cached.supported_assertions + cached.unsupported_assertions;
                    coverage.panic_sites += cached.panic_sites;
                    coverage.panic_sites_decided += cached.panic_sites_decided;
                    // Folded proofs leave no `PASS` event, so the count cannot be
                    // recomputed from the restored events.
                    folded_passes += cached.folded_passes;
                    function_count += 1;
                    continue;
                }
                // The entry is incomplete for this run: fall through to a fresh
                // analysis, which re-emits the missing artifacts.
            }
        }

        let func_events_start = shared_report.lock().unwrap().events.len();
        let func_evidence_start = shared_report.lock().unwrap().evidence.len();
        let func_supported_start = coverage.supported_assertions;
        let func_unsupported_start = coverage.unsupported_assertions;
        let func_panic_start = coverage.panic_sites;
        let func_panic_decided_start = coverage.panic_sites_decided;
        let func_folded_start = folded_passes;
        // Every SMT-LIB file this function writes, kept for the cache entry so a
        // cache hit can republish exactly the evidence the first run produced.
        let mut func_obligations: Vec<(String, String)> = Vec::new();

        let reachable = compute_reachable_blocks(&mir);

        // Per-function Z3 context: terms and solver state are freed after each function,
        // bounding memory usage to < 100 MB across large codebases.
        let ctx = Context::new(&cfg);

        // -----------------------------------------------------------------
        // PASS 1: Constant propagation via SSA-like MIR walk
        // -----------------------------------------------------------------
        let const_values = propagate_constants(&mir, &reachable, tcx);

        // -----------------------------------------------------------------
        // PASS 2: Path-sensitive symbolic state over the whole CFG
        // -----------------------------------------------------------------
        let param_to_local = crate::contract::build_param_mapping(mir);
        let mut entry_guard = None;
        if !contracts.requires.is_empty() {
            let mut initial_state = PathState::initial();
            let mut req_bools = Vec::new();
            for (_raw_req, req_expr) in &contracts.requires {
                let mut resolver = crate::contract::BodyVarResolver {
                    ctx: &ctx,
                    mir,
                    const_map: &const_values,
                    tcx,
                    state: &mut initial_state,
                    name_to_local: &param_to_local,
                };
                if let Ok(crate::contract::Z3Val::Bool(b)) = req_expr.eval(&ctx, &mut resolver) {
                    req_bools.push(b);
                }
            }
            if !req_bools.is_empty() {
                let refs: Vec<&z3::ast::Bool<'_>> = req_bools.iter().collect();
                entry_guard = Some(crate::path_state::all_of(&ctx, &refs));
            }
        }

        // A helper whose precondition its own call sites were shown to satisfy is
        // analysed under it (S7): the bound is a fact this run checked, so the
        // body is asked about the values its callers actually pass instead of about
        // all of them. Both guards are kept when both exist — `#[requires]` is the
        // user's statement about the function, the summary is its callers'.
        if let Some(summary) = summaries.get(&def_id) {
            if let Some(guard) =
                crate::summary::entry_guard(summary, &ctx, &mir, &const_values, tcx)
            {
                // Said out loud, because the body is about to be analysed under a
                // condition nobody wrote in it: the bound is the one its callers
                // were shown to satisfy, and this line is where a reader can see
                // that rather than infer it from a PASS.
                print_finding(
                    line_cap,
                    &mut printed_findings,
                    format!(
                        "[PASS] Helper summary in {function_name}: `{}` is checked at all {} call site(s), so this body is analysed under it",
                        summary
                            .bounds
                            .iter()
                            .map(|bound| bound.describe())
                            .collect::<Vec<_>>()
                            .join(", "),
                        summary.sites
                    ),
                );
                entry_guard = Some(match entry_guard {
                    Some(requires) => z3::ast::Bool::and(&ctx, &[&requires, &guard]),
                    None => guard,
                });
            }
        }

        // Computed once per function: every obligation in a block is asked about
        // the same (values, guards) state, so an assertion guarded by a branch is
        // only checked for states the branch actually allows.
        let paths = PathAnalysis::compute(&ctx, &mir, &reachable, &const_values, tcx, entry_guard);
        let mut disclosures = std::collections::BTreeSet::new();
        disclosures.insert("concrete execution not checked".to_string());
        for (_, block) in mir.basic_blocks.iter_enumerated() {
            if matches!(block.terminator().kind, TerminatorKind::Call { .. }) {
                disclosures.insert("callee behavior/return values may be abstracted".to_string());
            }
            for statement in &block.statements {
                if let StatementKind::Assign(box (_, value)) = &statement.kind {
                    if !matches!(
                        value,
                        Rvalue::Use(..)
                            | Rvalue::BinaryOp(..)
                            | Rvalue::UnaryOp(..)
                            | Rvalue::Cast(rustc_middle::mir::CastKind::IntToInt, ..)
                    ) {
                        disclosures.insert(
                            "memory/aggregate/unhandled values may be abstracted".to_string(),
                        );
                    }
                    if matches!(
                        value,
                        Rvalue::BinaryOp(
                            BinOp::AddUnchecked
                                | BinOp::SubUnchecked
                                | BinOp::MulUnchecked
                                | BinOp::ShlUnchecked
                                | BinOp::ShrUnchecked,
                            ..
                        )
                    ) {
                        disclosures
                            .insert("unchecked-operation validity is not proved".to_string());
                    }
                }
            }
        }
        if mir.basic_blocks.iter_enumerated().any(|(bb, block)| {
            block
                .terminator()
                .successors()
                .any(|target| target.index() <= bb.index())
        }) {
            disclosures.insert("cycles may use loop widening".to_string());
        }
        if paths.path_insensitivity_note().is_some() {
            disclosures.insert("path-insensitive fallback".to_string());
        }
        shared_report
            .lock()
            .unwrap()
            .evidence
            .push(FunctionEvidence {
                function: function_name.clone(),
                disclosures: disclosures.into_iter().collect(),
            });

        // -----------------------------------------------------------------
        // PASS 2b: A lost precision is a finding of its own
        // -----------------------------------------------------------------
        // When the dataflow cannot reach a fixpoint it falls back to the
        // path-insensitive state, which is sound but costs every guard in the
        // function — and until this was recorded, its only outward sign was a
        // FAIL the guards would have ruled out. That is the worst shape a false
        // positive can take: a reader has no way to tell it from a real one.
        //
        // It is recorded as a WARN with a `kind` of its own, so a consumer can
        // separate it from a solver that answered `unknown` while the exit-code
        // contract keeps treating both as "nothing was proved": under
        // `--deny-warn` a lost precision fails the build, which is the point of
        // asking for a definite answer.
        if let Some(note) = paths.path_insensitivity_note() {
            eprintln!(
                "[WARN] Path-insensitive fallback in {}: a FAIL in this function may be a false positive",
                function_name
            );
            let event = precision_event(note, &function_name, &file, line);
            let mut report_guard = shared_report.lock().unwrap();
            report_guard.events.push(event);
        }

        // -----------------------------------------------------------------
        // PASS 3a: the panic sites this analysis does not model (A6)
        // -----------------------------------------------------------------
        // Everything below is about *assertions*: conditions the compiler
        // inserted, each of which gets a proof attempt. `unwrap`, `expect`,
        // `panic!`, `unreachable!` and the failure path of a user `assert!` are
        // not assertions - they are calls into another crate whose body this
        // analysis does not read - and until this pass they left no trace at all.
        // A report is a list of what was checked, so a reader could take "no
        // findings" for "this cannot panic", which is a claim nothing here
        // earns.
        //
        // So every reachable call site that can panic through a callee from
        // another crate is recorded: a WARN naming the callee and the block,
        // counted in `coverage.panic_sites`, never silence. Not a FAIL, because
        // nothing was proved false, and not an `unsupported-assertion` either,
        // because the site is not an assertion - it is the statement that this
        // path was not decided. `--deny-warn` is what makes it fatal, which is
        // why every exit code this analysis produced before is unchanged.
        //
        // What it *decides* is the other half of A6. A site is decided when the
        // analysis already knows the one thing it needs:
        //
        //   * no input reaches the block (`path_is_reachable` is unsat), so the
        //     call cannot run and the site is proved safe;
        //   * the receiver of an `unwrap`/`expect` family call has a known enum
        //     variant, so its contract is answered from the variant rather than
        //     from the callee's unread body. The success variant returns; the
        //     failure variant is a settled FAIL, with no solver call because the
        //     fact is ground.
        //
        // A decided site leaves `panic_sites` (which keeps meaning "not
        // modelled") and is counted in `panic_sites_decided`, so a reader can
        // tell silence from a proof.
        for (bb, block_data) in mir.basic_blocks.iter_enumerated() {
            if !reachable.contains(&bb) {
                continue;
            }
            let Some(terminator) = &block_data.terminator else {
                continue;
            };
            let (func, args) = match &terminator.kind {
                TerminatorKind::Call { func, args, .. }
                | TerminatorKind::TailCall { func, args, .. } => (func, args),
                _ => continue,
            };
            let Some(class) = unmodelled_panic(tcx, func) else {
                continue;
            };
            let callee = operand_label(mir, func);
            let state = paths.state(bb);

            // The block's guard is unsatisfiable: no input reaches the call. The
            // `reachable` set is CFG reachability, which cannot see a branch whose
            // guard is contradictory, so this is a fact only the symbolic state
            // knows.
            if let Some(state) = state {
                if !path_is_reachable(&ctx, state) {
                    coverage.panic_sites_decided += 1;
                    continue;
                }
            }

            // A known receiver variant answers `unwrap`, `expect`, `unwrap_err`
            // and `expect_err` outright, or per-value argument provenance decides
            // bounds/indexing calls (A6).
            if let Some((panics, variant)) = state.and_then(|state| {
                decided_panic_site(tcx, mir, state, &const_values, func, args)
            }) {
                coverage.panic_sites_decided += 1;
                if panics {
                    let event = decided_panic_event(
                        &callee,
                        &variant,
                        &function_name,
                        &format!("{bb:?}"),
                        &file,
                        line,
                    );
                    print_finding(
                        line_cap,
                        &mut printed_findings,
                        format!(
                            "[FAIL] Decided panic site in {} ({bb:?}): {callee} holds {variant}",
                            function_name
                        ),
                    );
                    shared_report.lock().unwrap().events.push(event);
                }
                continue;
            }

            coverage.panic_sites += 1;
            let event = panic_site_event(
                class,
                &callee,
                &function_name,
                &format!("{bb:?}"),
                &file,
                line,
            );
            print_finding(
                line_cap,
                &mut printed_findings,
                format!(
                    "[WARN] Unmodelled panic site in {} ({bb:?}): {callee}",
                    function_name
                ),
            );
            shared_report.lock().unwrap().events.push(event);
        }

        // -----------------------------------------------------------------
        // PASS 3: Analyze each assert terminator
        // -----------------------------------------------------------------
        let mut solver = Solver::new(&ctx);
        let solver_budget = solver_timeout_budget();

        for (bb, block_data) in mir.basic_blocks.iter_enumerated() {
            if !reachable.contains(&bb) {
                continue;
            }
            let Some(terminator) = &block_data.terminator else {
                continue;
            };
            let TerminatorKind::Assert {
                msg,
                cond,
                // The polarity is already folded into the guard recorded for the
                // assert's success edge, so the obligation does not need it.
                expected,
                ..
            } = &terminator.kind
            else {
                continue;
            };
            coverage.reachable_assertions += 1;
            // A null or misaligned check is a statement about the *origin* of
            // the address it tests. Once that origin is a reference, a box or a
            // `NonNull`, the check is decided: those pointers are non-null and
            // aligned by construction. Left unmodelled it would be a WARN, which
            // is what it was until the provenance model existed.
            if matches!(
                &**msg,
                AssertKind::NullPointerDereference
                    | AssertKind::NullReferenceConstructed
                    | AssertKind::MisalignedPointerDereference { .. }
            ) {
                match crate::pointer::decide(&mir, &const_values, tcx, cond, *expected) {
                    Some(crate::pointer::PointerDecision::Proved(details)) => {
                        coverage.supported_assertions += 1;
                        let pointer_event = VerificationEvent {
                            crate_name: String::new(),
                            function: function_name.clone(),
                            block: format!("{bb:?}"),
                            status: "PASS".to_string(),
                            kind: "pointer-provenance".to_string(),
                            defect_class: String::new(),
                            file: file.clone(),
                            line,
                            details: details.clone(),
                            witness: Vec::new(),
                            witness_kind: String::new(),
                        };
                        print_finding(
                            line_cap,
                            &mut printed_findings,
                            format!("[PASS] {details} in {function_name} ({bb:?})"),
                        );
                        shared_report.lock().unwrap().events.push(pointer_event);
                        continue;
                    }
                    Some(crate::pointer::PointerDecision::Refuted {
                        reason,
                        address,
                        kind,
                    }) => {
                        coverage.supported_assertions += 1;
                        let witness = vec![Witness {
                            symbol: "address".to_string(),
                            input: "address".to_string(),
                            value: format!("{address:#x}"),
                        }];
                        let pointer_event = VerificationEvent {
                            crate_name: String::new(),
                            function: function_name.clone(),
                            block: format!("{bb:?}"),
                            status: "FAIL".to_string(),
                            kind: kind.to_string(),
                            defect_class: "pointer-violation".to_string(),
                            file: file.clone(),
                            line,
                            details: format!("Refuted by pointer model: {reason}"),
                            witness: witness.clone(),
                            witness_kind: "pinned".to_string(),
                        };
                        print_finding(
                            line_cap,
                            &mut printed_findings,
                            format!(
                                "[FAIL] {} in {function_name} ({bb:?}) [address = {address:#x}]",
                                pointer_event.details
                            ),
                        );
                        let obligation = format!(
                            "(declare-const address (_ BitVec 64))\n(assert (= address (_ bv{address} 64)))\n(check-sat)\n"
                        );
                        func_obligations.extend(
                            write_smt_obligation(
                                &crate_name,
                                &pointer_event,
                                "FAIL (sat)",
                                &witness,
                                None,
                                &obligation,
                            )
                            .unwrap_or_else(|err| {
                                eprintln!("[Keller-Pipeline] {err}");
                                std::process::exit(crate::EXIT_SETUP);
                            }),
                        );
                        shared_report.lock().unwrap().events.push(pointer_event);
                        continue;
                    }
                    None => {}
                }
            }
            if !is_user_assertion(msg) {
                coverage.unsupported_assertions += 1;
                // The kind without its operands: `BoundsCheck { .. }`-style debug
                // output is per-site and would make the list unrunnable, while the
                // variant name is what identifies the missing encoding.
                let kind_name = format!("{msg:?}")
                    .split([' ', '('])
                    .next()
                    .unwrap_or_default()
                    .to_string();
                let event = unsupported_event(
                    &format!("{msg:?}"),
                    &function_name,
                    &format!("{bb:?}"),
                    &file,
                    line,
                );
                // Named, not just counted: the kinds this build cannot model are
                // the shortlist of what to model next, and a count alone cannot
                // say which one a crate hit.
                if !coverage.unsupported_kinds.contains(&kind_name) {
                    coverage.unsupported_kinds.push(kind_name);
                }
                print_finding(
                    line_cap,
                    &mut printed_findings,
                    format!("[WARN] {} in {} ({bb:?})", event.details, function_name),
                );
                shared_report.lock().unwrap().events.push(event);
                continue;
            }
            coverage.supported_assertions += 1;

            // The state this assertion is checked in: the symbolic value of every
            // modelled local, and the condition required to be here at all.
            let mut state = paths.state(bb).cloned().unwrap_or_else(PathState::initial);
            let has_bitwise = state.has_bitwise_ops()
                || matches!(
                    &**msg,
                    AssertKind::Overflow(
                        BinOp::Shl | BinOp::Shr | BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor,
                        ..
                    )
                );
            let mut solver = if has_bitwise {
                crate::constraint::make_bitvector_solver(&ctx, solver_budget)
            } else {
                Solver::new(&ctx)
            };

            let mut event = VerificationEvent {
                // Stamped with the crate name when the report is written.
                crate_name: String::new(),
                function: function_name.clone(),
                block: format!("{:?}", bb),
                status: "UNKNOWN".to_string(),
                kind: String::new(),
                // Set only if this obligation turns out to be a FAIL.
                defect_class: String::new(),
                file: file.clone(),
                line,
                details: String::new(),
                witness: Vec::new(),
                // Set only if this obligation turns out to be a FAIL with a
                // witness; see `VerificationEvent::witness_kind`.
                witness_kind: String::new(),
            };

            // Why this obligation would be undecided rather than failed: the
            // reasons collected while resolving its operands, i.e. the values the
            // violation is stated over that this analysis did not model.
            let mut undecided: BTreeSet<String> = BTreeSet::new();

            // The variables to explain a SAT verdict in terms of, or `None` when
            // the operation was not modelled at all.
            let witness_vars: Option<Vec<WitnessVar>> = {
                let mut src = SymCtx {
                    ctx: &ctx,
                    mir,
                    const_map: &const_values,
                    state: &mut state,
                    tcx,
                    undecided: &mut undecided,
                };

                match &**msg {
                    // -------------------------------------------------------------
                    // BOUNDS CHECK
                    // -------------------------------------------------------------
                    AssertKind::BoundsCheck { len, index } => {
                        event.kind = "bounds".to_string();
                        let index_str = format!("{:?}", index);
                        // A7 residual: this used to `continue` — the assertion was
                        // counted as reached, then dropped without a verdict, a
                        // kind or a word on the console, so a bounds check the
                        // filter could not name disappeared from the report as
                        // though it had been proved. It falls through to a WARN
                        // now, which is the rule stated for an
                        // assertion the build cannot check: report it, never
                        // nothing.
                        //
                        // The assertion was already counted as *supported* above,
                        // so the two counters are adjusted rather than one
                        // incremented: the receipt's own consistency check requires
                        // `reachable == supported + unsupported` and
                        // `unsupported == the WARN findings that name a kind`,
                        // and moving the count is what keeps both true.
                        if is_panic_internal(&index_str) {
                            coverage.supported_assertions -= 1;
                            coverage.unsupported_assertions += 1;
                            let kind_name = "PanicInternalBoundsIndex".to_string();
                            if !coverage.unsupported_kinds.contains(&kind_name) {
                                coverage.unsupported_kinds.push(kind_name);
                            }
                            let reason = format!(
                                "the bounds index `{index_str}` is a panic-internal place, so no proof was attempted"
                            );
                            let mut unsupported = unsupported_event(
                                &format!("BoundsCheck over a panic-internal place ({index_str})"),
                                &function_name,
                                &format!("{bb:?}"),
                                &file,
                                line,
                            );
                            unsupported.details = format!("Unsupported MIR assertion: {reason}.");
                            print_finding(
                                line_cap,
                                &mut printed_findings,
                                format!(
                                    "[WARN] Unsupported in {} ({bb:?}): {reason}",
                                    function_name
                                ),
                            );
                            shared_report.lock().unwrap().events.push(unsupported);
                            continue;
                        }

                        // Try direct evaluation when both index and len are known constants
                        let idx_val = eval_operand(index, &mir, &const_values, tcx);
                        let len_val = eval_operand(len, &mir, &const_values, tcx);

                        // Keep the operands before the match shadows them: the arm
                        // below names its folded *values* `idx` and `len`, and a
                        // witness has to report the operand, not the number.
                        let index_operand = index;
                        let len_operand = len;
                        match (idx_val, len_val) {
                            (Some(idx), Some(len)) => {
                                // Both known: direct evaluation
                                if idx >= len {
                                    if !path_is_reachable(&ctx, src.state) {
                                        folded_passes += 1;
                                        continue;
                                    }
                                    event.details = format!(
                                        "Slice Boundary Analysis: constant index {} >= len {}",
                                        idx, len
                                    );
                                    // Ground counterpart of the bounds obligation:
                                    // the index and the length pinned to the
                                    // constants that were folded, and the
                                    // violation `index >= len`.
                                    let instance = format!(
                                        "{}{}(assert (>= folded_index folded_len))\n",
                                        ground_int("folded_index", &idx.to_string()),
                                        ground_int("folded_len", &len.to_string()),
                                    );
                                    let witness = vec![
                                        Witness {
                                            input: operand_label(&mir, index_operand),
                                            symbol: "folded_index".to_string(),
                                            value: idx.to_string(),
                                        },
                                        Witness {
                                            input: operand_label(&mir, len_operand),
                                            symbol: "folded_len".to_string(),
                                            value: len.to_string(),
                                        },
                                    ];
                                    func_obligations.extend(record_folded_failure(
                                        &shared_report,
                                        &crate_name,
                                        event,
                                        witness,
                                        instance,
                                    ));
                                }
                                if idx < len {
                                    folded_passes += 1;
                                }
                                continue;
                            }
                            _ => {
                                // At least one unknown: use Z3 with type-aware bounds
                                event.details = "Slice Boundary Analysis".to_string();
                                build_bounds_constraints(
                                    &mut src,
                                    &mut solver,
                                    index,
                                    len,
                                    operand_int_type(index, &mir, tcx),
                                )
                            }
                        }
                    }

                    // -------------------------------------------------------------
                    // ARITHMETIC OVERFLOW
                    // -------------------------------------------------------------
                    AssertKind::Overflow(op, left, right) => {
                        event.kind = "overflow".to_string();
                        let lv = eval_operand(left, &mir, &const_values, tcx);
                        let rv = eval_operand(right, &mir, &const_values, tcx);

                        match (lv, rv) {
                            (Some(l), Some(r)) => {
                                // Both constants: direct overflow check against the
                                // operand's actual integer type (u8 wraps at 255, not i128).
                                let int_ty =
                                    operand_int_type(left, &mir, tcx).unwrap_or(IntType::I128);
                                let would_overflow = overflow_detected(*op, l, r, int_ty);
                                if would_overflow {
                                    if !path_is_reachable(&ctx, src.state) {
                                        folded_passes += 1;
                                        continue;
                                    }
                                    event.details =
                                        format!("Arithmetic Overflow: {:?} {:?} {:?}", l, op, r);
                                    let instance = folded_overflow_instance(*op, l, r, int_ty);
                                    let witness = vec![
                                        Witness {
                                            input: operand_label(&mir, left),
                                            symbol: "folded_l".to_string(),
                                            value: folded_literal(l, int_ty),
                                        },
                                        Witness {
                                            input: operand_label(&mir, right),
                                            symbol: "folded_r".to_string(),
                                            value: folded_literal(r, int_ty),
                                        },
                                    ];
                                    func_obligations.extend(record_folded_failure(
                                        &shared_report,
                                        &crate_name,
                                        event,
                                        witness,
                                        instance,
                                    ));
                                }
                                if !would_overflow {
                                    folded_passes += 1;
                                }
                                continue;
                            }
                            _ => {
                                // Use Z3 with type-aware bounds
                                event.details = format!("Arithmetic Overflow Detection ({:?})", op);
                                let encoded = build_overflow_constraints(
                                    &mut src,
                                    &mut solver,
                                    *op,
                                    left,
                                    right,
                                    operand_int_type(left, &mir, tcx),
                                );
                                match encoded {
                                    Encoding::Modelled(vars) => Some(vars),
                                    // A proof, reported as one: "checked and safe"
                                    // must not look like "not checked".
                                    Encoding::Impossible => {
                                        event.status = "PASS".to_string();
                                        event.details = format!(
                                            "{:?} cannot overflow for this operand type",
                                            op
                                        );
                                        if !quiet() {
                                            println!(
                                                "[PASS] {:?} cannot overflow: {} ({:?})",
                                                op, function_name, bb
                                            );
                                        }
                                        let mut report_guard = shared_report.lock().unwrap();
                                        report_guard.events.push(event);
                                        continue;
                                    }

                                    // The obligation exists in the program but this
                                    // encoder cannot express it. WARN, never
                                    // silence: an assertion we did not check is not
                                    // an assertion that holds.
                                    Encoding::Unmodelled(reason) => {
                                        event.status = "WARN".to_string();
                                        event.details = format!("Not modelled: {reason}");
                                        print_finding(
                                            line_cap,
                                            &mut printed_findings,
                                            format!(
                                                "[WARN] Not modelled in {} ({:?}): {}",
                                                function_name, bb, reason
                                            ),
                                        );
                                        let mut report_guard = shared_report.lock().unwrap();
                                        report_guard.events.push(event);
                                        continue;
                                    }
                                }
                            }
                        }
                    }

                    // -------------------------------------------------------------
                    // NEGATION OVERFLOW (-x)
                    // -------------------------------------------------------------
                    AssertKind::OverflowNeg(operand) => {
                        event.kind = "overflow-neg".to_string();
                        let ty = operand_int_type(operand, &mir, tcx);
                        let val = eval_operand(operand, &mir, &const_values, tcx);

                        match (val, ty) {
                            (Some(v), Some(ty)) => {
                                // Both the value and its type are known: direct check.
                                // Signed: -MIN overflows. Unsigned: -x overflows for x != 0.
                                let would_overflow = if ty.is_unsigned() {
                                    v != 0
                                } else {
                                    let (min, _) = ty.bounds();
                                    v == min
                                };
                                if would_overflow {
                                    if !path_is_reachable(&ctx, src.state) {
                                        folded_passes += 1;
                                        continue;
                                    }
                                    event.details =
                                        "Arithmetic Overflow: negation of MIN".to_string();
                                    let instance = folded_neg_instance(v, ty);
                                    let witness = vec![Witness {
                                        input: operand_label(&mir, operand),
                                        symbol: "folded_operand".to_string(),
                                        value: folded_literal(v, ty),
                                    }];
                                    func_obligations.extend(record_folded_failure(
                                        &shared_report,
                                        &crate_name,
                                        event,
                                        witness,
                                        instance,
                                    ));
                                }
                                if !would_overflow {
                                    folded_passes += 1;
                                }
                                continue;
                            }
                            _ => {
                                event.details = "Arithmetic Overflow Detection (Neg)".to_string();
                                build_neg_overflow_constraints(&mut src, &mut solver, operand, ty)
                            }
                        }
                    }

                    // -------------------------------------------------------------
                    // DIVISION BY ZERO
                    // -------------------------------------------------------------
                    AssertKind::DivisionByZero(numerator) => {
                        event.kind = "divzero".to_string();
                        let divisor = divisor_of(&mir, bb, cond, &const_values, tcx, numerator);
                        if eval_operand(&divisor, &mir, &const_values, tcx) == Some(0) {
                            if !path_is_reachable(&ctx, src.state) {
                                folded_passes += 1;
                                continue;
                            }
                            event.details = "Division By Zero: constant zero divisor".to_string();
                            let instance = format!(
                                "{}(assert (= folded_divisor 0))\n",
                                ground_int("folded_divisor", "0"),
                            );
                            let witness = vec![Witness {
                                input: operand_label(&mir, &divisor),
                                symbol: "folded_divisor".to_string(),
                                value: "0".to_string(),
                            }];
                            func_obligations.extend(record_folded_failure(
                                &shared_report,
                                &crate_name,
                                event,
                                witness,
                                instance,
                            ));
                            continue;
                        }

                        let divisor_ty = operand_int_type(&divisor, &mir, tcx);
                        event.details = "Division By Zero Analysis".to_string();
                        build_divzero_constraints(&mut src, &mut solver, &divisor, divisor_ty)
                    }

                    // -------------------------------------------------------------
                    // REMAINDER BY ZERO
                    // -------------------------------------------------------------
                    AssertKind::RemainderByZero(numerator) => {
                        event.kind = "remainder".to_string();
                        let divisor = divisor_of(&mir, bb, cond, &const_values, tcx, numerator);
                        if eval_operand(&divisor, &mir, &const_values, tcx) == Some(0) {
                            if !path_is_reachable(&ctx, src.state) {
                                folded_passes += 1;
                                continue;
                            }
                            event.details = "Remainder By Zero: constant zero divisor".to_string();
                            let instance = format!(
                                "{}(assert (= folded_divisor 0))\n",
                                ground_int("folded_divisor", "0"),
                            );
                            let witness = vec![Witness {
                                input: operand_label(&mir, &divisor),
                                symbol: "folded_divisor".to_string(),
                                value: "0".to_string(),
                            }];
                            func_obligations.extend(record_folded_failure(
                                &shared_report,
                                &crate_name,
                                event,
                                witness,
                                instance,
                            ));
                            continue;
                        }

                        // Remainder-by-zero uses the same constraint as division-by-zero
                        let divisor_ty = operand_int_type(&divisor, &mir, tcx);
                        event.details = "Remainder By Zero Analysis".to_string();
                        build_divzero_constraints(&mut src, &mut solver, &divisor, divisor_ty)
                    }

                    // -------------------------------------------------------------
                    // COROUTINE RESUME STATE (S6)
                    // -------------------------------------------------------------
                    // The transform dispatches on the coroutine's discriminant at the
                    // resume entry, and the branches for the states it refuses to be
                    // resumed in hold `assert(const false, "…resumed after …")`. The
                    // entry state carries the coroutine ABI's invariant (state 0 or a
                    // suspension state), so those branches are unreachable and the
                    // assertion is discharged by the guard rather than by a model of a
                    // value the message names.
                    AssertKind::ResumedAfterReturn(_)
                    | AssertKind::ResumedAfterPanic(_)
                    | AssertKind::ResumedAfterDrop(_) => {
                        let forbidden = match &**msg {
                            AssertKind::ResumedAfterReturn(_) => "`1` (returned)",
                            AssertKind::ResumedAfterPanic(_) => "`2` (poisoned)",
                            _ => "ResumedAfterDrop (dropped)",
                        };
                        event.kind = "coroutine-resume".to_string();
                        event.details = format!(
                            "Coroutine resume-state assertion: decided under the coroutine ABI, \
                             which resumes a coroutine in state `0` (not started) or a suspension \
                             state, never {forbidden}"
                        );
                        // Two independent ways to discharge it: the CFG says every
                        // edge here selects a forbidden state, or the propagated
                        // guard (present when the body reached its fixpoint) says the
                        // block is unreachable. The first is what survives the
                        // path-insensitive fallback, which replaces every block's
                        // state with one that claims nothing.
                        let by_cfg = crate::path_state::resume_state_excludes_block(&mir, tcx, bb);
                        if by_cfg || !path_is_reachable(&ctx, src.state) {
                            event.status = "PASS".to_string();
                            event.details = format!(
                                "{} — {}",
                                event.details,
                                if by_cfg {
                                    "every edge into the block selects a state a resume cannot arrive in"
                                } else {
                                    "the branch that reaches it is unreachable"
                                }
                            );
                            if !quiet() {
                                println!(
                                    "[PASS] coroutine resume state in {} ({:?}): not {forbidden}",
                                    function_name, bb
                                );
                            }
                            shared_report.lock().unwrap().events.push(event);
                            continue;
                        }
                        // Reachable: this path can arrive with the coroutine in the
                        // state the transform forbids, so the assertion is a real
                        // finding. The model is reported by the obligation itself.
                        Some(Vec::new())
                    }

                    _ => None,
                }
            };

            let Some(mut witness_vars) = witness_vars else {
                continue;
            };

            // Assert the path-sensitive facts the obligation is conditioned on:
            // the guard that leads here, the merge definitions, and the bounds of
            // the free inputs. A FAIL that cannot be reproduced is not a bug
            // report, and a PASS that assumes an impossible state is not a proof.
            state.assert_into(&ctx, &mut solver);

            // The symbols the *failure* is stated over, as opposed to the ones the
            // state merely mentions. These are what the report is about, so they
            // are named whether or not the obligation pins their values; the state
            // inputs appended below are named only when it does.
            let failure_symbols: HashSet<String> =
                witness_vars.iter().map(|var| var.symbol.clone()).collect();

            // The names the *classification* may read, which is that set plus the
            // places the failing assertion is stated over. `overflow_l` is a bound
            // variable of the encoding; `mir_1` is the program's own place, and the
            // place is what says whether the value in the finding is one the loop
            // abstraction gave up on. Taken before the state's own inputs are
            // appended below, so `widening-limited` stays a statement about the
            // finding's quantity rather than about whatever else was in scope.
            let mut classified_symbols = failure_symbols.clone();
            for operand in asserted_operands(msg) {
                let place = match operand {
                    Operand::Copy(place) | Operand::Move(place) => place,
                    // A literal has no place and no symbol: it is the same number on
                    // every path, so there is nothing for a widening to have changed.
                    // `RuntimeChecks` is the same shape — the checks a build was
                    // compiled with, not a value read from the program.
                    Operand::Constant(_) | Operand::RuntimeChecks(_) => continue,
                };
                if let Some(name) = state.symbol_of(place) {
                    classified_symbols.insert(name);
                }
            }

            // Name the caller-settable inputs of this path too. An operand with
            // the same label is the same value twice, so it is not repeated.
            for input in state.witness_inputs() {
                if input.kind != crate::path_state::InputKind::Source
                    && !failure_symbols.contains(&input.symbol)
                {
                    continue;
                }
                if witness_vars.iter().any(|var| var.input == input.label) {
                    continue;
                }
                let is_bool = input.is_bool();
                witness_vars.push(WitnessVar {
                    symbol: input.symbol,
                    input: input.label,
                    is_bool,
                });
            }

            let precondition = state.condition();
            // The goal exactly as it stands *before* the check. Dumping the solver
            // afterwards adds its internal model bookkeeping (`model-add`, …) and
            // even stray declarations for symbols the model invented, which is not
            // the question that was asked.
            let obligation = solver.to_string();

            coverage.obligation_solver_calls += 1;
            let check_start = std::time::Instant::now();
            let check_result = solver.check();
            coverage.solver_elapsed_ms += check_start.elapsed().as_millis() as u64;
            match check_result {
                // A model that leans on a value this analysis did not model is not
                // a counterexample: the numbers in it are the solver's choice, not
                // the program's, and reporting them would be claiming a bug from
                // an abstraction hole — which is exactly how a crate of safe
                // functions gets a report full of exploits. Undecided, with the
                // reason; never silence, and never a FAIL.
                SatResult::Sat if !undecided.is_empty() => {
                    event.status = "WARN".to_string();
                    let reason: Vec<String> = undecided.iter().cloned().collect();
                    let reason = reason.join("; ");
                    let headline = format!("{} -> Undecided", event.details);
                    event.details = format!("{headline}: {reason}");
                    print_finding(
                        line_cap,
                        &mut printed_findings,
                        format!("[WARN] Undecided in {} ({:?}): {reason}", function_name, bb),
                    );
                    func_obligations.extend(
                        write_smt_obligation(
                            &crate_name,
                            &event,
                            "WARN (undecided)",
                            // No witness: a model is not evidence here, so there is
                            // nothing to pin and nothing a replay could reproduce.
                            &[],
                            precondition.as_deref(),
                            &obligation,
                        )
                        .unwrap_or_else(|err| {
                            eprintln!("[Keller-Pipeline] {err}");
                            std::process::exit(crate::EXIT_SETUP);
                        }),
                    );
                }
                SatResult::Sat => {
                    event.status = "FAIL".to_string();
                    // Classified from the symbols the *failure* is stated over
                    // (`failure_symbols`, taken before the state's own inputs are
                    // appended below), so `widening-limited` means the value in
                    // this finding depends on a generalised loop value rather
                    // than that the finding merely sits somewhere a widened block
                    // reached.
                    event.defect_class = state.defect_class(&classified_symbols).to_string();
                    // A verdict alone is not a bug report. Ask the
                    // solver for the model and record the concrete
                    // values that trigger the failure, so the FAIL
                    // names an input instead of an exploit path.
                    //
                    // The values are read out of the model *before* it is asked
                    // anything else: Z3's model belongs to the solver's last
                    // `check`, so it has to be consumed while it is still the
                    // current one.
                    let evaluated: Vec<Witness> = match solver.get_model() {
                        Some(model) => extract_witness(&ctx, &model, &witness_vars),
                        None => Vec::new(),
                    };
                    // Keep the operands the failure is stated over, and of
                    // everything else the state mentions only the values the
                    // obligation pins. `a + b` overflows for arbitrarily chosen
                    // `a` and `b`, so the operands stay named either way — but a
                    // loop counter under `i < steps` is a value the *model* chose,
                    // and printing it as a fact about the program is what lets two
                    // runs of one check disagree. The obligation file carries the
                    // guard that bounds it.
                    // Pinnedness is asked about every value kept, including the
                    // operands named unconditionally, because the answer is what
                    // says whether this witness is a fact or one instance among
                    // many (K10 / A14 residual). The solver holds the obligation
                    // here, so the question costs one `check()` per value and is
                    // asked inside a `push`/`pop`.
                    let mut kept: Vec<Witness> = Vec::new();
                    let mut all_pinned = true;
                    for witness in evaluated {
                        let is_operand = failure_symbols.contains(&witness.symbol);
                        if is_operand {
                            let pinned = value_is_pinned(&ctx, &solver, &witness);
                            all_pinned &= pinned;
                            kept.push(witness);
                        } else if kept.len() < 8 {
                            let pinned = value_is_pinned(&ctx, &solver, &witness);
                            if pinned {
                                all_pinned &= pinned;
                                kept.push(witness);
                            }
                        }
                    }
                    event.witness = kept;
                    event.witness_kind = if event.witness.is_empty() {
                        String::new()
                    } else if all_pinned {
                        "pinned".to_string()
                    } else {
                        // At least one entry is a value the obligation only bounds,
                        // so this is one valid instance and a re-run may print a
                        // different one. Saying so is the whole point of the field.
                        "one-of-many".to_string()
                    };
                    event.details.push_str(" -> Exploit path found!");
                    if !event.witness.is_empty() {
                        let rendered = event
                            .witness
                            .iter()
                            .map(|w| format!("{} = {}", w.input, w.value))
                            .collect::<Vec<_>>()
                            .join(", ");
                        event.details.push_str(&format!(" [{}]", rendered));
                    }
                    print_finding(
                        line_cap,
                        &mut printed_findings,
                        format!("[FAIL] {} in {} ({:?})", event.details, function_name, bb),
                    );
                    // Hand back the exact formula, not just a verdict.
                    func_obligations.extend(
                        write_smt_obligation(
                            &crate_name,
                            &event,
                            "FAIL (sat)",
                            &event.witness,
                            precondition.as_deref(),
                            &obligation,
                        )
                        .unwrap_or_else(|err| {
                            eprintln!("[Keller-Pipeline] {err}");
                            std::process::exit(crate::EXIT_SETUP);
                        }),
                    );
                }
                SatResult::Unsat => {
                    event.status = "PASS".to_string();
                    if !quiet() {
                        println!(
                            "[PASS] Verified formal integrity: {} ({:?})",
                            function_name, bb
                        );
                    }
                    // A proof is evidence too, and the same artifact that lets a
                    // third party re-check a counterexample has to exist for a
                    // proof: the obligation is the *negation* of what was proved,
                    // so re-discharging it must answer `unsat` exactly as the run
                    // did. Without this the artifacts were asymmetric — a FAIL
                    // could be checked, a PASS had to be taken on trust.
                    func_obligations.extend(
                        write_smt_obligation(
                            &crate_name,
                            &event,
                            "PASS (unsat)",
                            // No witness: there is no model to pin, the answer is that
                            // none exists.
                            &[],
                            precondition.as_deref(),
                            &obligation,
                        )
                        .unwrap_or_else(|err| {
                            eprintln!("[Keller-Pipeline] {err}");
                            std::process::exit(crate::EXIT_SETUP);
                        }),
                    );
                }
                SatResult::Unknown => {
                    event.status = "WARN".to_string();
                    // Never silenced: an undecided verdict is a finding, and under
                    // `--deny-warn` it is the reason the build fails.
                    print_finding(
                        line_cap,
                        &mut printed_findings,
                        format!("[WARN] Complex state in {} ({:?})", function_name, bb),
                    );
                    func_obligations.extend(
                        write_smt_obligation(
                            &crate_name,
                            &event,
                            "WARN (unknown)",
                            // No model exists for an unknown verdict, so
                            // there is nothing to pin.
                            &[],
                            precondition.as_deref(),
                            &obligation,
                        )
                        .unwrap_or_else(|err| {
                            eprintln!("[Keller-Pipeline] {err}");
                            std::process::exit(crate::EXIT_SETUP);
                        }),
                    );
                }
            }
            let mut report_guard = shared_report.lock().unwrap();
            report_guard.events.push(event);
        }

        // -----------------------------------------------------------------
        // PASS 3b: Contract Preconditions and Postconditions (A8)
        // -----------------------------------------------------------------
        // (1) Call-site Preconditions: For each call to a callee with requires,
        //     the caller must prove the precondition holds at the call site.
        for (bb, block_data) in mir.basic_blocks.iter_enumerated() {
            if !reachable.contains(&bb) {
                continue;
            }
            let Some(terminator) = &block_data.terminator else {
                continue;
            };
            let (func, args) = match &terminator.kind {
                TerminatorKind::Call { func, args, .. } => (func, args),
                _ => continue,
            };
            let Some((callee_def_id, _)) = func.const_fn_def() else {
                continue;
            };
            let callee_contracts = crate::contract::extract_fn_contracts(tcx, callee_def_id);
            if callee_contracts.requires.is_empty() {
                continue;
            }
            let callee_name = tcx.def_path_str(callee_def_id);
            let param_map = crate::contract::build_callsite_mapping(tcx, callee_def_id, args);
            let mut state = paths.state(bb).cloned().unwrap_or_else(PathState::initial);

            for (raw_req, req_expr) in &callee_contracts.requires {
                coverage.reachable_assertions += 1;
                coverage.supported_assertions += 1;

                let mut resolver = crate::contract::CallSiteVarResolver {
                    ctx: &ctx,
                    caller_mir: mir,
                    const_map: &const_values,
                    tcx,
                    state: &mut state,
                    param_to_arg: &param_map,
                    return_place: None,
                };

                let Ok(crate::contract::Z3Val::Bool(cond)) = req_expr.eval(&ctx, &mut resolver)
                else {
                    continue;
                };

                solver.reset();
                state.assert_into(&ctx, &mut solver);
                // Violation: !cond
                solver.assert(&cond.not());

                let mut event = VerificationEvent {
                    crate_name: crate_name.clone(),
                    function: function_name.clone(),
                    block: format!("{:?}", bb),
                    status: "UNKNOWN".to_string(),
                    kind: "contract-precondition".to_string(),
                    defect_class: String::new(),
                    file: file.clone(),
                    line,
                    details: format!("precondition `{raw_req}` for callee `{callee_name}`"),
                    witness: Vec::new(),
                    witness_kind: String::new(),
                };

                coverage.obligation_solver_calls += 1;
                let check_start = std::time::Instant::now();
                let check_result = solver.check();
                coverage.solver_elapsed_ms += check_start.elapsed().as_millis() as u64;

                match check_result {
                    z3::SatResult::Unsat => {
                        event.status = "PASS".to_string();
                        event.details =
                            format!("precondition `{raw_req}` proved for callee `{callee_name}`");
                    }
                    z3::SatResult::Sat => {
                        event.status = "FAIL".to_string();
                        event.defect_class = "contract-precondition-violation".to_string();
                        event.details = format!("precondition `{raw_req}` for callee `{callee_name}` may not be satisfied");
                        let witness_vars: Vec<WitnessVar> = state
                            .witness_inputs()
                            .into_iter()
                            .map(|inp| {
                                let is_bool = inp.is_bool();
                                WitnessVar {
                                    symbol: inp.symbol,
                                    input: inp.label,
                                    is_bool,
                                }
                            })
                            .collect();
                        if let Some(model) = solver.get_model() {
                            event.witness = extract_witness(&ctx, &model, &witness_vars);
                            if !event.witness.is_empty() {
                                event.witness_kind = "model".to_string();
                            }
                        }
                        print_finding(
                            line_cap,
                            &mut printed_findings,
                            format!("[FAIL] Precondition `{raw_req}` for callee `{callee_name}` in {function_name} ({bb:?}) can be violated"),
                        );
                    }
                    z3::SatResult::Unknown => {
                        event.status = "WARN".to_string();
                        event.kind = "undecided-precondition".to_string();
                    }
                }
                shared_report.lock().unwrap().events.push(event);
            }
        }

        // (3) Helper preconditions, checked where they are used (S7)
        //
        // A helper is analysed under its derived precondition only because every
        // one of its call sites was shown to satisfy it ([`crate::summary::build`]
        // refuses the summary otherwise). This pass records that check: one
        // obligation per bound per call site, so the assumption above and the
        // proof that justifies it are in the same report and the same run. Nothing
        // is recorded for a helper that has no summary, which is why an unchecked
        // call site still leaves the body's own `unguarded` finding in place.
        for (bb, block_data) in mir.basic_blocks.iter_enumerated() {
            if !reachable.contains(&bb) {
                continue;
            }
            let Some(terminator) = &block_data.terminator else {
                continue;
            };
            let (func, args) = match &terminator.kind {
                TerminatorKind::Call { func, args, .. }
                | TerminatorKind::TailCall { func, args, .. } => (func, args),
                _ => continue,
            };
            let Some((callee_def_id, _)) = func.const_fn_def() else {
                continue;
            };
            let Some(summary) = summaries.get(&callee_def_id) else {
                continue;
            };
            for bound in &summary.bounds {
                let Some(argument) = bound.local.checked_sub(1).and_then(|index| args.get(index))
                else {
                    continue;
                };
                let arg_desc = if let Some(value) = crate::summary::literal_i128(&argument.node, tcx) {
                    format!("argument {value}")
                } else {
                    format!("argument satisfies bounded range")
                };
                coverage.reachable_assertions += 1;
                coverage.supported_assertions += 1;
                let event = VerificationEvent {
                    crate_name: String::new(),
                    function: function_name.clone(),
                    block: format!("{bb:?}"),
                    status: "PASS".to_string(),
                    kind: "helper-precondition".to_string(),
                    defect_class: String::new(),
                    file: file.clone(),
                    line,
                    details: format!(
                        "derived precondition `{}` for helper `{}` holds at this call site ({arg_desc})",
                        bound.describe(),
                        summary.function
                    ),
                    witness: Vec::new(),
                    witness_kind: String::new(),
                };
                print_finding(
                    line_cap,
                    &mut printed_findings,
                    format!(
                        "[PASS] Helper precondition in {function_name} ({bb:?}): `{}` holds for `{}` ({arg_desc})",
                        bound.describe(),
                        summary.function
                    ),
                );
                shared_report.lock().unwrap().events.push(event);
            }
        }

        // (2) Return-site Postconditions: For each return block in this function,
        //     verify that ensures holds.
        if !contracts.ensures.is_empty() {
            for (bb, block_data) in mir.basic_blocks.iter_enumerated() {
                if !reachable.contains(&bb) {
                    continue;
                }
                let Some(terminator) = &block_data.terminator else {
                    continue;
                };
                if !matches!(terminator.kind, TerminatorKind::Return) {
                    continue;
                }
                let mut state = paths.state(bb).cloned().unwrap_or_else(PathState::initial);

                for (raw_ens, ens_expr) in &contracts.ensures {
                    coverage.reachable_assertions += 1;
                    coverage.supported_assertions += 1;

                    let mut resolver = crate::contract::BodyVarResolver {
                        ctx: &ctx,
                        mir,
                        const_map: &const_values,
                        tcx,
                        state: &mut state,
                        name_to_local: &param_to_local,
                    };

                    let Ok(crate::contract::Z3Val::Bool(cond)) = ens_expr.eval(&ctx, &mut resolver)
                    else {
                        continue;
                    };

                    solver.reset();
                    state.assert_into(&ctx, &mut solver);
                    // Violation: !cond
                    solver.assert(&cond.not());

                    let mut event = VerificationEvent {
                        crate_name: crate_name.clone(),
                        function: function_name.clone(),
                        block: format!("{:?}", bb),
                        status: "UNKNOWN".to_string(),
                        kind: "contract-postcondition".to_string(),
                        defect_class: String::new(),
                        file: file.clone(),
                        line,
                        details: format!("postcondition `{raw_ens}`"),
                        witness: Vec::new(),
                        witness_kind: String::new(),
                    };

                    coverage.obligation_solver_calls += 1;
                    let check_start = std::time::Instant::now();
                    let check_result = solver.check();
                    coverage.solver_elapsed_ms += check_start.elapsed().as_millis() as u64;

                    match check_result {
                        z3::SatResult::Unsat => {
                            event.status = "PASS".to_string();
                            event.details = format!("postcondition `{raw_ens}` proved on return");
                        }
                        z3::SatResult::Sat => {
                            event.status = "FAIL".to_string();
                            event.defect_class = "contract-postcondition-violation".to_string();
                            event.details = format!("ensures condition `{raw_ens}` may not hold on return in `{function_name}`");
                            let witness_vars: Vec<WitnessVar> = state
                                .witness_inputs()
                                .into_iter()
                                .map(|inp| {
                                    let is_bool = inp.is_bool();
                                    WitnessVar {
                                        symbol: inp.symbol,
                                        input: inp.label,
                                        is_bool,
                                    }
                                })
                                .collect();
                            if let Some(model) = solver.get_model() {
                                event.witness = extract_witness(&ctx, &model, &witness_vars);
                                if !event.witness.is_empty() {
                                    event.witness_kind = "model".to_string();
                                }
                            }
                            print_finding(
                                line_cap,
                                &mut printed_findings,
                                format!("[FAIL] Postcondition `{raw_ens}` in {function_name} ({bb:?}) can be violated"),
                            );
                        }
                        z3::SatResult::Unknown => {
                            event.status = "WARN".to_string();
                            event.kind = "undecided-postcondition".to_string();
                        }
                    }
                    shared_report.lock().unwrap().events.push(event);
                }
            }
        }

        if cacheable {
            let events = shared_report.lock().unwrap().events[func_events_start..].to_vec();
            let evidence = shared_report
                .lock()
                .unwrap()
                .evidence
                .get(func_evidence_start)
                .cloned();
            verification_cache.insert(
                function_name.clone(),
                crate::cache::CachedFunctionEntry {
                    fingerprint,
                    events,
                    evidence,
                    supported_assertions: coverage.supported_assertions - func_supported_start,
                    unsupported_assertions: coverage.unsupported_assertions
                        - func_unsupported_start,
                    panic_sites: coverage.panic_sites - func_panic_start,
                    panic_sites_decided: coverage.panic_sites_decided - func_panic_decided_start,
                    folded_passes: folded_passes - func_folded_start,
                    obligations: func_obligations
                        .into_iter()
                        .map(|(artifact, contents)| crate::cache::CachedObligation {
                            artifact,
                            contents,
                        })
                        .collect(),
                },
            );
        }

        function_count += 1;
    }

    if cache_enabled {
        let _ = verification_cache.save_for_crate(&current_dir, &crate_name);
    }

    {
        let mut report = shared_report.lock().unwrap();
        coverage.functions = function_count;
        coverage.passed =
            folded_passes + report.events.iter().filter(|e| e.status == "PASS").count();
        coverage.failed = report
            .events
            .iter()
            .filter(|e| e.status == "FAIL" && !is_non_assertion_kind(&e.kind))
            .count();
        // The kinds that are not a verdict on an assertion are excluded by one
        // predicate shared with the receipt check in `report.rs`, so the counter
        // here and the check there cannot disagree about which they are.
        coverage.warnings = report
            .events
            .iter()
            .filter(|e| e.status == "WARN" && !is_non_assertion_kind(&e.kind))
            .count();
        coverage.without_verdict =
            coverage.supported_assertions - coverage.passed - coverage.failed - coverage.warnings;
        report.coverage = coverage;
        report.analysis_completed = true;
    }
    if !quiet() {
        println!("[Keller] Analyzed {} functions.", function_count);
    }
    print_summary(&shared_report, started.elapsed());
}

/// The event that records a function whose dataflow fell back to the
/// path-insensitive state.
///
/// A `WARN` rather than a verdict: nothing about the program was proved or
/// refuted, and a gate that read it as a pass would be claiming more than was
/// checked. Its `kind` is its own so a reader — or a pipeline — can tell it from
/// a solver that answered `unknown`: one is answered by investigating a CFG
/// shape, the other by raising a solver budget.
///
/// `block` is `body` rather than a `bb…`: the loss is the whole function's, and
/// naming one block would make a narrower claim than the one being reported. The
/// `note` is the explanation [`PathAnalysis::path_insensitivity_note`] built,
/// which is also what the console line summarises.
/// The operands the assertion is *about*, whatever kind it is: the values a
/// counterexample to it would have to be stated over.
///
/// This is what lets a FAIL be classified by the quantity in it rather than by the
/// block it was found in (see `PathState::leans_on_widening`). The match is
/// exhaustive with no wildcard arm, so a new [`AssertKind`] is a compile error here
/// instead of an assertion that silently falls back to "no operands".
fn asserted_operands<'a, 'tcx>(kind: &'a AssertKind<Operand<'tcx>>) -> Vec<&'a Operand<'tcx>> {
    match kind {
        AssertKind::BoundsCheck { len, index } => vec![len, index],
        AssertKind::Overflow(_, left, right) => vec![left, right],
        AssertKind::OverflowNeg(operand) => vec![operand],
        AssertKind::DivisionByZero(operand) => vec![operand],
        AssertKind::RemainderByZero(operand) => vec![operand],
        AssertKind::MisalignedPointerDereference { required, found } => vec![required, found],
        AssertKind::InvalidEnumConstruction(operand) => vec![operand],
        // Nothing *in the message* to be about: the transform emits the resume-state
        // guards as `assert(const false, …)` in the branches its state dispatch
        // takes for `returned`/`poisoned`, so the failure is stated over the
        // coroutine's discriminant rather than over a value the message names. That
        // discriminant is named through the path state's own inputs (see
        // `PathState::seed_resume`). A null reference that was constructed, or a
        // dereference of one, is not a value this analysis models at all.
        AssertKind::ResumedAfterReturn(_)
        | AssertKind::ResumedAfterPanic(_)
        | AssertKind::ResumedAfterDrop(_)
        | AssertKind::NullPointerDereference
        | AssertKind::NullReferenceConstructed => Vec::new(),
    }
}

fn unsupported_event(
    kind: &str,
    function: &str,
    block: &str,
    file: &str,
    line: Option<u32>,
) -> VerificationEvent {
    VerificationEvent {
        crate_name: String::new(),
        function: function.to_string(),
        block: block.to_string(),
        status: "WARN".to_string(),
        kind: "unsupported-assertion".to_string(),
        // Not a FAIL: nothing was decided, so there is no class to report.
        defect_class: String::new(),
        file: file.to_string(),
        line,
        details: format!("Unsupported MIR assertion: {kind}; no proof was attempted."),
        witness: Vec::new(),
        witness_kind: String::new(),
    }
}

/// Why a call site can stop the program without this analysis having decided
/// anything about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PanicSite {
    /// The panic machinery itself: `panic!`, `unreachable!`, `resume_unwind`.
    Panic,
    /// `Option::unwrap`, `Result::expect` and their siblings: a call into `core`
    /// that panics on the case it has no value to return for.
    Unwrap,
    /// The failure path of a user `assert!`, which lowers to a call into
    /// `core::panicking::assert_failed`. The condition is already a guard for
    /// everything after the assertion; what is not modelled is the panic.
    Assert,
    /// `handle_alloc_error`: no panic, but the process ends all the same.
    Abort,
}

impl PanicSite {
    fn reason(self, callee: &str) -> String {
        match self {
            PanicSite::Panic => format!(
                "the call to `{callee}` can panic, and its body is not read by this analysis, so whether this site can panic was not decided"
            ),
            PanicSite::Unwrap => format!(
                "the call to `{callee}` panics when it has no value to return, and its body is not read by this analysis, so whether this site can panic was not decided"
            ),
            PanicSite::Assert => format!(
                "the analysis assumed this assertion holds; the panic the call to `{callee}` raises when it does not is not modelled"
            ),
        PanicSite::Abort => format!(
            "the call to `{callee}` ends the process on allocation failure, which this analysis does not model"
        ),
    }
    }
}

/// The functions the standard library uses to stop a program, by last path
/// segment.
///
/// Named by segment rather than by full path because the *prefix* is not stable
/// that way: this compiler renders `Option::unwrap` as
/// `std::option::Option::<T>::unwrap` and the `panic!` entry point as
/// `std::rt::panic_fmt`, not the `core::option::Option` / `core::panicking::panic`
/// spelling a rule written from the documentation would expect. A predicate that
/// matched the documented spelling reported no panic site at all for a crate full
/// of unwraps — which is the failure this whole pass exists to remove, arriving
/// silently a second time.
const PANIC_ENTRY_POINTS: &[&str] = &[
    // `panic!`, `unreachable!`, `todo!`, `unimplemented!`, and the `panic_fmt`
    // shape the three of them lower to on this compiler.
    "panic",
    "panic_fmt",
    "panic_nounwind",
    "panic_nounwind_fmt",
    "panic_display",
    "panic_explicit",
    "panic_str",
    "begin_panic",
    "begin_panic_handler",
    "panic_cannot_unwind",
    "resume_unwind",
];

/// The panic a call site can raise, when the callee comes from a crate this
/// analysis does not read.
///
/// `None` for a local callee on purpose: its own body is analysed, so a panic in
/// it is reported where it happens rather than again at every call to it.
fn unmodelled_panic(tcx: TyCtxt<'_>, func: &Operand<'_>) -> Option<PanicSite> {
    let (def_id, _) = func.const_fn_def()?;
    if def_id.is_local() {
        return None;
    }
    let krate = tcx.crate_name(def_id.krate);
    let path = tcx.def_path_str(def_id);
    classify_callee(krate.as_str(), &path)
}

/// The classification itself, from the two facts this rule is allowed to use: the
/// crate the callee is defined in, and its path.
///
/// Split out from [`unmodelled_panic`] so it can be tested on the exact strings
/// this compiler produces — which is not a stylistic split. The rule was written
/// from the documented spelling (`core::option::Option::unwrap`,
/// `core::panicking::panic`), and this toolchain spells those `std::option::…` and
/// `std::rt::panic_fmt`, so the first version reported nothing at all for a crate
/// full of unwraps while looking right. The strings below are those measurements,
/// kept as tests rather than as a sentence in a comment.
fn classify_callee(krate: &str, path: &str) -> Option<PanicSite> {
    // The standard library and the allocator specifically, so that a *dependency*
    // of the crate under analysis which happens to export a function named
    // `panic` or `unwrap` is not read as the standard library's contract.
    if !matches!(krate, "std" | "core" | "alloc" | "panic_unwind") {
        return None;
    }
    // The name, with a function's own generic arguments left out of it. They are
    // their own segment at the end of the path
    // (`core::panicking::assert_failed::<u8, u8>`), so taking the last segment
    // literally answers `<u8, u8>` and an `assert_failed` site was not named at
    // all. Generic arguments in the middle of a path are somebody else's segment
    // and are skipped by the same rule.
    let last = path
        .rsplit("::")
        .find(|segment| !segment.starts_with('<'))
        .unwrap_or("");
    if last.starts_with("handle_alloc_error") {
        return Some(PanicSite::Abort);
    }
    // `assert_failed` is the failure path of `assert_eq!` and `assert_ne!`, which
    // is not the same statement as a bare `panic!`: the analysis already assumed
    // the assertion holds, so what is unmodelled is the panic when it does not.
    if last.starts_with("assert_failed") {
        return Some(PanicSite::Assert);
    }
    // The two functions that *are* the panic of an `unwrap` or an `expect`, seen
    // when the wrapper around them was inlined into the caller.
    if last == "unwrap_failed" || last == "expect_failed" {
        return Some(PanicSite::Unwrap);
    }
    if PANIC_ENTRY_POINTS.contains(&last) {
        return Some(PanicSite::Panic);
    }
    // `unwrap` and `expect` are named only for the two types whose contract that
    // is: another type's `unwrap` is that type's own method, and this rule has no
    // business guessing what it does.
    if matches!(last, "unwrap" | "expect" | "unwrap_err" | "expect_err")
        && (path.contains("option::Option") || path.contains("result::Result"))
    {
        return Some(PanicSite::Unwrap);
    }
    None
}

/// Decide an `unwrap`-family call from the receiver's known enum variant.
///
/// Only the calls whose contract *is* the variant are decided here: `unwrap`,
/// `expect`, `unwrap_err` and `expect_err`. The `unwrap_failed`/`expect_failed`
/// shapes are the panic bodies after inlining and carry no receiver, and a
/// `panic!` or an `assert!` call site has no case analysis to make — those stay
/// disclosures.
///
/// `Some((true, variant))` means the call is reached holding the variant it
/// cannot return, so it panics; `Some((false, variant))` means it returns;
/// `None` means the receiver's variant was not tracked, and the site stays
/// undecided. The variant is named rather than assumed from the callee path, so
/// the same rule reads `Option` and `Result` without hardcoding their indices.
fn decided_unwrap<'tcx>(
    tcx: TyCtxt<'tcx>,
    mir: &Body<'tcx>,
    state: &PathState<'_>,
    func: &Operand<'tcx>,
    args: &[rustc_span::Spanned<Operand<'tcx>>],
) -> Option<(bool, String)> {
    let (def_id, _) = func.const_fn_def()?;
    let path = tcx.def_path_str(def_id);
    let last = path
        .rsplit("::")
        .find(|segment| !segment.starts_with('<'))?;
    // `unwrap_err`/`expect_err` panic on the *value* variant, so the polarity is
    // the opposite of `unwrap`/`expect`.
    let err_flavour = matches!(last, "unwrap_err" | "expect_err");
    if !err_flavour && !matches!(last, "unwrap" | "expect") {
        return None;
    }

    let receiver = args.first()?;
    let place = match &receiver.node {
        Operand::Copy(place) | Operand::Move(place) => place,
        Operand::Constant(_) | Operand::RuntimeChecks(_) => return None,
    };
    let discriminant = state.discriminant_of(place)?;
    let ty = place.ty(&mir.local_decls, tcx).ty;
    let TyKind::Adt(adt, _) = ty.kind() else {
        return None;
    };
    if !adt.is_enum() {
        return None;
    }
    let variant = adt
        .variants()
        .iter()
        .enumerate()
        .find(|(index, _)| *index as u32 == discriminant)
        .map(|(_, variant)| variant)?;
    let name = variant.name.to_string();
    let panics = match name.as_str() {
        "Some" | "Ok" => err_flavour,
        "None" | "Err" => !err_flavour,
        // Any other enum is not one whose contract this rule knows, whatever
        // the callee's last path segment looked like.
        _ => return None,
    };
    Some((panics, name))
}

/// Decide a panic site from receiver variant, argument provenance, or bounds (A6).
fn decided_panic_site<'tcx>(
    tcx: TyCtxt<'tcx>,
    mir: &Body<'tcx>,
    state: &PathState<'_>,
    const_map: &ConstMap,
    func: &Operand<'tcx>,
    args: &[rustc_span::Spanned<Operand<'tcx>>],
) -> Option<(bool, String)> {
    if let Some(res) = decided_unwrap(tcx, mir, state, func, args) {
        return Some(res);
    }
    let (def_id, _) = func.const_fn_def()?;
    let path = tcx.def_path_str(def_id);
    let last = path.rsplit("::").find(|segment| !segment.starts_with('<'))?;

    // Bounds and slice indexing checks: e.g. panic_bounds_check, slice_start_index_len_fail, etc.
    if last.contains("bounds_check") || last.contains("index_len_fail") || last.contains("index_order_fail") {
        if args.len() >= 2 {
            let idx = eval_operand(&args[0].node, mir, const_map, tcx);
            let len = eval_operand(&args[1].node, mir, const_map, tcx);
            if let (Some(idx), Some(len)) = (idx, len) {
                if idx < len {
                    return Some((false, format!("in-bounds ({idx} < {len})")));
                } else {
                    return Some((true, format!("out-of-bounds ({idx} >= {len})")));
                }
            }
        }
    }

    // Inlined unwrap_failed / expect_failed
    if last == "unwrap_failed" || last == "expect_failed" {
        for (key, &disc) in state.discriminants() {
            if disc == 0 || disc == 1 {
                return Some((true, format!("receiver {:?} holds failing variant {disc}", key)));
            }
        }
    }

    None
}

/// The event for a panic site whose receiver variant settles the call.
///
/// A FAIL rather than a WARN: the receiver is known to hold the variant the
/// callee cannot return, so this is a counterexample that needs no model and no
/// solver call. The witness is empty on purpose — there is no input to blame,
/// only the value the state already proved.
fn decided_panic_event(
    callee: &str,
    variant: &str,
    function: &str,
    block: &str,
    file: &str,
    line: Option<u32>,
) -> VerificationEvent {
    VerificationEvent {
        crate_name: String::new(),
        function: function.to_string(),
        block: block.to_string(),
        status: "FAIL".to_string(),
        kind: "decided-panic".to_string(),
        defect_class: "known-panic".to_string(),
        file: file.to_string(),
        line,
        details: format!(
            "Decided panic site: the call to `{callee}` is reached with `{variant}`, the variant it cannot return, so it panics whenever this site is reached."
        ),
        witness: Vec::new(),
        witness_kind: String::new(),
    }
}

/// The event that records a call site whose panic behaviour was not decided.
fn panic_site_event(
    class: PanicSite,
    callee: &str,
    function: &str,
    block: &str,
    file: &str,
    line: Option<u32>,
) -> VerificationEvent {
    VerificationEvent {
        crate_name: String::new(),
        function: function.to_string(),
        block: block.to_string(),
        status: "WARN".to_string(),
        kind: "unmodelled-panic".to_string(),
        // Not a FAIL: nothing was decided, so there is no class to report.
        defect_class: String::new(),
        file: file.to_string(),
        line,
        details: format!("Unmodelled panic site: {}.", class.reason(callee)),
        witness: Vec::new(),
        witness_kind: String::new(),
    }
}

fn precision_event(
    note: String,
    function: &str,
    file: &str,
    line: Option<u32>,
) -> VerificationEvent {
    VerificationEvent {
        // Stamped with the crate name when the report is written.
        crate_name: String::new(),
        function: function.to_string(),
        block: "body".to_string(),
        status: "WARN".to_string(),
        kind: "path-insensitive".to_string(),
        // A lost precision is its own kind, not a class of FAIL.
        defect_class: String::new(),
        file: file.to_string(),
        line,
        details: note,
        witness: Vec::new(),
        witness_kind: String::new(),
    }
}

/// A ground `(declare-fun NAME () Int)` and the assertion pinning it to a value.
fn ground_int(name: &str, literal: &str) -> String {
    format!("(declare-fun {name} () Int)\n(assert (= {name} {literal}))\n")
}

/// Render a folded constant in its own type's domain.
///
/// A `u128` does not fit in the `i128` the constant path carries values in —
/// `u128::MAX` arrives as `-1` — so a report or an obligation that printed the
/// `i128` would name an input the program cannot contain, and the ground instance
/// would be unsatisfiable for a failure that is real. The type decides how the
/// bits are read, exactly as [`overflow_detected`] reads them.
fn folded_literal(value: i128, ty: IntType) -> String {
    if ty == IntType::U128 {
        (value as u128).to_string()
    } else {
        value.to_string()
    }
}

/// The ground counterpart of the obligation for a constant-folded overflow.
///
/// The symbolic encoder asserts the violation predicate over the operands' types
/// and asks Z3 whether it is satisfiable. With both operands folded there is
/// nothing to ask, so the same predicate is emitted with the operands pinned; an
/// independent solver evaluates it and must answer `sat`.
///
/// Shifts are checked on the *amount* alone — a value that leaves the type's range
/// is a truncation, not a panic — so their instance is the amount test and not a
/// range test on the result, exactly as [`overflow_detected`] decides it.
fn folded_overflow_instance(op: BinOp, l: i128, r: i128, ty: IntType) -> String {
    let mut instance = ground_int("folded_l", &folded_literal(l, ty));
    instance.push_str(&ground_int("folded_r", &folded_literal(r, ty)));
    match op {
        BinOp::Shl | BinOp::Shr => instance.push_str(&format!(
            "(assert (or (< folded_r 0) (>= folded_r {})))\n",
            ty.bit_width()
        )),
        // `MIN / -1` is the one division that overflows a signed type, and rustc
        // emits an `Overflow` assert for it like any other operation.
        BinOp::Div | BinOp::Rem => {
            let (min_s, _) = ty.bounds_str();
            instance.push_str(&format!(
                "(assert (and (= folded_l {min_s}) (= folded_r (- 1))))\n"
            ));
        }
        _ => {
            let (min_s, max_s) = ty.bounds_str();
            let expr = match op {
                BinOp::Add => "(+ folded_l folded_r)",
                BinOp::Sub => "(- folded_l folded_r)",
                // Multiplication is the remaining operator `overflow_detected`
                // reports on; anything else evaluates to a predicate this does not
                // decide, so the obligation states the violation as unsatisfiable
                // rather than inventing one. That is deliberately loud: the pinned
                // file would not re-discharge, and the fixture suite fails.
                _ => "(* folded_l folded_r)",
            };
            instance.push_str(&format!(
                "(assert (or (< {expr} {min_s}) (> {expr} {max_s})))\n"
            ));
        }
    }
    instance
}

/// The ground counterpart of the obligation for a constant-folded `-x` overflow.
fn folded_neg_instance(v: i128, ty: IntType) -> String {
    let mut instance = ground_int("folded_operand", &folded_literal(v, ty));
    if ty.is_unsigned() {
        // A checked negation of a non-zero unsigned value wraps, whatever it is.
        instance.push_str("(assert (not (= folded_operand 0)))\n");
    } else {
        let (min_s, _) = ty.bounds_str();
        instance.push_str(&format!("(assert (= folded_operand {min_s}))\n"));
    }
    instance
}

/// Record a FAIL the constant-folding pass reached without asking the solver.
///
/// The symbolic path gives every FAIL two things: a structured `witness` naming
/// the input, and a pinned obligation under `obligations/` that re-discharges to
/// `sat`. A folded failure used to get neither — its operands were spelled out in
/// prose inside `details` and nothing was written to disk — which made the rule the
/// fixture suite asserts ("every FAIL names a concrete input") true only for the
/// obligations Z3 happened to decide, and left a folded FAIL that no one could
/// re-check.
///
/// `instance` is the ground restatement described above. The header records the
/// verdict as `FAIL (folded)` so a reader of the file can tell which path produced
/// it. No precondition is attached: the ground instance names only the folded
/// operands, while a path condition is a formula over the symbolic state those
/// operands replaced, so asserting both into one file would reference symbols the
/// instance does not declare. Reachability was already decided before this is
/// called (`path_is_reachable`), and an unreachable folded failure is not reported
/// at all.
fn record_folded_failure(
    shared_report: &Arc<Mutex<SharedReport>>,
    crate_name: &str,
    mut event: VerificationEvent,
    witness: Vec<Witness>,
    instance: String,
) -> Vec<(String, String)> {
    event.status = "FAIL".to_string();
    // Decided from the operands themselves, under no symbolic state at all: this
    // is the one class where the counterexample is a pair of literals the compiler
    // already folded, so it needs neither a guard nor an invariant.
    event.defect_class = "constant-folded".to_string();
    // A verdict alone is not a bug report. Name the values in `details` too, in the
    // same shape the symbolic path uses.
    let rendered = witness
        .iter()
        .map(|w| format!("{} = {}", w.input, w.value))
        .collect::<Vec<_>>()
        .join(", ");
    if !rendered.is_empty() {
        event.details.push_str(&format!(" [{rendered}]"));
    }
    event
        .details
        .push_str(" -> Counterexample found by constant folding!");
    // A folded witness is a pair of literals the compiler already evaluated, so
    // it is pinned by construction: there is no model and no choice involved.
    event.witness_kind = if witness.is_empty() {
        String::new()
    } else {
        "pinned".to_string()
    };
    // `event.block` is a `String`, and the symbolic path below prints a `BasicBlock`
    // through `{:?}`, which is already `bb0`. Formatting this one with `{:?}` too
    // would print `("bb0")` and make two FAIL lines for the same kind of event
    // two different shapes for a reader — or a script — to parse.
    eprintln!(
        "[FAIL] {} in {} ({})",
        event.details, event.function, event.block
    );
    let written = write_smt_obligation(
        crate_name,
        &event,
        "FAIL (folded)",
        &witness,
        None,
        &instance,
    )
    .unwrap_or_else(|err| {
        eprintln!("[Keller-Pipeline] {err}");
        std::process::exit(crate::EXIT_SETUP);
    });
    event.witness = witness;
    let mut report_guard = shared_report.lock().unwrap();
    report_guard.events.push(event);
    written
}

/// Is there any input that actually reaches this point?
///
/// A constant-folded failure still has to be reachable: a check inside a branch
/// no input can take is not a bug, and reporting it would be a false positive.
/// The guard is only inspected when the function has one — an unguarded
/// assertion costs no solver call.
fn path_is_reachable<'ctx>(ctx: &'ctx Context, state: &PathState<'ctx>) -> bool {
    if state.condition().is_none() {
        return true;
    }
    let mut probe = Solver::new(ctx);
    state.assert_into(ctx, &mut probe);
    !matches!(probe.check(), SatResult::Unsat)
}

/// Did the obligation *force* this value, or did the model pick it?
///
/// A SAT model assigns a value to every symbol it can, including ones the
/// obligation only bounds. Asking whether the same formula without
/// `symbol = value` is still satisfiable separates the two cases: `unsat` means
/// the value was the only one the obligation allows — a fact about the program —
/// while `sat` means the model made a choice, and a second run could print a
/// different number for the same check.
///
/// The question is asked of the solver that already holds the obligation, inside
/// a `push`/`pop` pair, so it costs one `check()` per value and cannot disturb the
/// formula the obligation file was written from. It takes the *rendered* value
/// rather than the model, because a model stops being the current one the moment
/// the solver is asked anything else.
///
/// An undecided answer counts as *not* pinned: a value the solver will not vouch
/// for is not reported as a fact. The report never comes out empty because of that
/// — the operands of the failure are named unconditionally — but a tight budget
/// can make a witness shorter, which is the direction to fail in.
fn value_is_pinned<'ctx>(ctx: &'ctx Context, solver: &Solver<'ctx>, witness: &Witness) -> bool {
    let term = Int::new_const(ctx, witness.symbol.as_str());
    // `extract_witness` renders a `bool` as `true`/`false`; the formula holds it
    // as the `1`/`0` it is encoded as.
    let Some(value) = Int::from_str(ctx, &witness.value).or_else(|| match witness.value.as_str() {
        "true" => Some(Int::from_i64(ctx, 1)),
        "false" => Some(Int::from_i64(ctx, 0)),
        _ => None,
    }) else {
        return false;
    };
    solver.push();
    solver.assert(&term._eq(&value).not());
    let pinned = matches!(solver.check(), SatResult::Unsat);
    solver.pop(1);
    pinned
}

/// Recover the divisor of a division-by-zero assertion.
///
/// rustc lowers `a / b` to
///
/// ```text
/// _t = Eq(b, 0);
/// assert(_t, false, DivisionByZero(a));
/// ```
///
/// so the assertion's payload is the **numerator**, not the divisor (see
/// `rustc_mir_build`'s `builder/expr/as_rvalue.rs`, which passes `lhs.to_copy()`).
/// Encoding "the numerator is zero" instead of "the divisor is zero" is not a
/// division-by-zero check at all — it reports `a = 0` for `a / b` and stays
/// silent about the actual failing input. The divisor is therefore read back out
/// of the condition the assert tests.
///
/// Falls back to the payload when the condition is not the expected
/// `something == 0`, so an unfamiliar lowering still produces a check (a weaker
/// one, but never a missing one).
fn divisor_of<'tcx>(
    mir: &Body<'tcx>,
    block: BasicBlock,
    cond: &Operand<'tcx>,
    const_map: &ConstMap,
    tcx: TyCtxt<'tcx>,
    numerator: &Operand<'tcx>,
) -> Operand<'tcx> {
    let (Operand::Copy(cond_place) | Operand::Move(cond_place)) = cond else {
        return numerator.clone();
    };
    for statement in &mir.basic_blocks[block].statements {
        let StatementKind::Assign(box (place, Rvalue::BinaryOp(BinOp::Eq, args))) = &statement.kind
        else {
            continue;
        };
        if place != cond_place {
            continue;
        }
        let (left, right) = (&args.0, &args.1);
        if eval_operand(right, mir, const_map, tcx) == Some(0) {
            return left.clone();
        }
        if eval_operand(left, mir, const_map, tcx) == Some(0) {
            return right.clone();
        }
    }
    numerator.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The panic-site classifier, on the strings this toolchain actually produces.
    ///
    /// Every path below was read out of a MIR dump (`-Zunpretty=mir`) of
    /// `tests/panic_sites`, not written from the documentation: the documented
    /// spelling of these is `core::…`, and the compiler spells them `std::…`, which
    /// is exactly how the first version of the rule came to report nothing for a
    /// crate full of unwraps. The negative cases are half the test: `_print` is the
    /// call every `println!` makes, and a *dependency* exporting its own `panic` or
    /// `unwrap` is not the standard library's contract.
    #[test]
    fn the_panic_classifier_matches_what_the_compiler_emits() {
        // `Option::unwrap`, `Result::expect`, and the bodies they inline to.
        for path in [
            "std::option::Option::<u8>::unwrap",
            "std::result::Result::<u8, u8>::expect",
            "std::result::Result::<i32, std::num::ParseIntError>::unwrap",
            "core::panicking::unwrap_failed",
            "core::panicking::expect_failed",
        ] {
            assert_eq!(
                classify_callee("core", path),
                Some(PanicSite::Unwrap),
                "{path}"
            );
        }

        // `panic!`, `unreachable!` and the plain `assert!` this compiler lowers to
        // a `panic` with a constant message.
        for path in [
            "std::rt::panic_fmt",
            "std::rt::panic_nounwind",
            "std::panicking::begin_panic",
            "core::panicking::panic",
            "core::panicking::resume_unwind",
        ] {
            assert_eq!(
                classify_callee("std", path),
                Some(PanicSite::Panic),
                "{path}"
            );
        }

        // `assert_eq!`/`assert_ne!` only: the generic arguments on a function's
        // own path segment have to be skipped for this one to be seen at all.
        assert_eq!(
            classify_callee("core", "core::panicking::assert_failed::<u8, u8>"),
            Some(PanicSite::Assert)
        );

        // Allocation failure ends the process without a panic.
        assert_eq!(
            classify_callee("alloc", "alloc::alloc::handle_alloc_error"),
            Some(PanicSite::Abort)
        );

        // The control, in both directions it could go wrong: an ordinary
        // standard-library call is not a panic site, and the same names in a
        // dependency of the crate under analysis are not the standard library's.
        for path in [
            "std::io::_print",
            "core::fmt::rt::Argument::<'_>::new_display",
            "std::option::Option::<u8>::unwrap_or",
            "std::vec::Vec::<u8>::pop",
        ] {
            assert_eq!(classify_callee("std", path), None, "{path}");
        }
        for path in ["some_dep::panic", "some_dep::option::Option::unwrap"] {
            assert_eq!(classify_callee("some_dep", path), None, "{path}");
        }
    }

    /// Ask Z3 whether a ground instance is satisfiable.
    ///
    /// `Solver::from_string` is the solver's own SMT-LIB parser, so this exercises
    /// the exact string the emitter writes rather than a second model of what it
    /// means — the same reasoning the obligation files themselves rest on.
    fn instance_is_sat(instance: &str) -> bool {
        let ctx = Context::new(&Config::new());
        let solver = Solver::new(&ctx);
        solver.from_string(instance);
        matches!(solver.check(), SatResult::Sat)
    }

    /// The folded path is a second implementation of the verdicts the symbolic
    /// encoder reaches, and the two have to agree. This holds the half a unit test
    /// can: for every operand pair the constant path calls a violation, the ground
    /// instance it writes must be satisfiable — and for an ordinary pair it must
    /// not be, which is what makes the instance a statement about the failure
    /// rather than a tautology.
    #[test]
    fn a_folded_overflow_instance_reproduces_the_verdict() {
        let violating = [
            (BinOp::Add, 200i128, 100i128, IntType::U8),
            (BinOp::Sub, 0, 1, IntType::U8),
            (BinOp::Mul, 200, 100, IntType::U8),
            (BinOp::Add, i32::MAX as i128, 1, IntType::I32),
            (BinOp::Sub, i32::MIN as i128, 1, IntType::I32),
            (BinOp::Mul, i64::MAX as i128, 2, IntType::I64),
            (BinOp::Shl, 1, 8, IntType::U8),
            (BinOp::Shl, 1, -1, IntType::I32),
            (BinOp::Div, i32::MIN as i128, -1, IntType::I32),
            (BinOp::Rem, i32::MIN as i128, -1, IntType::I32),
            // The one that does not fit in an `i128`: `u128::MAX + 1` arrives as
            // `(-1) + 1`, and the instance has to be written in the u128 domain.
            (BinOp::Add, u128::MAX as i128, 1, IntType::U128),
        ];
        for (op, l, r, ty) in violating {
            assert!(
                overflow_detected(op, l, r, ty),
                "{op:?} {l} {r} {ty:?} is a violation for the constant path"
            );
            let instance = folded_overflow_instance(op, l, r, ty);
            assert!(
                instance_is_sat(&instance),
                "{op:?} {l} {r} {ty:?} is reported, but its instance is unsatisfiable:\n{instance}"
            );
        }

        let safe = [
            (BinOp::Add, 1i128, 2i128, IntType::U8),
            (BinOp::Mul, 15, 15, IntType::U8),
            (BinOp::Shl, 1, 3, IntType::U8),
            (BinOp::Div, 1, 2, IntType::I32),
        ];
        for (op, l, r, ty) in safe {
            assert!(!overflow_detected(op, l, r, ty));
            let instance = folded_overflow_instance(op, l, r, ty);
            assert!(
                !instance_is_sat(&instance),
                "{op:?} {l} {r} {ty:?} is not a violation, so its instance must not be satisfiable"
            );
        }
    }

    #[test]
    fn a_folded_negation_instance_only_holds_for_the_overflowing_operand() {
        // Signed: only the type's minimum overflows.
        assert!(instance_is_sat(&folded_neg_instance(
            i8::MIN as i128,
            IntType::I8
        )));
        assert!(!instance_is_sat(&folded_neg_instance(5, IntType::I8)));
        // Unsigned: a checked negation of anything non-zero wraps.
        assert!(instance_is_sat(&folded_neg_instance(3, IntType::U8)));
        assert!(!instance_is_sat(&folded_neg_instance(0, IntType::U8)));
    }

    #[test]
    fn a_folded_u128_is_reported_in_its_own_domain() {
        // `u128::MAX` is carried as `-1`. Printing the carrier would name an input
        // the program cannot contain.
        assert_eq!(
            folded_literal(u128::MAX as i128, IntType::U128),
            u128::MAX.to_string()
        );
        assert_eq!(folded_literal(-1, IntType::I32), "-1");
        assert_eq!(folded_literal(-128, IntType::I8), "-128");
    }

    /// A witness entry, as the report stores one.
    fn witness_of(symbol: &str, value: &str) -> Witness {
        Witness {
            input: symbol.to_string(),
            symbol: symbol.to_string(),
            value: value.to_string(),
        }
    }

    #[test]
    fn a_pinned_value_is_a_fact_and_a_bounded_one_is_a_choice() {
        // The report's witness rule in one test. `x` is pinned by the obligation,
        // so its value is a fact about the program; `y` is only bounded to its
        // type, so the model has to choose, and a second run of the same check is
        // free to print a different number. Reporting the two the same way is what
        // makes a witness look like evidence it is not.
        let ctx = Context::new(&Config::new());
        let solver = Solver::new(&ctx);
        let x = Int::new_const(&ctx, "x");
        let y = Int::new_const(&ctx, "y");
        solver.assert(&x._eq(&Int::from_i64(&ctx, 5)));
        solver.assert(&y.ge(&Int::from_i64(&ctx, 0)));
        solver.assert(&y.le(&Int::from_i64(&ctx, 255)));
        assert_eq!(solver.check(), SatResult::Sat);

        assert!(
            value_is_pinned(&ctx, &solver, &witness_of("x", "5")),
            "the only value the obligation allows is a fact"
        );
        assert!(
            !value_is_pinned(&ctx, &solver, &witness_of("x", "6")),
            "and it is the value that is pinned, not the symbol"
        );
        assert!(
            !value_is_pinned(&ctx, &solver, &witness_of("y", "7")),
            "a value inside the type's bounds is the model's choice"
        );
    }

    #[test]
    fn a_relational_failure_pins_neither_operand_which_is_why_they_are_still_named() {
        // `a + b` overflowing is the shape that makes pinnedness insufficient on
        // its own: the formula admits a range of pairs, so neither operand is
        // forced, and a witness that kept only pinned values would be empty for
        // the most common failure there is. That is why the operands the failure
        // is stated over are named unconditionally and only the rest is filtered.
        let ctx = Context::new(&Config::new());
        let solver = Solver::new(&ctx);
        let a = Int::new_const(&ctx, "a");
        let b = Int::new_const(&ctx, "b");
        let max = Int::from_i64(&ctx, 255);
        let zero = Int::from_i64(&ctx, 0);
        for term in [&a, &b] {
            solver.assert(&term.ge(&zero));
            solver.assert(&term.le(&max));
        }
        solver.assert(&(&a + &b).gt(&max));
        assert_eq!(solver.check(), SatResult::Sat);

        assert!(!value_is_pinned(&ctx, &solver, &witness_of("a", "255")));
        assert!(!value_is_pinned(&ctx, &solver, &witness_of("b", "1")));
    }

    #[test]
    fn a_flag_is_pinned_through_its_rendered_form() {
        // `extract_witness` renders a flag as `true`/`false` while the formula
        // holds the `1`/`0` it is encoded as. Asking about the rendered form has to
        // reach the same symbol, or every flag would come back as a choice.
        let ctx = Context::new(&Config::new());
        let solver = Solver::new(&ctx);
        let flag = Int::new_const(&ctx, "mir_4");
        solver.assert(&flag._eq(&Int::from_i64(&ctx, 1)));
        assert_eq!(solver.check(), SatResult::Sat);

        assert!(value_is_pinned(&ctx, &solver, &witness_of("mir_4", "true")));
        assert!(!value_is_pinned(
            &ctx,
            &solver,
            &witness_of("mir_4", "false")
        ));
    }

    #[test]
    fn a_precision_finding_is_a_labelled_warn_about_the_whole_body() {
        // The shape the merge, the gate, the SARIF log and the summary all read.
        // A lost precision is not an obligation class, so it must not borrow one:
        // `kind` is the token a consumer groups on, and `status` is what the exit
        // code is decided from.
        let note = "Path-sensitive dataflow did not reach a fixpoint".to_string();
        let event = precision_event(
            note.clone(),
            "nested_walk",
            "tests/nested_loop/src/main.rs",
            Some(38),
        );
        assert_eq!(event.status, "WARN", "nothing was proved, so not a verdict");
        assert_eq!(event.kind, "path-insensitive");
        assert_eq!(
            event.block, "body",
            "the loss is the function's, not one block's"
        );
        assert_eq!(event.details, note, "the note is the explanation");
        assert_eq!(event.function, "nested_walk");
        assert_eq!(event.file, "tests/nested_loop/src/main.rs");
        assert_eq!(event.line, Some(38));
        assert!(
            event.witness.is_empty(),
            "there is no input to name: the finding is about the analysis"
        );
        // Stamped per crate when the worker writes its report, like any other
        // event, so a workspace merge can say whose function this is.
        assert!(event.crate_name.is_empty());
    }
}
