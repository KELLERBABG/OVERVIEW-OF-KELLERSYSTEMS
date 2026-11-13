use std::sync::{Arc, Mutex};

use crate::types::{Coverage, FunctionEvidence, SharedReport, VerificationEvent, Witness};

pub const RUN_FILE: &str = "report.run.json";

/// Cargo may run a workspace worker from a member directory.
fn artifact_path(name: &str) -> std::path::PathBuf {
    std::env::var_os("KELLER_REPORT_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(name)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerReport {
    schema_version: u32,
    crate_name: String,
    completed: bool,
    coverage: Coverage,
    findings: Vec<VerificationEvent>,
    compiler: Option<serde_json::Value>,
    evidence: Vec<FunctionEvidence>,
    /// Every obligation this crate emitted, one entry per SMT-LIB file under
    /// `obligations/<crate>/` — the PASS ones included, which `findings`
    /// deliberately leaves out because the merged report is a list of problems to
    /// act on rather than a list of everything that was checked.
    ///
    /// The top-level command turns these into the per-crate `index.json`
    /// manifest, so a reviewer or a dashboard reads one file instead of globbing a
    /// directory. `default` keeps receipts written before the field existed
    /// readable.
    #[serde(default)]
    obligations: Vec<ObligationRecord>,
}

/// One emitted proof obligation, as recorded in `obligations/<crate>/index.json`.
///
/// The obligation files themselves are the evidence; this is the index that makes
/// them addressable — every verdict the run reached, including the proofs, with
/// enough identity to open the right file without listing the directory.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObligationRecord {
    /// Serialised as `crate`, the same vocabulary `report.json` uses.
    #[serde(rename = "crate", default)]
    pub crate_name: String,
    pub function: String,
    pub block: String,
    /// `PASS`, `FAIL` or `WARN` — the same three tokens the report uses.
    pub status: String,
    /// The obligation class (`bounds`, `overflow`, …), which is what a dashboard
    /// groups on.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kind: String,
    /// The verdict string this file's header records, so an index and the artifact
    /// it points at cannot disagree about what was decided.
    pub verdict: String,
    /// The obligation's file name under `obligations/<crate>/`.
    pub artifact: String,
    /// The pinned companion file, when one was written (every FAIL names an
    /// input, and that input is asserted in this file).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pinned_artifact: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

/// Name of the per-crate obligation manifest, written under
/// `obligations/<crate>/`.
///
/// One file instead of a directory glob: a reviewer or a dashboard should be able
/// to read every verdict a run reached — proofs included — from a single
/// artifact, and the emitted proofs are only *addressable* if something lists
/// them.
pub const OBLIGATION_MANIFEST_FILE: &str = "index.json";

/// The verdict string an obligation's header records, derived from the event the
/// same way [`write_smt_obligation`] writes it.
fn obligation_verdict(event: &VerificationEvent) -> String {
    match event.status.as_str() {
        "PASS" => "PASS (unsat)".to_string(),
        // The folded path is told apart in the artifact, so it is told apart here
        // too: a ground instance and a solved goal carry different weight.
        "FAIL" if event.details.contains("constant folding") => "FAIL (folded)".to_string(),
        "FAIL" => "FAIL (sat)".to_string(),
        // `-> Undecided` is the encoder's own wording for "a value this analysis
        // does not model is in the violation"; anything else that reached WARN was
        // a solver that did not answer.
        _ if event.details.contains("-> Undecided") => "WARN (undecided)".to_string(),
        _ => "WARN (unknown)".to_string(),
    }
}

/// The finding kinds that are not the verdict of a checked operation.
///
/// An unsupported assertion, a lost precision, an unmodelled panic site and a
/// *decided* panic site are findings about the panic/assertion surface rather
/// than verdicts about an assertion: none of them is one of the
/// `supported_assertions`, so counting one as an assertion warning (or failure)
/// would break the receipt's rule that
/// `supported == passed + failed + warnings + without_verdict`. One predicate,
/// read by the counters in `verifier.rs` and by the receipt check in this file,
/// so the two cannot disagree about which kinds those are.
pub fn is_non_assertion_kind(kind: &str) -> bool {
    matches!(
        kind,
        "unsupported-assertion" | "path-insensitive" | "unmodelled-panic" | "decided-panic"
    )
}

/// Whether this event left an obligation file behind.
///
/// The kinds that do not are the non-assertion kinds above: none of them is an
/// obligation, so none has a formula to emit.
fn is_obligation_event(event: &VerificationEvent) -> bool {
    matches!(event.status.as_str(), "PASS" | "FAIL" | "WARN")
        && !event.kind.is_empty()
        && !is_non_assertion_kind(&event.kind)
}

/// The record one event contributes to the manifest, or `None` when it left no
/// file.
fn obligation_record(event: &VerificationEvent) -> Option<ObligationRecord> {
    if !is_obligation_event(event) {
        return None;
    }
    let stem = format!("{}_{}", safe_stem(&event.function), safe_stem(&event.block));
    Some(ObligationRecord {
        crate_name: event.crate_name.clone(),
        function: event.function.clone(),
        block: event.block.clone(),
        status: event.status.clone(),
        kind: event.kind.clone(),
        verdict: obligation_verdict(event),
        artifact: format!("{stem}.smt2"),
        pinned_artifact: if event.witness.is_empty() {
            String::new()
        } else {
            format!("{stem}.witness.smt2")
        },
        file: event.file.clone(),
        line: event.line,
    })
}

/// Write one `index.json` per crate that emitted anything.
///
/// Sorted by function and block and de-duplicated, so two runs of the same crate
/// produce byte-identical manifests — the same property `report.json` has, and for
/// the same reason: the file is meant to be reviewed in a diff.
fn write_obligation_manifests(records: &[ObligationRecord]) -> Result<(), String> {
    if records.is_empty() {
        return Ok(());
    }
    let mut by_crate: std::collections::BTreeMap<&str, Vec<&ObligationRecord>> =
        std::collections::BTreeMap::new();
    for record in records {
        by_crate
            .entry(record.crate_name.as_str())
            .or_default()
            .push(record);
    }
    for (crate_name, mut entries) in by_crate {
        entries.sort();
        entries.dedup();
        let dir = artifact_path(OBLIGATION_DIR).join(safe_stem(crate_name));
        std::fs::create_dir_all(&dir)
            .map_err(|err| format!("Could not create '{}': {err}", dir.display()))?;
        let manifest = serde_json::json!({
            "schema_version": 1,
            "crate": crate_name,
            // The command that re-checks any entry, so the manifest is usable
            // without reading the docs for it.
            "re_discharge": format!("z3 -smt2 {OBLIGATION_DIR}/{}/{}", safe_stem(crate_name), "<artifact>"),
            "obligations": entries,
        });
        write_json(&dir.join(OBLIGATION_MANIFEST_FILE), &manifest)?;
    }
    Ok(())
}

#[derive(Debug, Default, serde::Serialize)]
pub struct RunCoverage {
    pub workers: usize,
    #[serde(flatten)]
    pub totals: Coverage,
    pub compilers: Vec<serde_json::Value>,
    pub function_evidence: Vec<serde_json::Value>,
}

fn write_json<T: serde::Serialize>(path: &std::path::Path, value: &T) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|err| format!("Could not serialise '{}': {err}", path.display()))?;
    std::fs::write(path, json).map_err(|err| format!("Could not write '{}': {err}", path.display()))
}

pub fn start_worker_report(crate_name: &str) -> Result<(), String> {
    if std::env::var_os("KELLER_OUTPUT_WORKER").is_none() {
        return Ok(());
    }
    let path = artifact_path(REPORT_DIR).join(format!(
        "{}-{}.started",
        safe_stem(crate_name),
        std::process::id()
    ));
    write_json(
        &path,
        &WorkerReport {
            schema_version: 1,
            crate_name: crate_name.to_string(),
            completed: false,
            coverage: Coverage::default(),
            findings: Vec::new(),
            compiler: None,
            evidence: Vec::new(),
            obligations: Vec::new(),
        },
    )
}

fn read_workers(
    dir: &std::path::Path,
) -> Result<
    (
        Vec<Vec<VerificationEvent>>,
        RunCoverage,
        Vec<ObligationRecord>,
    ),
    String,
> {
    let entries = std::fs::read_dir(dir)
        .map_err(|err| format!("Could not read '{}': {err}", dir.display()))?;
    let mut reports = Vec::new();
    let mut coverage = RunCoverage::default();
    let mut obligations = Vec::new();
    let mut starts = std::collections::BTreeSet::new();
    let mut ends = std::collections::BTreeSet::new();
    for entry in entries {
        let path = entry
            .map_err(|err| format!("Could not list worker receipt: {err}"))?
            .path();
        let ext = path.extension().and_then(|ext| ext.to_str());
        if !matches!(ext, Some("json" | "started")) {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|err| format!("Could not read '{}': {err}", path.display()))?;
        let receipt: WorkerReport = serde_json::from_str(&text)
            .map_err(|err| format!("Invalid worker receipt '{}': {err}", path.display()))?;
        if receipt.schema_version != 1
            || receipt.crate_name.is_empty()
            || receipt.completed != (ext == Some("json"))
            || receipt.findings.iter().any(|e| {
                e.crate_name != receipt.crate_name || !matches!(e.status.as_str(), "FAIL" | "WARN")
            })
        {
            return Err(format!(
                "Invalid worker receipt contract: '{}'",
                path.display()
            ));
        }
        let key = (path.file_stem().unwrap().to_os_string(), receipt.crate_name);
        if receipt.completed {
            let c = &receipt.coverage;
            if c.reachable_assertions != c.supported_assertions + c.unsupported_assertions
                || c.supported_assertions != c.passed + c.failed + c.warnings + c.without_verdict
                || c.failed
                    != receipt
                        .findings
                        .iter()
                        .filter(|e| e.status == "FAIL" && !is_non_assertion_kind(&e.kind))
                        .count()
                || c.unsupported_assertions
                    != receipt
                        .findings
                        .iter()
                        .filter(|e| e.kind == "unsupported-assertion" && e.status == "WARN")
                        .count()
                || c.warnings
                    != receipt
                        .findings
                        .iter()
                        .filter(|e| e.status == "WARN" && !is_non_assertion_kind(&e.kind))
                        .count()
                // A decided panic leaves a FAIL event but no assertion failure;
                // the counter has to at least cover the decided panics the
                // receipt actually recorded (safe decided sites leave no event).
                || c.panic_sites_decided
                    < receipt
                        .findings
                        .iter()
                        .filter(|e| e.kind == "decided-panic" && e.status == "FAIL")
                        .count()
            {
                return Err(format!("Inconsistent coverage in '{}'", path.display()));
            }
            ends.insert(key.clone());
            coverage.workers += 1;
            coverage.totals.add(c);
            if let Some(compiler) = receipt.compiler {
                if !coverage.compilers.contains(&compiler) {
                    coverage.compilers.push(compiler);
                }
            }
            for evidence in receipt.evidence {
                coverage.function_evidence.push(serde_json::json!({
                    "crate": key.1,
                    "function": evidence.function,
                    "classification": "model-relative",
                    "native_reproduction": "not-attempted",
                    "disclosures": evidence.disclosures,
                    "provenance_precision": "conservative function-level inventory",
                }));
            }
            reports.push(receipt.findings);
            // Stamped from the receipt's own crate name, like the findings, so a
            // manifest cannot claim a crate the worker did not name.
            for mut record in receipt.obligations {
                record.crate_name = key.1.clone();
                obligations.push(record);
            }
        } else {
            starts.insert(key);
        }
    }
    if starts.is_empty() || starts != ends {
        return Err(
            "Incomplete analysis: no completed workers, or start/completion receipts disagree."
                .to_string(),
        );
    }
    coverage
        .compilers
        .sort_by_key(|compiler| compiler.to_string());
    Ok((reports, coverage, obligations))
}

/// A value the helper can print and the report can carry.
pub type EnvValue = serde_json::Value;

/// Name of the merged report file written to the working directory by the
/// top-level `keller verify` command.
pub const REPORT_FILE: &str = "report.json";

/// File beside `report.json` recording the toolchain and the solver this run was
/// decided in.
///
/// Beside the report rather than inside it because these facts are absolute
/// paths: `report.json` is byte-compared against the committed goldens in
/// `tests/goldens/`, so a machine-specific path in it would turn every golden
/// difference into noise and fail the suite on any other machine. The verdict
/// stays attributable without making the findings file unportable.
pub const ENV_FILE: &str = "report.environment.json";

/// Directory holding one report file per compiled crate.
///
/// Every rustc invocation is a separate process and several run in parallel.
/// They used to write `report.json` directly, so a multi-crate workspace kept
/// only whichever crate happened to finish last. Workers now write here and the
/// top-level command merges the lot.
pub const REPORT_DIR: &str = ".keller-reports";

/// Directory (relative to the working directory) holding emitted SMT-LIB proof
/// obligations.
pub const OBLIGATION_DIR: &str = "obligations";

/// Name this tool reports itself by: in `--version`, in the `--json` summary and
/// in the SARIF driver.
pub const TOOL_NAME: &str = "keller-verify";

/// The build profile every verdict describes (A5).
///
/// The assertions this tool reads — overflow, bounds, division by zero — are the
/// ones rustc emits for a debug build. A release build *wraps* instead of
/// panicking and omits most of them, so "this cannot overflow" and "this will not
/// abort" are different claims, and a report that does not say which one it is
/// making is a report a reader can quote for the wrong artifact.
///
/// One constant, used by the run record, the `--json` summary, the SARIF log and
/// the console line, so the four cannot disagree about what was checked.
pub const ANALYSIS_PROFILE: &str = "debug (dev profile, rustc overflow/bounds assertions enabled)";

/// What [`ANALYSIS_PROFILE`] does not cover, in the terms a reader would act on.
pub const ANALYSIS_PROFILE_NOTE: &str =
    "A release build wraps instead of panicking, so a PASS here is not a guarantee about a shipped release artifact.";

/// Return the analysis profile description depending on whether `--release` was selected.
pub fn analysis_profile(release: bool) -> &'static str {
    if release {
        "release (release profile, cargo --release)"
    } else {
        ANALYSIS_PROFILE
    }
}

/// Return the analysis profile note depending on whether `--release` was selected.
pub fn analysis_profile_note(release: bool) -> &'static str {
    if release {
        "Rust release mode wraps on overflow by default; checks are active where rustc overflow/bounds assertions were explicitly preserved."
    } else {
        ANALYSIS_PROFILE_NOTE
    }
}

/// The environment this run was decided in, in the same keys `keller-env`
/// prints, so the report and the helper use one vocabulary.
///
/// Recorded in [`ENV_FILE`], beside `report.json`, so a published verdict is
/// attributable to the toolchain and solver that produced it, not only to the
/// tool version the `--json` summary already carries. The value is
/// `serde_json::Value` rather than a struct so an absent fact is an absent field
/// rather than a placeholder default.
pub fn run_environment(
    env_source: &str,
    rustc: &str,
    host: &str,
    rustlib: &str,
    z3_bin: &str,
    z3_source: &str,
    path_add: &str,
) -> EnvValue {
    serde_json::json!({
        "env_source": env_source,
        "rustc": rustc,
        "host": host,
        "rustlib": rustlib,
        "z3_bin": z3_bin,
        "z3_source": z3_source,
        "path_add": path_add,
    })
}

/// Write the run's environment to `path` - in practice [`ENV_FILE`], beside the
/// report.
///
/// Required evidence failures propagate as setup errors; an incomplete artifact
/// set must not be reported as a successfully recorded run.
fn write_report_environment(path: &std::path::Path, env: &EnvValue) -> Result<(), String> {
    write_json(path, env)
}

/// Version of this build, taken from `Cargo.toml` rather than hardcoded.
///
/// `1.0.0` is the version the `v1.0.0` git tag meant, and what it means is the
/// promise in [docs/RELEASE.md](../docs/RELEASE.md): a source build on the pinned
/// nightly, the five assertion kinds that document names, and no claim outside
/// them. Aligning the two removed an objection that was visible in the first five
/// minutes of reading the repository — a package claiming `0.1.0` beside a release
/// tagged `v1.0.0`.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The per-obligation solver budget applied when the caller names none.
///
/// A budget is not only about speed, it is the difference between a verdict and a
/// hang: Z3's nonlinear *integer* arithmetic is not a decision procedure for the
/// shapes this encoder emits, and an unbounded obligation can simply never return.
/// With no default, the documented way to get a verdict on such a shape was to
/// know to pass `--solver-timeout`, which means the default path could wedge a CI
/// job — measured on `tests/undecided`, which had not returned after 16 minutes
/// and finishes in about 11 s with a 10 s budget.
///
/// `--solver-timeout 0` still means "no limit" for a caller who says so on
/// purpose; the default only replaces *silence*.
pub const DEFAULT_SOLVER_TIMEOUT_MS: u64 = 30_000;

/// The wall-clock cap on one analysis phase, in seconds.
///
/// This is not a per-obligation budget: it bounds the whole `cargo check` phase, so
/// it is the backstop for the failure a solver budget cannot reach -- a run that
/// stops making progress at all. Measured here, twice: `op-gate` wedged past 20 and
/// 33 minutes with `--solver-timeout 5000` set, and `tests/undecided` with no solver
/// budget at all ran without returning. (Corrected later: the 0.03 s readings were on
/// the parents, which wait on their child; the wrapper hosting Z3 is CPU-bound, so
/// the no-budget case is an unbounded solver call. `op-gate` also wedges on the default budget, so a budget
/// does not bound every wedge.)
///
/// The default is `0`, i.e. disabled. A cap guessed for every project would have to be
/// above the slowest honest run measured here (a cold crate took 2-6 minutes) and below
/// the point where a CI runner is assumed dead, and a number that is wrong for a large
/// crate turns a real analysis into a stopped run with no verdict. So the cap is a
/// per-project decision and `--run-timeout 0` states the default explicitly. A former
/// default of 1800 s was removed on 2026-11-04 for that reason.
pub const DEFAULT_RUN_TIMEOUT_SECONDS: u64 = 0;

/// Whether the per-assertion progress lines should stay quiet.
///
/// Only the lines that report a *pass* are silenced. A failing verdict is never
/// suppressed: a gate that hides its findings is worse than a noisy one, and
/// `--quiet` is for CI logs, not for people reading a failure.
pub fn quiet() -> bool {
    std::env::var("KELLER_QUIET")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// The flags `keller verify` was invoked with, in the form the reporting layer
/// needs them.
///
/// One struct rather than half a dozen parameters, because the merge, the gate,
/// the SARIF log and the summary all need to agree about what was asked for.
/// A checked-in per-crate gate policy.
///
/// The gap (B8 / K19) is that a repository cannot say "these crates are
/// gated, these are advisory" in a file it commits, so every run repeats the
/// decision on the command line instead. A `[[policy]]` entry in `keller.toml`
/// names a crate and whether its findings are gated.
///
/// `advisory` means the crate's findings are still reported — they are in
/// `report.json`, on the console, and in SARIF — but they are not what fails the
/// build. That is the same shape as a baseline, scoped to a crate, and it is
/// deliberately *not* a way to hide a counterexample: the count of what was
/// excluded is printed and recorded, so a reader can see what the policy cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    pub crate_name: String,
    pub path: Option<String>,
    pub advisory: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VerifyOptions {
    /// Treat a `WARN` — an undecided verdict, or a lost path sensitivity — as a
    /// failure.
    pub deny_warn: bool,
    /// Coverage gaps cannot be hidden by a baseline.
    pub strict_coverage: bool,
    /// Restrict the run to findings whose `crate::function` contains this text.
    ///
    /// Findings outside the filter are not reported *and not gated*, which is why
    /// the summary's `scope` says `partial` whenever it is set: a narrowed run must
    /// never be readable as a full one.
    pub only: Option<String>,
    /// Cap how many findings are listed (console, SARIF and the `--json` summary).
    ///
    /// The gate is deliberately not affected: it still judges every finding, so
    /// this can make a large first run reviewable but can never turn a
    /// counterexample into a passing build.
    pub max_findings: Option<usize>,
    /// The named gate profile (`advisory` or `strict`) this run was asked for, if
    /// any. Recorded so a verdict carries the contract it was produced under.
    pub gate_profile: Option<String>,
    pub no_cache: bool,
    pub watch: bool,
    pub keep_runs: Option<usize>,
    pub build_elapsed_ms: Option<u64>,
    pub run_id: Option<String>,
    pub run_directory: Option<std::path::PathBuf>,
    /// Suppress the pass lines (the worker side is told through the environment).
    pub quiet: bool,
    /// Print the summary as JSON on stdout.
    pub json: bool,
    /// Also write a SARIF 2.1.0 log here.
    pub sarif: Option<std::path::PathBuf>,
    /// Write an interactive HTML report here.
    pub html: Option<std::path::PathBuf>,
    /// Compiler diagnostic format: "human" or "json".
    pub error_format: Option<String>,
    /// Findings listed here are reported but not gated.
    pub baseline: Option<std::path::PathBuf>,
    /// Record the current findings as a baseline here.
    pub write_baseline: Option<std::path::PathBuf>,
    /// Per-obligation solver budget in milliseconds. `Some(0)` is what
    /// `--solver-timeout 0` means: no limit. `None` means the caller said nothing,
    /// and [`DEFAULT_SOLVER_TIMEOUT_MS`] applies.
    pub solver_timeout_ms: Option<u64>,
    /// Wall-clock cap on the whole analysis phase, in seconds. `Some(0)` is what
    /// `--run-timeout 0` means: no cap. `None` means the caller said nothing, and
    /// [`DEFAULT_RUN_TIMEOUT_SECONDS`] applies.
    pub run_timeout_seconds: Option<u64>,
    /// The environment this run was decided in. Populated by the top-level command
    /// from the shared discovery and written beside `report.json` when present,
    /// so a verdict is attributable to the toolchain and solver that decided it.
    pub env: Option<EnvValue>,
    /// Per-crate gate policy read from `keller.toml` (B8 / K19). Empty means every
    /// crate is gated, which is the behaviour without a config file.
    pub policies: Vec<Policy>,
    /// Whether to verify third-party dependencies as well as workspace members.
    /// By default (false), only primary/workspace crates are verified (`CARGO_PRIMARY_PACKAGE`),
    /// avoiding redundant analysis of external crates (K4).
    pub verify_deps: bool,
    /// Whether to run verification against a release build (`--release`, A5).
    pub release: bool,
    /// Selected SMT solver backend: "z3", "cvc5", "bitwuzla", or "portfolio"
    pub solver: Option<String>,
}

/// One finding's identity, for the baseline file.
///
/// Deliberately *not* the whole event: a baseline must survive a message
/// rewording, a witness changing, or the `kind` token being renamed, while still
/// matching the same obligation. `crate`/`function`/`block`/`status` is what
/// identifies an obligation; everything else describes it.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct FindingKey {
    #[serde(rename = "crate", default)]
    pub crate_name: String,
    pub function: String,
    pub block: String,
    pub status: String,
}

impl FindingKey {
    /// The identity of an event.
    pub fn of(event: &VerificationEvent) -> Self {
        FindingKey {
            crate_name: event.crate_name.clone(),
            function: event.function.clone(),
            block: event.block.clone(),
            status: event.status.clone(),
        }
    }
}

/// On-disk shape of a baseline file. Versioned, so a future change can refuse to
/// read a file it does not understand instead of silently ignoring every entry.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BaselineFile {
    pub keller_baseline: u32,
    pub findings: Vec<FindingKey>,
}

/// The baseline format this build writes.
pub const BASELINE_VERSION: u32 = 1;

/// Read a baseline file.
///
/// Errors are returned as text rather than as a panic, because a mistyped
/// `--baseline` path must not abort a run halfway: the caller turns this into the
/// usage-error exit code, so the build fails for a reason it can explain.
pub fn load_baseline(path: &std::path::Path) -> Result<Vec<FindingKey>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("Could not read baseline '{}': {err}", path.display()))?;
    let parsed: BaselineFile = serde_json::from_str(&text).map_err(|err| {
        format!(
            "Baseline '{}' is not a keller baseline: {err}",
            path.display()
        )
    })?;
    if parsed.keller_baseline != BASELINE_VERSION {
        return Err(format!(
            "Baseline '{}' is version {}, this build writes version {}.",
            path.display(),
            parsed.keller_baseline,
            BASELINE_VERSION
        ));
    }
    Ok(parsed.findings)
}

/// Write the current findings as a baseline. Returns how many were recorded.
pub fn write_baseline(
    path: &std::path::Path,
    events: &[VerificationEvent],
) -> Result<usize, String> {
    let mut findings: Vec<FindingKey> = events.iter().map(FindingKey::of).collect();
    // Sorted and de-duplicated so re-recording an unchanged baseline produces
    // identical bytes — the same property `report.json` has, and for the same
    // reason: the file is meant to be reviewed in a diff.
    findings.sort();
    findings.dedup();
    let file = BaselineFile {
        keller_baseline: BASELINE_VERSION,
        findings: findings.clone(),
    };
    let json = serde_json::to_string_pretty(&file)
        .map_err(|err| format!("Could not serialise baseline: {err}"))?;
    std::fs::write(path, json)
        .map_err(|err| format!("Could not write baseline '{}': {err}", path.display()))?;
    Ok(findings.len())
}

/// Split findings into the ones the gate still sees and the number the baseline
/// covered.
///
/// Matching is by [`FindingKey`], so a finding whose witness or message changed is
/// still recognised as the same obligation — otherwise every re-run after an
/// unrelated edit would look like a new failure, and the baseline would be
/// useless in exactly the situation it exists for.
pub fn suppress(
    events: &[VerificationEvent],
    baseline: &[FindingKey],
) -> (Vec<VerificationEvent>, usize) {
    let known: std::collections::HashSet<&FindingKey> = baseline.iter().collect();
    let mut suppressed = 0;
    let kept: Vec<VerificationEvent> = events
        .iter()
        .filter(|event| {
            let key = FindingKey::of(event);
            if known.contains(&key) {
                suppressed += 1;
                false
            } else {
                true
            }
        })
        .cloned()
        .collect();
    (kept, suppressed)
}

/// The rule id a finding is filed under in SARIF, and in any dashboard that
/// groups by class.
fn rule_id(kind: &str, defect_class: &str) -> String {
    let kind = if kind.is_empty() { "unknown" } else { kind };
    // A failure's class is part of its rule so a dashboard can route or suppress
    // "may be false positives" separately from "bugs", which is the whole point of
    // classifying them (K6). Anything that is not a FAIL has no class
    // and so keeps the rule it had before the field existed.
    if defect_class.is_empty() {
        format!("keller/{kind}")
    } else {
        format!("keller/{kind}/{defect_class}")
    }
}

/// One-line description of a defect class, for the SARIF rule catalogue.
///
/// The class is *why* the finding is a finding, so its catalogue entry says what
/// stands between the verdict and the program — not what can go wrong, which is
/// the obligation kind's line.
fn defect_class_description(defect_class: &str) -> &'static str {
    match defect_class {
        "guard-limited" => {
            "Stated under the guards that lead to it and satisfied by the model: a stronger guard on the path is what would settle it."
        }
        "widening-limited" => {
            "Leans on a loop-carried value the abstraction generalised, so it may be what the loop widening cost rather than a bug."
        }
        "unguarded" => "Reached with no path condition at all, so no guard is missing.",
        "constant-folded" => {
            "Decided from constant operands, with no symbolic state involved."
        }
        "known-panic" => {
            "The receiver's enum variant is known and is the one the callee cannot return, so the panic is settled without a solver model."
        }
        _ => "A failure keller-verify could not classify.",
    }
}

/// One-line description of a class, for the SARIF rule catalogue.
fn rule_description(kind: &str) -> &'static str {
    match kind {
        "bounds" => "A slice, array or string index can leave the bounds of the value.",
        "overflow" => "An arithmetic operation can overflow its integer type.",
        "overflow-neg" => "Negating an integer can overflow (the signed minimum has no positive counterpart).",
        "divzero" => "An integer division can divide by zero.",
        "remainder" => "An integer remainder can divide by zero.",
        "unsupported-assertion" => "A reachable MIR assertion is unsupported; no proof was attempted.",
        // Not an obligation class: the program can panic at this call and the
        // analysis does not read the callee, so nothing was proved either way.
        "unmodelled-panic" => {
            "A call that can panic is not modelled; whether this site can panic was not decided."
        }
        // Not an assertion class either: the call's own receiver is known, so the
        // panic is settled rather than disclosed.
        "decided-panic" => {
            "A call is reached with the enum variant it cannot return, so it panics."
        }
        // Not an obligation class: the function's dataflow could not reach a
        // fixpoint, so its guards were dropped and a FAIL in it may be a false
        // positive.
        "path-insensitive" => {
            "The function's dataflow fell back to the path-insensitive state, so a FAIL in it may be a false positive."
        }
        _ => "An obligation keller-verify could not classify.",
    }
}

/// Write the findings as a SARIF 2.1.0 log.
///
/// This is the format GitHub code scanning, and most other review platforms,
/// accept, so a finding can be annotated on the pull request that introduced it
/// instead of only appearing in a CI log. Physical locations are included when
/// the compiler gave a usable span; when it did not, only the logical location
/// (the function) is written — a SARIF result with a wrong file is worse than one
/// with no file.
///
/// `events` are the findings the gate judged, so a baselined finding is not
/// annotated as if it were new.
///
/// `coverage` is the run's own account of what it did *not* check. K8's second
/// meaning is that the unsupported fraction has to be visible wherever the
/// findings are, not only on the console and in `--json`: a code-scanning reader
/// that sees three annotations and no coverage would read a partial run as a
/// complete one. The whole headline goes in the run's `properties`, next to the
/// analysis profile, so it travels with the results instead of needing the
/// console log kept beside them.
pub fn write_sarif(
    path: &std::path::Path,
    events: &[VerificationEvent],
    coverage: &Coverage,
) -> Result<(), String> {
    let mut rules_by_class: Vec<(&str, &str)> = events
        .iter()
        .map(|event| (event.kind.as_str(), event.defect_class.as_str()))
        .collect();
    rules_by_class.sort_unstable();
    rules_by_class.dedup();
    let rules: Vec<serde_json::Value> = rules_by_class
        .iter()
        .map(|(kind, defect_class)| {
            let id = rule_id(kind, defect_class);
            let mut rule = serde_json::json!({
                "id": id,
                "name": id.replace('/', "_"),
                "shortDescription": { "text": rule_description(kind) },
            });
            if !defect_class.is_empty() {
                rule["fullDescription"] =
                    serde_json::json!({ "text": defect_class_description(defect_class) });
            }
            rule
        })
        .collect();

    let results: Vec<serde_json::Value> = events
        .iter()
        .map(|event| {
            let mut location = serde_json::Map::new();
            if !event.file.is_empty() {
                let mut physical = serde_json::json!({
                    "artifactLocation": { "uri": event.file },
                });
                if let Some(line) = event.line {
                    physical["region"] = serde_json::json!({ "startLine": line });
                }
                location.insert("physicalLocation".to_string(), physical);
            }
            location.insert(
                "logicalLocations".to_string(),
                serde_json::json!([{
                    "fullyQualifiedName": event.function,
                    "kind": "function",
                }]),
            );
            serde_json::json!({
                "ruleId": rule_id(&event.kind, &event.defect_class),
                "level": if event.status == "FAIL" { "error" } else { "warning" },
                "message": { "text": event.details },
                "locations": [serde_json::Value::Object(location)],
                "partialFingerprints": {
                    "keller/v1": format!("{}::{}::{}", event.crate_name, event.function, event.block),
                },
                "properties": {
                    "crate": event.crate_name,
                    "function": event.function,
                    "block": event.block,
                    "status": event.status,
                    "kind": event.kind,
                    "defectClass": event.defect_class,
                    "witness": event.witness,
                },
            })
        })
        .collect();

    let log = serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": TOOL_NAME,
                    "version": version(),
                    "informationUri": "https://github.com/",
                    "rules": rules,
                }
            },
            // A5: an annotation that appears in a pull request has to say what
            // was checked, or a reviewer reads a debug-build proof as a statement
            // about the release binary.
            "properties": {
                "analysis_profile": ANALYSIS_PROFILE,
                "analysis_profile_note": ANALYSIS_PROFILE_NOTE,
                // The same object `--json` prints under `coverage_summary`, so a
                // consumer of either artifact reads the same numbers: encountered,
                // decided, and the two reasons the rest was not decided —
                // unsupported kinds this build does not model and obligations the
                // solver left undecided.
                "coverage": coverage_headline(coverage),
            },
            "results": results,
        }],
    });

    let text = serde_json::to_string_pretty(&log)
        .map_err(|err| format!("Could not serialise SARIF: {err}"))?;
    std::fs::write(path, text)
        .map_err(|err| format!("Could not write SARIF log '{}': {err}", path.display()))?;
    Ok(())
}

/// Split a Z3 solver dump into complete top-level s-expressions.
///
/// The dump cannot be filtered line by line: Z3 pretty-prints a long command
/// across several lines (a wide `assert` is wrapped), so a line filter would
/// truncate it into a syntax error. This tracks parenthesis depth — skipping
/// `"`-strings, `|`-quoted symbols and `;` comments, which are only recognised
/// outside a form — and yields each balanced top-level form intact.
fn split_top_level_forms(text: &str) -> Vec<String> {
    let mut forms: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut depth: usize = 0;
    let mut in_string = false;
    let mut in_quoted = false;

    for ch in text.chars() {
        if in_string {
            current.push(ch);
            if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if in_quoted {
            current.push(ch);
            if ch == '|' {
                in_quoted = false;
            }
            continue;
        }
        match ch {
            '"' if depth > 0 => {
                current.push(ch);
                in_string = true;
            }
            '|' if depth > 0 => {
                current.push(ch);
                in_quoted = true;
            }
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                if depth > 0 {
                    depth -= 1;
                    current.push(ch);
                    if depth == 0 {
                        forms.push(current.trim().to_string());
                        current.clear();
                    }
                }
            }
            // Outside a form every other character (including `;` comments) is
            // noise between commands and is dropped.
            _ => {
                if depth > 0 {
                    current.push(ch);
                }
            }
        }
    }

    forms
}

/// Turn a def path / block label into a filesystem-safe file stem.
fn safe_stem(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.len() > 120 {
        s.truncate(120);
    }
    s
}

/// Emit the exact formula Z3 was asked about as SMT-LIB, so a verdict can be
/// re-checked independently instead of resting on this binary's word:
///
/// ```text
/// z3 -smt2 obligations/add/add_bb0.smt2
/// ```
///
/// `solver_smt2` comes from the solver's own `Display` impl, so it is the
/// declarations and assertions that were actually solved rather than a second
/// implementation that could drift from it. Only FAIL/WARN obligations are
/// written (a bound is all an independent re-check needs), and emission can be
/// turned off with `KELLER_EMIT_SMT2=0`.
///
/// When a SAT model was extracted, the witness is (a) recorded as comments and
/// (b) written as a second, *pinned* obligation — the same declarations and
/// assertions with the reported inputs asserted as equalities. Re-running that
/// file must answer `sat`; if the extracted values were wrong (a truncated 128-bit
/// value, a mangled sign) it answers `unsat`, so the witness itself is checkable
/// rather than merely printed.
pub fn write_smt_obligation(
    crate_name: &str,
    event: &VerificationEvent,
    verdict: &str,
    witness: &[Witness],
    precondition: Option<&str>,
    solver_smt2: &str,
) -> Result<Vec<(String, String)>, String> {
    if std::env::var("KELLER_EMIT_SMT2")
        .map(|v| v == "0")
        .unwrap_or(false)
    {
        return Ok(Vec::new());
    }

    // One directory per crate. Two crates can define the same function at the
    // same block, and a flat directory would let them overwrite each other — the
    // same clobbering the per-crate reports exist to fix.
    let dir = artifact_path(OBLIGATION_DIR).join(safe_stem(crate_name));
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("Could not create '{}': {err}", dir.display()))?;

    // The header has to answer a reviewer's first three questions without them
    // opening another file: *what* was checked (`kind`), *where* it is (`file` and
    // `line`), and what the report says about it (`details`, which is the same
    // sentence the console and SARIF carry). Anything else means the artifact and
    // the report disagree about how to describe one site.
    let mut header = String::new();
    header.push_str("; keller-verify proof obligation\n");
    header.push_str(&format!("; crate:    {}\n", crate_name));
    header.push_str(&format!("; function: {}\n", event.function));
    header.push_str(&format!("; block:    {}\n", event.block));
    if !event.kind.is_empty() {
        header.push_str(&format!("; kind:     {}\n", event.kind));
    }
    if !event.file.is_empty() {
        let line = event
            .line
            .map(|line| format!(":{line}"))
            .unwrap_or_default();
        header.push_str(&format!("; file:     {}{}\n", event.file, line));
    }
    if !event.details.is_empty() {
        header.push_str(&format!(
            "; details:  {}\n",
            first_line(&event.details, 300)
        ));
    }
    header.push_str(&format!("; verdict:  {}\n", verdict));
    // The guards on the paths that reach this assertion. Without them the
    // violation below would read as "this can happen", when the whole point is
    // that it can happen *only* within this condition.
    if let Some(precondition) = precondition {
        header.push_str(&format!("; reachable: {}\n", first_line(precondition, 300)));
    }
    for w in witness {
        header.push_str(&format!(
            "; witness:  {} = {} ({})\n",
            w.symbol, w.value, w.input
        ));
    }

    // `Solver`'s `Display` dumps the whole solver state, not just the goal: it
    // includes internal `model-add` / `model-del` / `pb2bv-model-converter`
    // forms that a fresh solver rejects as `unsupported`. Keeping only the
    // declarations and assertions (split at top level, so wrapped commands
    // survive intact) leaves a portable SMT-LIB file.
    let mut goal = String::new();
    for form in split_top_level_forms(solver_smt2) {
        let name = form
            .trim_start_matches('(')
            .split_whitespace()
            .next()
            .unwrap_or("");
        if name == "declare-fun" || name == "assert" {
            goal.push_str(&form);
            goal.push('\n');
        }
    }

    let stem = format!("{}_{}", safe_stem(&event.function), safe_stem(&event.block));
    let artifact = format!("{}.smt2", stem);
    let path = dir.join(&artifact);
    let contents = format!("{}{}(check-sat)\n", header, goal);
    std::fs::write(&path, &contents)
        .map_err(|err| format!("Could not write '{}': {err}", path.display()))?;
    let mut written = vec![(artifact, contents)];

    // Pin the extracted witness: assert each reported input and re-ask. This is
    // the file that proves "the failure is this input" rather than "some input".
    if !witness.is_empty() {
        let mut pinned = header.clone();
        pinned.push_str("; pinned: the witness above, asserted; re-checking must answer `sat`\n");
        pinned.push_str(&goal);
        for w in witness {
            pinned.push_str(&format!(
                "(assert (= {} {}))\n",
                w.symbol,
                pinned_literal(&w.value)
            ));
        }
        // Make the witness checkable against the path condition too: if the report
        // only exists because the assertion is unreachable, asserting the witness
        // *and* the precondition must be unsatisfiable. Corrupted witnesses still
        // answer `sat` under (witness + precondition), so this is additive, not a
        // replacement for that check.
        if let Some(ref precondition_string) = precondition {
            // Emit the precondition as proper SMT-LIB forms with balanced parens.
            // The precondition string can be a long Z3 term that the pretty-printer
            // writes across lines; splitting at top level keeps each form intact.
            let forms: Vec<String> = split_top_level_forms(precondition_string);
            for form in forms {
                pinned.push_str(&format!("(assert {})\n", form));
            }
        }
        pinned.push_str("(check-sat)\n");
        let artifact = format!("{}.witness.smt2", stem);
        let path = dir.join(&artifact);
        std::fs::write(&path, &pinned)
            .map_err(|err| format!("Could not write '{}': {err}", path.display()))?;
        written.push((artifact, pinned));
    }
    Ok(written)
}

/// Whether this run is expected to emit obligation files.
///
/// The same switch `write_smt_obligation` and the manifest writer read, exposed
/// so a cache hit can tell "this function had no obligations" from "the entry was
/// written by a run that suppressed them".
pub fn obligations_enabled() -> bool {
    !std::env::var("KELLER_EMIT_SMT2")
        .map(|value| value == "0")
        .unwrap_or(false)
}

/// How many SMT-LIB files the given events should have produced.
///
/// One per obligation event, plus the pinned companion whenever the event named
/// a witness. A cache hit checks this against the artifacts the entry carries:
/// restoring fewer would leave `index.json` indexing a file this run never wrote.
pub fn expected_obligation_artifacts(events: &[VerificationEvent]) -> usize {
    events
        .iter()
        .filter(|event| is_obligation_event(event))
        .map(|event| usize::from(event.witness.is_empty()) + 1)
        .sum()
}

/// Republish the artifacts recorded on a cache hit.
///
/// `prepare_report_dir` deletes `obligations/` before every run, so a hit that
/// restores only the event list produces a manifest pointing at files that do not
/// exist. Writing the cached bytes back keeps the evidence for a cached verdict
/// identical to the first run's, instead of making the cache a weaker claim than
/// a fresh analysis.
pub fn restore_smt_obligations(
    crate_name: &str,
    artifacts: &[(String, String)],
) -> Result<(), String> {
    if artifacts.is_empty() || !obligations_enabled() {
        return Ok(());
    }
    let dir = artifact_path(OBLIGATION_DIR).join(safe_stem(crate_name));
    std::fs::create_dir_all(&dir)
        .map_err(|err| format!("Could not create '{}': {err}", dir.display()))?;
    for (artifact, contents) in artifacts {
        let path = dir.join(artifact);
        std::fs::write(&path, contents)
            .map_err(|err| format!("Could not write '{}': {err}", path.display()))?;
    }
    Ok(())
}

pub fn obligation_expected(text: &str) -> Result<&'static str, String> {
    if !text.starts_with("; keller-verify proof obligation") {
        return Err("Not a Keller obligation: missing header.".to_string());
    }
    let verdict = text
        .lines()
        .find_map(|line| line.strip_prefix("; verdict:").map(str::trim))
        .ok_or_else(|| "Missing recorded obligation verdict.".to_string())?;
    match verdict {
        "FAIL (sat)" | "FAIL (folded)" => Ok("sat"),
        "PASS (unsat)" => Ok("unsat"),
        "WARN (unknown)" => Ok("unknown"),
        _ => Err(format!("Unsupported recorded verdict: {verdict}")),
    }
}

pub fn check_replay(expected: &str, output: &str) -> Result<(), String> {
    let answer = output.trim();
    if answer == expected && matches!(expected, "sat" | "unsat") {
        Ok(())
    } else {
        Err(format!(
            "Replay did not match: recorded {expected}, solver returned {answer:?}."
        ))
    }
}

/// Restrict a comment to its first line and `limit` characters.
///
/// A precondition is a Z3 term, and a long one would push the rest of the
/// obligation header off the screen.
fn first_line(text: &str, limit: usize) -> String {
    let line = text.lines().next().unwrap_or("");
    if line.len() <= limit {
        return line.to_string();
    }
    let mut cut = limit;
    while cut > 0 && !line.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}…", &line[..cut])
}

/// Render a witness value as a literal of the symbol's own sort.
///
/// A `bool` input is reported as `true`/`false` for the reader but encoded as
/// `1`/`0` in the formula, so the pinned equality has to translate it back —
/// `(assert (= mir_1 true))` would be a sort error, not a re-check.
fn pinned_literal(value: &str) -> &str {
    match value {
        "true" => "1",
        "false" => "0",
        other => other,
    }
}

/// Write this crate's FAIL/WARN events into its own file under [`REPORT_DIR`].
///
/// The file name carries both the crate name and the process id, so parallel
/// workers — and two targets of the same package — cannot overwrite each other.
/// Only called in worker mode; the top-level command merges afterwards.
pub fn write_worker_report(
    crate_name: &str,
    shared_report: &Arc<Mutex<SharedReport>>,
) -> Result<(), String> {
    if std::env::var_os("KELLER_OUTPUT_WORKER").is_none() {
        return Ok(());
    }
    let report = shared_report.lock().unwrap();
    if !report.analysis_completed {
        return Err(format!(
            "Compiler worker '{crate_name}' did not complete MIR analysis."
        ));
    }
    let mut findings: Vec<VerificationEvent> = report
        .events
        .iter()
        .filter(|e| matches!(e.status.as_str(), "FAIL" | "WARN"))
        .cloned()
        .collect();
    for event in &mut findings {
        event.crate_name = crate_name.to_string();
    }
    let path = artifact_path(REPORT_DIR).join(format!(
        "{}-{}.json",
        safe_stem(crate_name),
        std::process::id()
    ));
    // Every verdict that left a file, proofs included. Emission can be turned off
    // with `KELLER_EMIT_SMT2=0`, and then there are no files to index — writing a
    // manifest that pointed at nothing would be a claim the run cannot support.
    let obligations: Vec<ObligationRecord> = if std::env::var("KELLER_EMIT_SMT2")
        .map(|value| value == "0")
        .unwrap_or(false)
    {
        Vec::new()
    } else {
        let mut records: Vec<ObligationRecord> = report
            .events
            .iter()
            // `filter_map`, not `collect::<Option<_>>`: an event that left no file
            // (an unsupported assertion, a path-insensitive fallback) is the
            // normal case, and skipping it must not discard the rest of the run's
            // obligations. That is what a collected `Option` did — one WARN of a
            // kind this build does not model blanked the manifest for the whole
            // crate, so a run with proofs and one skipped assertion indexed
            // nothing at all.
            .filter_map(|event| {
                let mut record = obligation_record(event)?;
                record.crate_name = crate_name.to_string();
                Some(record)
            })
            .collect();
        records.sort();
        records.dedup();
        records
    };
    write_json(
        &path,
        &WorkerReport {
            schema_version: 1,
            crate_name: crate_name.to_string(),
            completed: true,
            coverage: report.coverage.clone(),
            findings,
            compiler: report.compiler.clone(),
            evidence: report.evidence.clone(),
            obligations,
        },
    )
}

/// Reset everything a run writes, so nothing from an earlier run can decide it.
///
/// Three separate staleness traps, all of which would let a previous run speak
/// for this one: stale per-crate files get merged into the new report, a stale
/// `report.json` from an earlier failing run sits there looking current, and
/// stale SMT-LIB obligations get re-discharged by the test suite as if this run
/// had produced them.
pub fn prepare_report_dir() -> Result<(), String> {
    for name in [REPORT_DIR, OBLIGATION_DIR] {
        match std::fs::remove_dir_all(artifact_path(name)) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(format!("Could not reset '{name}': {err}")),
        }
    }
    for name in [REPORT_FILE, ENV_FILE, RUN_FILE] {
        match std::fs::remove_file(artifact_path(name)) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(format!("Could not reset '{name}': {err}")),
        }
    }
    std::fs::create_dir_all(artifact_path(REPORT_DIR))
        .map_err(|err| format!("Could not create '{REPORT_DIR}': {err}"))
}

/// Combine per-crate event lists into one deterministic, de-duplicated list.
///
/// Parallel workers finish in nondeterministic order and a crate can be compiled
/// more than once, so the merge is sorted and de-duplicated: otherwise
/// `report.json` would differ between identical runs and could not be compared
/// as a golden file. Events are keyed by crate first, so two crates reporting the
/// same function and block stay distinct.
pub fn merge_events(reports: Vec<Vec<VerificationEvent>>) -> Vec<VerificationEvent> {
    fn key(e: &VerificationEvent) -> (&str, &str, &str, &str, &str) {
        (&e.crate_name, &e.function, &e.block, &e.status, &e.details)
    }

    let mut merged: Vec<VerificationEvent> = reports.into_iter().flatten().collect();
    merged.sort_by(|a, b| key(a).cmp(&key(b)));
    merged.dedup();
    merged
}

/// Why the exit-code contract is violated, if it is.
///
/// Kept separate from the exit code so the policy is testable without a
/// compiler, and so the two reasons stay distinguishable in the message: a
/// counterexample is a bug in the verified code, an undecided verdict is a gap
/// in the proof. They need different fixes, so `--deny-warn` must not launder
/// one into the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateBreach {
    /// `FAIL`: the solver exhibited an input that reaches the assertion.
    Counterexample { count: usize },
    /// `WARN` while `--deny-warn` was given: the solver answered `unknown`, or a
    /// function's dataflow lost path sensitivity, so nothing was proved in the
    /// affirmative either way.
    Undecided { count: usize },
    /// Both kinds are present; the counterexample is the more urgent.
    Both {
        counterexamples: usize,
        undecided: usize,
    },
}

impl std::fmt::Display for GateBreach {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GateBreach::Counterexample { count } => {
                write!(formatter, "report.json contains {count} FAIL event(s)")
            }
            // "finding" rather than "obligation": a path-insensitive fallback is
            // recorded against the function as a whole, not against one
            // obligation inside it.
            GateBreach::Undecided { count } => write!(
                formatter,
                "{count} finding(s) are undecided (WARN) and --deny-warn was given"
            ),
            GateBreach::Both {
                counterexamples,
                undecided,
            } => write!(
                formatter,
                "report.json contains {counterexamples} FAIL event(s) and {undecided} undecided finding(s) (WARN)"
            ),
        }
    }
}

/// Decide whether a set of findings should fail the build.
///
/// `FAIL` always does — the solver returned a counterexample. `WARN` means
/// nothing was proved in the affirmative, for either of two reasons: the solver
/// returned `unknown`, or the function's dataflow fell back to the
/// path-insensitive state (`kind` = `path-insensitive`) and its guards went with
/// it. Neither is a verdict, and a gate that treated either as a pass would be
/// claiming more than was checked. Both are tolerated by default and fatal under
/// `--deny-warn`, which is how CI demands a definite answer instead of only
/// catching proven findings.
pub fn gate_breach(events: &[VerificationEvent], deny_warn: bool) -> Option<GateBreach> {
    let counterexamples = events.iter().filter(|e| e.status == "FAIL").count();
    let undecided = if deny_warn {
        events.iter().filter(|e| e.status == "WARN").count()
    } else {
        0
    };

    match (counterexamples, undecided) {
        (0, 0) => None,
        (0, undecided) => Some(GateBreach::Undecided { count: undecided }),
        (counterexamples, 0) => Some(GateBreach::Counterexample {
            count: counterexamples,
        }),
        (counterexamples, undecided) => Some(GateBreach::Both {
            counterexamples,
            undecided,
        }),
    }
}

/// Everything one run produced, for the caller that has to print it, gate on it
/// and possibly annotate a pull request with it.
#[derive(Debug, Default)]
pub struct MergeOutcome {
    /// The breach of the exit-code contract, if any.
    pub breach: Option<GateBreach>,
    /// Every finding merged from the workers; what `report.json` holds.
    pub findings: Vec<VerificationEvent>,
    /// The findings the gate actually judged (baselined ones removed).
    pub gated: Vec<VerificationEvent>,
    /// How many findings the baseline covered.
    pub suppressed: usize,
    /// How many findings `--only` kept out of this run entirely. Counted so a
    /// narrowed run states how much of the crate it did not look at.
    pub filtered_out: usize,
    /// The listing cap this run was given, echoed so the summary and the SARIF log
    /// can apply the same one.
    pub max_findings: Option<usize>,
    /// How many findings `--max-findings` kept out of the *listing*. The gate saw
    /// all of them, so this number is a display fact and never a verdict fact.
    pub not_listed: usize,
    /// Why the baseline could not be read, when one was given. Reported rather
    /// than swallowed: a baseline that fails to load must not gate with nothing.
    pub baseline_error: Option<String>,
    /// The environment this run saw, if it was captured. Populated by the
    /// top-level command from the shared discovery, and written beside
    /// `report.json` when present so a verdict is attributable to the toolchain
    /// and solver that decided it.
    pub env: Option<EnvValue>,
    pub coverage: RunCoverage,
    pub run_directory: Option<std::path::PathBuf>,
    pub run_status: String,
    pub build_elapsed_ms: Option<u64>,
    /// How many findings a `keller.toml` `advisory` policy kept out of the gate.
    ///
    /// Reported rather than folded into `suppressed`: a baseline is a temporary
    /// record of known findings, while this is a standing decision about a crate,
    /// and a reader has to be able to tell which one decided the exit code.
    pub advisory_excluded: usize,
}

/// Attribute compiler and runtime-loader paths to the compiler a worker actually ran.
///
/// Discovery may see the parent process's rustup override, which can differ from
/// Cargo's `RUSTC` path. The worker receipt is authoritative for both `rustc` and
/// the host runtime directory; keep non-toolchain path additions such as Z3.
fn attribute_worker_compiler(
    env: &mut EnvValue,
    compiler: &serde_json::Value,
) -> Result<(), String> {
    let rustc = compiler["rustc"]
        .as_str()
        .ok_or_else(|| "Worker receipt has no rustc path.".to_string())?;
    let version = compiler["version_verbose"]
        .as_str()
        .ok_or_else(|| "Worker receipt has no rustc -vV output.".to_string())?;
    let host = version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or_else(|| "Worker rustc -vV output has no host triple.".to_string())?;

    let discovered_rustlib = env["rustlib"].as_str().map(std::path::PathBuf::from);
    let worker_rustlib = crate::host_env::driver_lib_dir(std::path::Path::new(rustc), host);
    let mut paths: Vec<std::path::PathBuf> = env["path_add"]
        .as_str()
        .map(std::env::split_paths)
        .into_iter()
        .flatten()
        .filter(|path| Some(path) != discovered_rustlib.as_ref() && path != &worker_rustlib)
        .collect();
    paths.insert(0, worker_rustlib.clone());
    let path_add = std::env::join_paths(paths)
        .map_err(|err| format!("Could not compose worker runtime PATH: {err}"))?;

    env["rustc"] = serde_json::json!(rustc);
    env["rustc_version_verbose"] = serde_json::json!(version);
    env["host"] = serde_json::json!(host);
    env["rustlib"] = serde_json::json!(worker_rustlib.display().to_string());
    env["path_add"] = serde_json::json!(path_add.to_string_lossy());
    env["compiler_identity_source"] = serde_json::json!(
        "actual worker rustc path and -vV; full arguments in report.run.json coverage.compilers"
    );
    env["pointer_width"] = serde_json::json!(usize::BITS);
    env["tool_version"] = serde_json::json!(self::version());
    env["schema_version"] = serde_json::json!(1);
    let is_release = std::env::var("KELLER_RELEASE").is_ok();
    env["analysis_profile"] = serde_json::json!(analysis_profile(is_release));
    env["identity_note"] = serde_json::json!("rustc, host, rustlib and its PATH entry come from actual worker receipts. z3_bin is the discovered replay executable; the solver this process loaded is recorded under provenance.solver, its version asked of the loaded library itself, and provenance.config_hash digests the settings the verdict was produced under.");
    Ok(())
}

/// Merge every per-crate report into `report.json`.
///
/// Returns the breach of the exit-code contract, if any, together with the
/// findings it was decided on. That return value is the gate, so it has to
/// reflect every crate rather than whichever worker finished last — and it says
/// *why* the gate failed, because an undecided verdict and a counterexample are
/// not the same finding.
///
/// `report.json` keeps *every* finding, including baselined ones: the report is
/// the record of what the tool saw, while the baseline only says what the gate
/// should stop complaining about. A baseline that also erased findings from the
/// report would hide real problems from the people reading it.
pub fn merge_worker_reports(options: &VerifyOptions) -> Result<MergeOutcome, String> {
    let (reports, coverage, obligations) = read_workers(&artifact_path(REPORT_DIR))?;
    let mut merged = merge_events(reports);

    // `--only`: a review aid for a crate whose first run is a wall of findings.
    // Applied here, before the baseline and the gate, because a narrowed run is a
    // narrowed *run* — the summary says `scope: partial` so nobody can read the
    // exit code as a statement about the whole crate.
    let mut filtered_out = 0usize;
    if let Some(pattern) = &options.only {
        let needle = pattern.to_lowercase();
        let before = merged.len();
        merged.retain(|event| {
            format!("{}::{}", event.crate_name, event.function)
                .to_lowercase()
                .contains(&needle)
        });
        filtered_out = before - merged.len();
    }

    let mut env = options.env.clone();
    if let (Some(env), Some(compiler)) = (&mut env, coverage.compilers.first()) {
        attribute_worker_compiler(env, compiler)?;
    }

    // The environment is recorded even when there is nothing to report: a run
    // that proved everything is exactly the run whose verdict a reader may want
    // to attribute to a toolchain. It goes in its own file (see [`ENV_FILE`]),
    // so `report.json` stays the bare findings array the goldens pin.
    if let Some(env) = &env {
        write_report_environment(&artifact_path(ENV_FILE), env)?;
    }

    write_json(&artifact_path(REPORT_FILE), &merged)?;

    // The index of what this run emitted: one file per crate, listing every
    // obligation — proofs included, which `report.json` deliberately does not
    // carry — so a reviewer reads one artifact instead of globbing the directory.
    // Written before the gate so a failing run leaves it behind too: the evidence
    // of what was checked is not contingent on the verdict.
    write_obligation_manifests(&obligations)?;
    // The headline is part of the run record as well as the summary and the
    // console: a retained run is what an audit reads later, and a record that
    // lists findings but not what was left unmodelled invites the reader to treat
    // the finding list as the whole of what was checked.
    let coverage_summary = coverage_headline(&coverage.totals);
    /* RUN_FILE is published only after findings and the gate result are finalized. */
    let run_record = serde_json::json!({
        "$schema": "https://kellersystems.dev/schemas/run-v1.json",
        "schema_version": 1,
        "completed": false,
        "run_id": options.run_id,
        "run_directory": options.run_directory,
        "tool": TOOL_NAME,
        "version": version(),
        // The compiler this binary is valid for, not merely the one a worker
        // happened to run: the run record is retained evidence, and an audit of a
        // verdict has to be able to say which private MIR dialect produced it.
        "compiler": crate::compiler::this_build().as_json(),
        // P1.4: what ran, not only what was configured. The build identity, the
        // solver library this process loaded (version asked of the loaded library)
        // and a digest of the effective settings, so two retained verdicts can be
        // told apart without comparing `settings` field by field by eye.
        "provenance": crate::provenance::evidence(options),
        "coverage": coverage,
        "coverage_summary": coverage_summary,
        "env": env,
        "settings": {
            "strict_coverage": options.strict_coverage,
            "deny_warn": options.deny_warn,
            // The *effective* budget, not the flag: a reader has to be able to tell
            // a verdict that came from the default from one the caller chose, and
            // whether any limit was in force at all.
            "solver_timeout_ms": options
                .solver_timeout_ms
                .unwrap_or(DEFAULT_SOLVER_TIMEOUT_MS),
            "solver_timeout_source": if options.solver_timeout_ms.is_none() {
                "default"
            } else {
                "flag"
            },
            // Recorded the same way, and for the same reason: a reader has to be
            // able to tell a run stopped at the default cap from one stopped at a
            // cap the caller chose.
            "run_timeout_seconds": options
                .run_timeout_seconds
                .unwrap_or(DEFAULT_RUN_TIMEOUT_SECONDS),
            "run_timeout_source": if options.run_timeout_seconds.is_none() {
                "default"
            } else {
                "flag"
            },
            "gate_profile": options.gate_profile,
            "only": options.only,
            "max_findings": options.max_findings,
            "keep_runs": options.keep_runs.unwrap_or(10),
            "max_rounds": crate::path_state::round_budget(),
            "emit_smt2": std::env::var("KELLER_EMIT_SMT2").map(|v| v != "0").unwrap_or(true),
            "fresh_workspace_analysis": true,
            "dependency_cache": !options.no_cache,
            "build_elapsed_ms": options.build_elapsed_ms,
            "workspace_only": true,
            "requested_action": if options.write_baseline.is_some() { "record-baseline" } else { "gate" },
            "profile": "cargo check (development; effective flags may override defaults)",
            "rustflags": std::env::var("RUSTFLAGS").ok(),
            "encoded_rustflags": std::env::var("CARGO_ENCODED_RUSTFLAGS").ok(),
        },
        "scope": "Reachable supported MIR assertions in compiled targets; not a no-panic guarantee.",
        "evidence": "SAT witnesses satisfy the encoded model; abstractions can produce non-executable counterexamples.",
        // A5: the verdict describes a *debug* build, because that is where rustc
        // emits the overflow, bounds and div-by-zero assertions this tool reads. A
        // release build wraps instead of panicking, so "this cannot overflow" and
        // "this will not abort" are different claims, and every report has to say
        // which one it is making.
        "analysis_profile": ANALYSIS_PROFILE,
        "analysis_profile_note": ANALYSIS_PROFILE_NOTE,
        "limitations": ["debug-build-only verdicts", "loop widening", "unmodelled calls/memory/unchecked validity", "no concrete reproduction or general soundness certification"],
        "finding_evidence": merged.iter().map(|finding| serde_json::json!({
            "crate": finding.crate_name,
            "function": finding.function,
            "block": finding.block,
            "status": finding.status,
                "classification": if finding.status == "FAIL" {
                if finding.details.contains("constant folding") { "constant-folded-candidate" } else { "model-satisfiable-candidate" }
            } else { "incomplete-proof" },
            "native_reproduction": "not-attempted",
            "native_reproduction_note": "`keller reproduce` pins this site's identity, flags and obligation and re-discharges the encoded formula; no concrete execution is reconstructed",
            "disclosures": function_disclosures(&coverage, &finding.crate_name, &finding.function),
        })).collect::<Vec<_>>(),
    });

    let crates: std::collections::BTreeSet<&str> =
        merged.iter().map(|e| e.crate_name.as_str()).collect();
    eprintln!(
        "[Keller-Pipeline] Merged {} issue(s) from {} crate(s) into '{}'.",
        merged.len(),
        crates.len(),
        REPORT_FILE
    );

    // The baseline says which of those findings are already known, so the gate
    // can be switched on for a codebase that has not been clean yet.
    let mut baseline_error = None;
    let mut suppressed = 0;
    let mut gated = merged.clone();
    if let Some(path) = &options.baseline {
        match load_baseline(path) {
            Ok(keys) => {
                let (kept, count) = suppress(&merged, &keys);
                suppressed = count;
                gated = kept;
                if count > 0 {
                    eprintln!(
                        "[Keller-Pipeline] {} finding(s) are in the baseline '{}' and do not fail the build.",
                        count,
                        path.display()
                    );
                }
            }
            // Never fall back to an empty baseline: that would turn a typo in the
            // path into a gate that complains about every known finding at once.
            Err(err) => {
                eprintln!("[Keller-Pipeline] {err}");
                baseline_error = Some(err);
            }
        }
    }

    if options.strict_coverage {
        // A baseline records known findings; it cannot certify missing coverage.
        for event in &merged {
            if event.status == "WARN" {
                if !gated.contains(event) {
                    gated.push(event.clone());
                    suppressed -= 1;
                }
            }
        }
    }
    // The checked-in per-crate or per-path policy (B8 / K19), applied after the baseline
    // and after strict-coverage widening so an `advisory` entry stays advisory however the
    // rest of the run was configured. Applied to the *gate* only: the findings are
    // still in `report.json`, on the console and in SARIF.
    let mut advisory_excluded = 0usize;
    if !options.policies.is_empty() {
        let before = gated.len();
        gated.retain(|event| {
            let is_advisory = options.policies.iter().any(|policy| {
                if !policy.advisory {
                    return false;
                }
                let crate_matches =
                    policy.crate_name.is_empty() || policy.crate_name == event.crate_name;
                let path_matches = match &policy.path {
                    Some(pat) => crate::config::path_matches(pat, &event.file),
                    None => true,
                };
                crate_matches && path_matches
            });
            !is_advisory
        });
        advisory_excluded = before - gated.len();
        if advisory_excluded > 0 {
            eprintln!(
                "[Keller-Pipeline] keller.toml marks {} finding(s) from advisory policy table(s) as reported but not gated.",
                advisory_excluded
            );
        }
    }
    let breach = if options.write_baseline.is_some() {
        None
    } else {
        gate_breach(&gated, options.deny_warn || options.strict_coverage)
    };
    let gate_status = if options.write_baseline.is_some() {
        "baseline-recording"
    } else if baseline_error.is_some() {
        "baseline-usage-error"
    } else if breach.is_some() {
        "gate-failed"
    } else {
        "gate-passed"
    };
    let mut run_record = run_record;
    run_record["gate"] = serde_json::json!({
        "status": gate_status,
        "deny_warn": options.deny_warn,
        "strict_coverage": options.strict_coverage,
        "gated_findings": gated.len(),
        "baseline_suppressed": suppressed,
        "advisory_excluded": advisory_excluded,
        "breach": breach.map(|breach| breach.to_string()),
    });
    write_json(&artifact_path(RUN_FILE), &run_record)?;

    // `--max-findings` caps the listing without touching the gate: the exit code
    // above was decided on every finding, and the summary below says how many were
    // kept out of the list so the number is never silently incomplete.
    let not_listed = match options.max_findings {
        Some(limit) if merged.len() > limit => merged.len() - limit,
        _ => 0,
    };

    Ok(MergeOutcome {
        breach,
        findings: merged,
        gated,
        suppressed,
        filtered_out,
        max_findings: options.max_findings,
        not_listed,
        baseline_error,
        env,
        coverage,
        run_directory: options.run_directory.clone(),
        run_status: gate_status.to_string(),
        build_elapsed_ms: options.build_elapsed_ms,
        advisory_excluded,
    })
}

/// The findings a listing flag would show, in report order.
///
/// Ordering is `merge_events`'s (crate, function, block, status), so the first N
/// findings of a run are stable between runs — which is what makes
/// `--max-findings 10` a usable window onto a crate rather than a lottery.
pub fn listed_findings(
    findings: &[VerificationEvent],
    limit: Option<usize>,
) -> &[VerificationEvent] {
    match limit {
        Some(limit) => &findings[..findings.len().min(limit)],
        None => findings,
    }
}

/// The coverage the run can state, as opposed to the findings it can list.
///
/// The gap (K8) was that the unsupported fraction was a footnote:
/// a reader of `report.json` could see two findings and not know that the run had
/// decided 34 obligations or that it had skipped one assertion kind entirely. This
/// is the headline — encountered, decided, and *why* the rest was not decided —
/// computed from the coverage the workers already validate against their findings.
pub fn coverage_headline(coverage: &Coverage) -> serde_json::Value {
    let encountered = coverage.reachable_assertions;
    let decided = coverage.passed + coverage.failed;
    let undecided = coverage.supported_assertions.saturating_sub(decided);
    // Rounded to three places so two runs of the same crate print the same number
    // rather than a float's last bit.
    let ratio = if encountered == 0 {
        1.0
    } else {
        decided as f64 / encountered as f64
    };
    serde_json::json!({
        "encountered": encountered,
        "decided": decided,
        "proved": coverage.passed,
        "counterexamples": coverage.failed,
        "undecided": undecided,
        "unsupported": coverage.unsupported_assertions,
        // Call sites that can panic through a callee this analysis does not read.
        // Beside the assertion counts rather than inside them: these were never
        // assertions, and a run's ratio of decided assertions says nothing about
        // them.
        "panic_sites": coverage.panic_sites,
        // The other half of the panic surface: sites this analysis settled. A
        // safe site leaves no event, so this count is the only trace it was
        // looked at; a decided panic is a FAIL event as well.
        "panic_sites_decided": coverage.panic_sites_decided,
        "decided_ratio": (ratio * 1000.0).round() / 1000.0,
        "unsupported_kinds": coverage.unsupported_kinds,
    })
}

/// One line saying how much of what was encountered was actually decided.
///
/// Printed by the top-level command rather than by a worker, because coverage is a
/// property of the run: a workspace with four crates has one coverage figure, and
/// four per-crate lines would invite a reader to add them up.
pub fn print_coverage_headline(coverage: &Coverage) {
    if quiet() {
        return;
    }
    let headline = coverage_headline(coverage);
    let encountered = coverage.reachable_assertions;
    let decided = coverage.passed + coverage.failed;
    let percent = headline["decided_ratio"].as_f64().unwrap_or(1.0) * 100.0;
    let unsupported = coverage.unsupported_assertions;
    let undecided = coverage.supported_assertions.saturating_sub(decided);
    // To stderr, like every other line the top-level command prints. `--json`
    // promises stdout is the summary and nothing else, and a coverage line written
    // to stdout is exactly the kind of thing that turns a pipeline's `json.loads`
    // into a parse error — which is how this was caught.
    eprintln!(
        "[Keller] Coverage: {decided} of {encountered} encountered assertion(s) decided ({percent:.1}%)"
    );
    if unsupported > 0 {
        let kinds = if coverage.unsupported_kinds.is_empty() {
            String::new()
        } else {
            format!(" [{}]", coverage.unsupported_kinds.join(", "))
        };
        eprintln!(
            "[Keller]   {unsupported} assertion(s) of a kind this build does not model were not attempted{kinds}"
        );
    }
    if undecided > 0 {
        eprintln!("[Keller]   {undecided} assertion(s) were attempted and left undecided (WARN)");
    }
    // Said out loud rather than left to the reader: an assertion count says
    // nothing about the calls that can panic on their own, and a report with no
    // findings would otherwise read as "this cannot panic".
    if coverage.panic_sites > 0 {
        eprintln!(
            "[Keller]   {} call site(s) that can panic were not modelled (WARN)",
            coverage.panic_sites
        );
    }
    if coverage.panic_sites_decided > 0 {
        eprintln!(
            "[Keller]   {} call site(s) that can panic were decided (proof or counterexample)",
            coverage.panic_sites_decided
        );
    }
}

fn function_disclosures(
    coverage: &RunCoverage,
    crate_name: &str,
    function: &str,
) -> Vec<serde_json::Value> {
    coverage
        .function_evidence
        .iter()
        .filter(|e| {
            e["crate"].as_str() == Some(crate_name) && e["function"].as_str() == Some(function)
        })
        .cloned()
        .collect()
}

/// The `--json` summary: the findings plus the counts a pipeline would otherwise
/// have to recount, and the tool version that produced them.
///
/// `report.json` stays a bare array of findings — the goldens pin its bytes —
/// which is why the tool version stamp and the totals live here instead of in
/// that file. The `env` field, when present, is the same value written beside the
/// report in [`ENV_FILE`], so a reader of the summary can tell which toolchain
/// and solver produced this run without opening a second file.
pub fn summary_json(outcome: &MergeOutcome) -> Result<String, String> {
    let count = |status: &str| {
        outcome
            .findings
            .iter()
            .filter(|event| event.status == status)
            .count()
    };
    // A `WARN`'s `kind` is what separates a lost precision from a solver that
    // answered `unknown`, and the distinction is the whole point of recording it:
    // one is fixed by investigating a CFG shape, the other by raising a solver
    // budget. Counted here so a pipeline does not have to scan `findings` for it.
    let path_insensitive = outcome
        .findings
        .iter()
        .filter(|event| event.kind == "path-insensitive")
        .count();
    // Why the failures are failures, counted so a decision about what to fix next
    // does not require scanning every finding (K6). A class is only
    // ever on a FAIL; anything else is absent rather than counted as unknown.
    let mut by_defect_class: std::collections::BTreeMap<&str, usize> =
        std::collections::BTreeMap::new();
    for event in &outcome.findings {
        if !event.defect_class.is_empty() {
            *by_defect_class
                .entry(event.defect_class.as_str())
                .or_default() += 1;
        }
    }
    let value = serde_json::json!({
        "tool": TOOL_NAME,
        "version": version(),
        // A5: the same claim the run record and the SARIF log carry. A pipeline
        // reads this summary, not `report.run.json`, and a verdict it cannot
        // attribute to a build profile is a verdict it can quote wrongly.
        "analysis_profile": ANALYSIS_PROFILE,
        "analysis_profile_note": ANALYSIS_PROFILE_NOTE,
        // Whether this verdict covers the crate or only the part a filter selected.
        // A machine consumer has to be able to tell a narrowed pass from a full one.
        "scope": if outcome.filtered_out > 0 { "partial" } else { "full" },
        "counts": {
            "fail": count("FAIL"),
            "warn": count("WARN"),
            "path_insensitive": path_insensitive,
            "suppressed": outcome.suppressed,
            "advisory_excluded": outcome.advisory_excluded,
            "gated": outcome.gated.len(),
            "filtered_out": outcome.filtered_out,
            "not_listed": outcome.not_listed,
            "by_defect_class": by_defect_class,
        },
        // The listing, capped by `--max-findings`; the counts above and the gate
        // still describe every finding, and `not_listed` says how many are absent.
        "findings": listed_findings(&outcome.findings, outcome.max_findings),
        "coverage": outcome.coverage,
        // The headline: how much of what was encountered was actually decided, and
        // how much of the rest was a kind this build cannot model rather than a
        // solver that did not decide.
        "coverage_summary": coverage_headline(&outcome.coverage.totals),
        "run_directory": outcome.run_directory,
        "run_status": outcome.run_status,
        "build_elapsed_ms": outcome.build_elapsed_ms,
        "env": outcome.env,
    });
    serde_json::to_string_pretty(&value)
        .map_err(|err| format!("Could not serialise the summary: {err}"))
}

/// Check if there were any FAIL events and exit accordingly.
pub fn check_failures(shared_report: &Arc<Mutex<SharedReport>>) -> bool {
    let report_guard = shared_report.lock().unwrap();
    if report_guard.events.iter().any(|e| e.status == "FAIL") {
        let is_top_level = std::env::var("KELLER_OUTPUT_WORKER").is_err();
        if !is_top_level {
            eprintln!(
                "[Keller] Crate contributed {} FAIL(s); non-fatal, merged into '{}' by the top-level command.",
                report_guard.events.iter().filter(|e| e.status == "FAIL").count(),
                REPORT_FILE
            );
            false
        } else {
            true
        }
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt_dir(label: &str) -> std::path::PathBuf {
        let dir = temp_path(label);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn receipt(crate_name: &str, completed: bool) -> WorkerReport {
        WorkerReport {
            schema_version: 1,
            crate_name: crate_name.to_string(),
            completed,
            coverage: Coverage::default(),
            findings: Vec::new(),
            compiler: None,
            evidence: Vec::new(),
            obligations: Vec::new(),
        }
    }

    #[test]
    fn clean_worker_receipts_are_required_and_counted() {
        let dir = receipt_dir("clean-receipts");
        assert!(
            read_workers(&dir).is_err(),
            "no workers is not a clean verdict"
        );
        write_json(&dir.join("alpha-1.started"), &receipt("alpha", false)).unwrap();
        assert!(
            read_workers(&dir).is_err(),
            "an unfinished worker is not clean"
        );
        write_json(&dir.join("alpha-1.json"), &receipt("alpha", true)).unwrap();
        let (reports, coverage, obligations) = read_workers(&dir).unwrap();
        assert!(
            obligations.is_empty(),
            "a clean crate emitted no obligation"
        );
        assert_eq!(coverage.workers, 1);
        assert_eq!(reports, vec![Vec::<VerificationEvent>::new()]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn corrupt_wrong_version_and_unpaired_receipts_fail_closed() {
        let dir = receipt_dir("bad-receipts");
        write_json(&dir.join("alpha-1.started"), &receipt("alpha", false)).unwrap();
        let path = dir.join("alpha-1.json");
        std::fs::write(&path, "[").unwrap();
        assert!(read_workers(&dir)
            .unwrap_err()
            .contains("Invalid worker receipt"));
        let mut wrong = receipt("alpha", true);
        wrong.schema_version = 2;
        write_json(&path, &wrong).unwrap();
        assert!(read_workers(&dir).is_err());
        write_json(&path, &receipt("beta", true)).unwrap();
        assert!(
            read_workers(&dir).is_err(),
            "a different crate cannot complete alpha"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn inconsistent_coverage_and_missing_findings_fail_closed() {
        let dir = receipt_dir("bad-counts");
        write_json(&dir.join("alpha-1.started"), &receipt("alpha", false)).unwrap();
        let mut end = receipt("alpha", true);
        end.coverage.failed = 1;
        write_json(&dir.join("alpha-1.json"), &end).unwrap();
        assert!(read_workers(&dir).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn replay_rejects_unknown_errors_and_mismatches() {
        assert!(check_replay("sat", "sat\r\n").is_ok());
        for output in ["unsat", "unknown", "(error invalid)\nsat", "sat\nsat", ""] {
            assert!(check_replay("sat", output).is_err());
        }
        assert_eq!(
            obligation_expected("; keller-verify proof obligation\n; verdict:  FAIL (folded)\n")
                .unwrap(),
            "sat"
        );
        assert_eq!(
            obligation_expected("; keller-verify proof obligation\n; verdict: WARN (unknown)\n")
                .unwrap(),
            "unknown"
        );
        assert!(check_replay("unknown", "unknown").is_err());
        assert!(obligation_expected("(check-sat)").is_err());
    }

    #[test]
    fn empty_reports_are_arrays_and_output_errors_are_returned() {
        let dir = receipt_dir("output-errors");
        let path = dir.join("report.json");
        write_json(&path, &Vec::<VerificationEvent>::new()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[]");
        assert!(write_json(&dir, &Vec::<VerificationEvent>::new()).is_err());
        assert!(write_report_environment(&dir, &serde_json::json!({})).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn wrapped_command_survives_as_one_form() {
        // Z3 wraps a long `assert`; a line-based split would truncate it.
        let dump = "(assert (or (> (+ a b) 2147483647)\n           (< (+ a b) (- 2147483648))))\n(check-sat)\n";
        let forms = split_top_level_forms(dump);
        assert_eq!(forms.len(), 2, "got {:?}", forms);
        assert!(forms[0].starts_with("(assert"));
        assert!(forms[0].ends_with("(- 2147483648))))"));
        assert_eq!(forms[1], "(check-sat)");
    }

    #[test]
    fn comments_between_commands_are_dropped() {
        let dump = "; function: f\n(declare-fun x () Int)\n; block: bb0\n(assert (= x 1))\n";
        let forms = split_top_level_forms(dump);
        assert_eq!(forms, vec!["(declare-fun x () Int)", "(assert (= x 1))"]);
    }

    fn event(crate_name: &str, function: &str, status: &str) -> VerificationEvent {
        VerificationEvent {
            crate_name: crate_name.to_string(),
            function: function.to_string(),
            block: "bb0".to_string(),
            status: status.to_string(),
            kind: "overflow".to_string(),
            // Set by the cases that are about a failure's class.
            defect_class: String::new(),
            file: String::new(),
            line: None,
            details: format!("{} detail", function),
            witness: Vec::new(),
            witness_kind: String::new(),
        }
    }

    #[test]
    fn a_cache_hit_must_republish_one_artifact_per_obligation_event() {
        // A PASS/WARN leaves one file; a FAIL with a witness leaves the
        // obligation plus its pinned companion; an unsupported assertion is not
        // an obligation, so a cache entry that counted it as one would still be
        // treated as complete while a file was missing.
        let pass = event("alpha", "proved", "PASS");
        let mut fail = event("alpha", "failed", "FAIL");
        fail.witness = vec![Witness {
            input: "x (_1)".to_string(),
            symbol: "x".to_string(),
            value: "1".to_string(),
        }];
        let mut unsupported = event("alpha", "unsupported", "WARN");
        unsupported.kind = "unsupported-assertion".to_string();
        assert_eq!(expected_obligation_artifacts(&[pass, fail, unsupported]), 3);
    }

    #[test]
    fn witness_survives_serialisation_and_the_merge() {
        // The witness has to make it into report.json intact: it is the part of a
        // FAIL a reader acts on, and a 128-bit value must not be rounded.
        let mut finding = event("alpha", "alpha_overflow", "FAIL");
        finding.witness = vec![
            Witness {
                input: "x (_1)".to_string(),
                symbol: "overflow_l".to_string(),
                value: "4294967295".to_string(),
            },
            Witness {
                input: "y (_2)".to_string(),
                symbol: "overflow_r".to_string(),
                value: "18446744073709551616".to_string(),
            },
        ];

        let json =
            serde_json::to_string_pretty(&merge_events(vec![vec![finding.clone()]])).unwrap();
        assert!(json.contains("\"symbol\": \"overflow_l\""), "got {}", json);
        assert!(
            json.contains("\"value\": \"18446744073709551616\""),
            "got {}",
            json
        );
        let back: Vec<VerificationEvent> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, vec![finding]);
    }

    #[test]
    fn an_event_without_a_witness_omits_the_field() {
        // PASS events must not grow a `"witness": []` line in report.json.
        let json = serde_json::to_string(&event("alpha", "f", "PASS")).unwrap();
        assert!(!json.contains("witness"), "got {}", json);
    }

    #[test]
    fn a_counterexample_always_violates_the_contract() {
        // FAIL blocks under either policy: the solver exhibited an input that
        // reaches the assertion, and `--deny-warn` neither adds nor removes that.
        let events = vec![event("alpha", "alpha_overflow", "FAIL")];
        let expected = Some(GateBreach::Counterexample { count: 1 });
        assert_eq!(gate_breach(&events, false), expected);
        assert_eq!(gate_breach(&events, true), expected);
    }

    #[test]
    fn an_undecided_verdict_violates_the_contract_only_when_asked() {
        // The flag in one test. `unknown` is a gap in the proof rather than a
        // bug, so it passes by default and blocks under `--deny-warn` — which is
        // the point: CI can require a definite answer.
        let events = vec![event("alpha", "complex", "WARN")];
        assert_eq!(gate_breach(&events, false), None);
        assert_eq!(
            gate_breach(&events, true),
            Some(GateBreach::Undecided { count: 1 })
        );
    }

    #[test]
    fn a_lost_path_sensitivity_blocks_the_gate_only_when_asked() {
        // The precision finding rides the WARN channel, and the contract it obeys
        // is the WARN contract: nothing was proved in the affirmative, so a run
        // that only lost precision is tolerated by default and fatal under
        // `--deny-warn`. The `kind` is what keeps it distinguishable, and the
        // point of the record is that a false positive in the same report can be
        // attributed to it.
        let mut imprecise = event("alpha", "a_nested_walk", "WARN");
        imprecise.kind = "path-insensitive".to_string();
        let events = vec![imprecise];
        assert_eq!(gate_breach(&events, false), None);
        assert_eq!(
            gate_breach(&events, true),
            Some(GateBreach::Undecided { count: 1 })
        );
    }

    #[test]
    fn a_clean_report_never_violates_the_contract() {
        // The flag must not make a proved run fail; otherwise it would be
        // unusable as a gate.
        let events = vec![event("alpha", "ok", "PASS")];
        assert_eq!(gate_breach(&events, false), None);
        assert_eq!(gate_breach(&events, true), None);
    }

    #[test]
    fn an_undecided_verdict_survives_the_merge_and_then_blocks_the_gate() {
        // The chain rather than the predicate alone. A worker's WARN has to reach
        // `report.json` — the artefact CI reads and the only thing the gate is
        // given — before `--deny-warn` can mean anything: a WARN dropped by the
        // merge would pass silently, and the flag would look like it worked.
        let merged = merge_events(vec![
            vec![event("alpha", "complex", "WARN")],
            // The same crate again, as two targets of one package produce.
            vec![event("alpha", "complex", "WARN")],
        ]);
        assert_eq!(merged.len(), 1, "the duplicate must collapse: {merged:?}");
        assert_eq!(merged[0].status, "WARN");
        assert_eq!(gate_breach(&merged, false), None);
        assert_eq!(
            gate_breach(&merged, true),
            Some(GateBreach::Undecided { count: 1 })
        );
    }

    #[test]
    fn an_undecided_verdict_is_not_laundered_into_a_counterexample() {
        // Both kinds present: the message must name both counts, so a WARN is
        // never reported as a counterexample nor hidden behind one.
        let events = vec![
            event("alpha", "alpha_overflow", "FAIL"),
            event("beta", "complex", "WARN"),
        ];
        assert_eq!(
            gate_breach(&events, false),
            Some(GateBreach::Counterexample { count: 1 }),
            "without the flag the WARN must not block"
        );
        let both = gate_breach(&events, true).expect("both kinds breach the contract");
        assert_eq!(
            both,
            GateBreach::Both {
                counterexamples: 1,
                undecided: 1
            }
        );
        let message = both.to_string();
        assert!(message.contains("1 FAIL"), "got {message}");
        assert!(message.contains("1 undecided"), "got {message}");
    }

    #[test]
    fn merge_keeps_every_crate() {
        // The bug this guards: two crates' findings used to overwrite each other.
        let merged = merge_events(vec![
            vec![event("alpha", "alpha_overflow", "FAIL")],
            vec![event("beta", "beta_divide", "FAIL")],
        ]);
        assert_eq!(merged.len(), 2, "got {:?}", merged);
        assert_eq!(merged[0].crate_name, "alpha");
        assert_eq!(merged[1].crate_name, "beta");
    }

    #[test]
    fn merge_is_order_independent_and_deduplicates() {
        let a = vec![event("alpha", "f", "FAIL")];
        let b = vec![event("beta", "g", "WARN")];
        // Same inputs in the other order, plus a duplicate compile of alpha.
        let merged = merge_events(vec![b.clone(), a.clone(), a.clone()]);
        assert_eq!(merged, merge_events(vec![a, b]));
        assert_eq!(merged.len(), 2, "duplicates must collapse: {:?}", merged);
    }

    #[test]
    fn merge_does_not_collide_across_crates() {
        // Identical function/block in two crates are two findings, not one.
        let merged = merge_events(vec![
            vec![event("alpha", "same", "FAIL")],
            vec![event("beta", "same", "FAIL")],
        ]);
        assert_eq!(merged.len(), 2, "crate name is part of the identity");
    }

    #[test]
    fn internal_forms_and_quoted_strings_are_kept_whole() {
        let dump = "(model-add overflow_l () Int (+ k!0 (* 2 k!1)))\n(pb2bv-model-converter (k!0 k!1))\n(assert (= s \"a)b\"))\n";
        let forms = split_top_level_forms(dump);
        assert_eq!(forms.len(), 3, "got {:?}", forms);
        assert!(
            forms[2].contains("\"a)b\""),
            "parens inside strings must not count"
        );
    }

    /// A path under the system temp directory that no other test is using. The
    /// tests only need somewhere to write, so no `tempfile` dependency is added.
    fn temp_path(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "keller-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock is after the epoch")
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }

    #[test]
    fn a_baseline_holds_exactly_the_obligations_it_recorded() {
        // The baseline is keyed on the obligation, not on the message, so a
        // reworded detail or a different witness still counts as "already
        // known" — otherwise the first unrelated edit would resurface every
        // finding and the baseline would be useless exactly when it is needed.
        let known = event("alpha", "alpha_overflow", "FAIL");
        let mut same_obligation_other_wording = known.clone();
        same_obligation_other_wording.details = "different prose, same obligation".to_string();
        same_obligation_other_wording.witness = vec![Witness {
            input: "a (_1)".to_string(),
            symbol: "overflow_l".to_string(),
            value: "127".to_string(),
        }];
        let new_one = event("beta", "beta_divide", "FAIL");

        let baseline = vec![FindingKey::of(&known)];
        let (kept, suppressed) = suppress(
            &[
                known.clone(),
                same_obligation_other_wording,
                new_one.clone(),
            ],
            &baseline,
        );
        assert_eq!(suppressed, 2, "both instances of the known obligation");
        assert_eq!(
            kept,
            vec![new_one.clone()],
            "a new obligation is still gated"
        );
        // And nothing else matches it.
        let (kept, suppressed) = suppress(&[known.clone()], &[FindingKey::of(&new_one)]);
        assert_eq!(suppressed, 0);
        assert_eq!(kept, vec![known]);
    }

    #[test]
    fn a_baseline_round_trips_through_a_file() {
        let path = temp_path("baseline.json");
        let events = vec![
            event("alpha", "alpha_overflow", "FAIL"),
            event("alpha", "weird", "WARN"),
        ];
        let recorded = write_baseline(&path, &events).expect("the baseline is written");
        assert_eq!(recorded, 2);

        let loaded = load_baseline(&path).expect("the baseline reads back");
        let mut expected: Vec<FindingKey> = events.iter().map(FindingKey::of).collect();
        expected.sort();
        assert_eq!(loaded, expected);

        // Re-recording an unchanged baseline produces identical bytes: the file is
        // meant to be reviewable in a diff.
        let first = std::fs::read_to_string(&path).expect("read");
        write_baseline(&path, &events).expect("rewrite");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), first);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_malformed_or_future_baseline_is_refused() {
        // A baseline that cannot be read must fail the run with an explanation.
        // Falling back to "no baseline" would turn a typo into a gate that
        // suddenly reports every long-known finding as new.
        let not_a_baseline = temp_path("junk.json");
        std::fs::write(&not_a_baseline, "{\"report\": []}").expect("write");
        assert!(load_baseline(&not_a_baseline).is_err());
        let _ = std::fs::remove_file(&not_a_baseline);

        let from_the_future = temp_path("future.json");
        std::fs::write(
            &from_the_future,
            "{\"keller_baseline\": 999, \"findings\": []}",
        )
        .expect("write");
        let err = load_baseline(&from_the_future).expect_err("a future version is refused");
        assert!(err.contains("version"), "got {err}");
        let _ = std::fs::remove_file(&from_the_future);

        assert!(load_baseline(std::path::Path::new("definitely/missing.json")).is_err());
    }

    #[test]
    fn sarif_log_carries_the_rule_the_level_and_the_location() {
        let path = temp_path("findings.sarif");
        let mut failing = event("alpha", "alpha_overflow", "FAIL");
        failing.kind = "overflow".to_string();
        // A failure keeps its obligation class *and* carries why it failed, so the
        // rule id has to name both: a dashboard filters "may be the loop
        // abstraction" apart from "a bug" on exactly this (K6).
        failing.defect_class = "widening-limited".to_string();
        failing.file = "tests/alpha/src/main.rs".to_string();
        failing.line = Some(12);
        let mut warning = event("beta", "beta_divide", "WARN");
        // No file: the span was unavailable, so the result must carry only the
        // logical location rather than inventing a path.
        warning.kind = "divzero".to_string();
        // The precision finding has a class of its own, so it must get a rule of
        // its own too rather than falling through to the unclassified one.
        let mut imprecise = event("gamma", "a_nested_walk", "WARN");
        imprecise.kind = "path-insensitive".to_string();
        imprecise.block = "body".to_string();

        // K8: the annotations are only part of the answer. This run encountered
        // five assertions and could not attempt one of them, so the SARIF run has
        // to say so beside the results rather than leaving a code-scanning reader
        // to infer completeness from an empty rule list.
        let coverage = Coverage {
            reachable_assertions: 5,
            supported_assertions: 4,
            unsupported_assertions: 1,
            unsupported_kinds: vec!["BoundsCheck".to_string()],
            passed: 3,
            failed: 1,
            ..Coverage::default()
        };
        write_sarif(&path, &[failing, warning, imprecise], &coverage).expect("the log is written");
        let text = std::fs::read_to_string(&path).expect("read");
        let log: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");

        assert_eq!(log["version"], "2.1.0");
        let run = &log["runs"][0];
        assert_eq!(run["tool"]["driver"]["name"], TOOL_NAME);
        assert_eq!(run["tool"]["driver"]["version"], version());
        assert_eq!(
            run["properties"]["coverage"]["unsupported"], 1,
            "the unsupported fraction has to travel with the results: {run:?}"
        );
        assert_eq!(run["properties"]["coverage"]["encountered"], 5);
        assert_eq!(
            run["properties"]["coverage"]["unsupported_kinds"][0],
            "BoundsCheck"
        );

        let rules = run["tool"]["driver"]["rules"].as_array().expect("rules");
        let ids: Vec<&str> = rules
            .iter()
            .map(|rule| rule["id"].as_str().unwrap())
            .collect();
        assert!(
            ids.contains(&"keller/overflow/widening-limited"),
            "a failure's class is part of its rule: {ids:?}"
        );
        assert!(ids.contains(&"keller/divzero"), "got {ids:?}");
        assert!(
            ids.contains(&"keller/path-insensitive"),
            "a lost precision is a rule of its own: {ids:?}"
        );
        let described: Vec<&str> = rules
            .iter()
            .filter(|rule| rule["id"] == "keller/path-insensitive")
            .map(|rule| rule["shortDescription"]["text"].as_str().unwrap())
            .collect();
        assert_eq!(described.len(), 1, "got {described:?}");
        assert!(
            described[0].contains("false positive"),
            "the rule says what a reader should do with the FAILs: {described:?}"
        );

        let results = run["results"].as_array().expect("results");
        assert_eq!(results.len(), 3);
        let first = &results[0];
        assert_eq!(first["ruleId"], "keller/overflow/widening-limited");
        assert_eq!(
            first["properties"]["defectClass"], "widening-limited",
            "the class is a property too, so a consumer need not parse the rule id"
        );
        assert_eq!(first["level"], "error", "a counterexample is an error");
        assert_eq!(
            first["locations"][0]["physicalLocation"]["region"]["startLine"],
            12
        );
        assert_eq!(
            first["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "tests/alpha/src/main.rs"
        );
        assert_eq!(
            first["locations"][0]["logicalLocations"][0]["fullyQualifiedName"],
            "alpha_overflow"
        );
        assert_eq!(results[1]["level"], "warning", "a WARN is a warning");
        assert!(
            results[1]["locations"][0]["physicalLocation"].is_null(),
            "no file means no physical location"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn the_summary_json_reports_counts_and_the_version() {
        let mut imprecise = event("gamma", "a_nested_walk", "WARN");
        imprecise.kind = "path-insensitive".to_string();
        let outcome = MergeOutcome {
            breach: Some(GateBreach::Counterexample { count: 1 }),
            findings: vec![
                event("alpha", "alpha_overflow", "FAIL"),
                event("beta", "complex", "WARN"),
                imprecise,
            ],
            gated: vec![event("alpha", "alpha_overflow", "FAIL")],
            suppressed: 1,
            filtered_out: 0,
            max_findings: None,
            not_listed: 0,
            baseline_error: None,
            env: Some(serde_json::json!({ "z3_bin": "z3-under-test" })),
            coverage: RunCoverage::default(),
            run_directory: None,
            run_status: "gate-passed".to_string(),
            build_elapsed_ms: None,
            advisory_excluded: 0,
        };
        let text = summary_json(&outcome).expect("the summary serialises");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(value["tool"], TOOL_NAME);
        assert_eq!(value["version"], version());
        assert_eq!(value["counts"]["fail"], 1);
        assert_eq!(value["counts"]["warn"], 2);
        assert_eq!(
            value["counts"]["path_insensitive"], 1,
            "a pipeline must be able to see the lost precision without scanning findings"
        );
        assert_eq!(value["counts"]["suppressed"], 1);
        assert_eq!(value["counts"]["gated"], 1);
        assert_eq!(value["findings"].as_array().expect("findings").len(), 3);
        assert_eq!(
            value["env"]["z3_bin"], "z3-under-test",
            "the summary carries the environment the run was decided in"
        );
    }

    #[test]
    fn the_run_environment_is_written_beside_the_report() {
        // These facts are absolute paths, which is why they live in their own
        // file: `report.json` is byte-compared against the committed goldens,
        // and a machine-specific path inside it would fail every one of them on
        // another machine. The test is that the sidecar carries exactly what
        // `keller-env` prints - the same keys, from the same discovery - so a
        // reader can attribute a verdict to the toolchain and solver behind it.
        // Discovery can run under a different rustup override from Cargo's
        // actual worker rustc. The evidence must replace that stale loader path
        // while retaining the discovered Z3 directory.
        let root = std::env::temp_dir().join(format!("keller-worker-path-{}", std::process::id()));
        let stale_rustc = root.join("msvc").join("bin").join("rustc.exe");
        let actual_rustc = root.join("gnu").join("bin").join("rustc.exe");
        let stale_rustlib = crate::host_env::driver_lib_dir(&stale_rustc, "x86_64-pc-windows-msvc");
        let z3_bin = root.join("z3").join("bin");
        let old_paths = std::env::join_paths([&stale_rustlib, &z3_bin]).unwrap();
        let mut observed = serde_json::json!({
            "rustc": stale_rustc,
            "host": "x86_64-pc-windows-msvc",
            "rustlib": stale_rustlib,
            "path_add": old_paths.to_string_lossy(),
            "z3_bin": root.join("z3").join("bin").join("z3.exe"),
        });
        let compiler = serde_json::json!({
            "rustc": actual_rustc,
            "version_verbose": "rustc 1.99.0-nightly\nhost: x86_64-pc-windows-gnu",
            "arguments": [],
        });
        attribute_worker_compiler(&mut observed, &compiler).unwrap();

        let path = temp_path("report.environment.json");
        write_report_environment(&path, &observed).expect("write attributed sidecar");
        let text = std::fs::read_to_string(&path).expect("read");
        let written: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(written, observed, "the corrected provenance is persisted");

        let actual_rustlib = crate::host_env::driver_lib_dir(
            compiler["rustc"]
                .as_str()
                .map(std::path::Path::new)
                .unwrap(),
            "x86_64-pc-windows-gnu",
        );
        let runtime_paths: Vec<_> =
            std::env::split_paths(observed["path_add"].as_str().unwrap()).collect();
        assert_eq!(observed["rustc"], compiler["rustc"]);
        assert_eq!(observed["host"], "x86_64-pc-windows-gnu");
        assert_eq!(observed["rustlib"], actual_rustlib.display().to_string());
        assert_eq!(runtime_paths.first(), Some(&actual_rustlib));
        assert!(!runtime_paths.contains(&stale_rustlib));
        assert!(runtime_paths.contains(&z3_bin));
        assert!(observed["identity_note"]
            .as_str()
            .unwrap()
            .contains("worker receipts"));
        let _ = std::fs::remove_file(&path);
    }
}

/// Print a summary of all verification results.
///
/// Printed by the worker that analysed the crate, so in a workspace each crate
/// reports its own totals. The counts are the events this crate produced; only
/// FAIL and WARN reach `report.json`, which is why the total analysed can exceed
/// the number of findings.
/// `widening-limited 2`, and `3 what the loop abstraction may have cost` for the
/// class a reader most needs spelled out.
fn to_class_label(class: &str, count: usize) -> String {
    if class == "widening-limited" {
        format!("{class} {count} (may be what the loop abstraction cost)")
    } else {
        format!("{class} {count}")
    }
}

pub fn print_summary(shared_report: &Arc<Mutex<SharedReport>>, elapsed: std::time::Duration) {
    if quiet() {
        return;
    }
    let report_guard = shared_report.lock().unwrap();
    // Assertions only. A panic site this analysis does not read is a WARN, but it
    // is not an assertion, and counting one here inflated the headline: a crate
    // with two assertions and six `unwrap`s read as "eight assertions checked",
    // which is the same overstatement as reporting nothing at all, from the other
    // side. It gets its own line below instead.
    let assertion = |event: &&VerificationEvent| {
        event.kind != "unmodelled-panic" && event.kind != "decided-panic"
    };
    let total = report_guard.events.iter().filter(assertion).count();
    let fail_count = report_guard
        .events
        .iter()
        .filter(|e| e.status == "FAIL")
        .count();
    let pass_count = report_guard
        .events
        .iter()
        .filter(assertion)
        .filter(|e| e.status == "PASS")
        .count();
    let warn_count = report_guard
        .events
        .iter()
        .filter(assertion)
        .filter(|e| e.status == "WARN")
        .count();
    let panic_sites = report_guard.coverage.panic_sites;
    let panic_sites_decided = report_guard.coverage.panic_sites_decided;
    let panic_fails = report_guard
        .events
        .iter()
        .filter(|e| e.kind == "decided-panic" && e.status == "FAIL")
        .count();
    // Broken out of the WARN count rather than added to it: a lost precision is
    // recorded as a WARN because nothing was proved, but what it means for the
    // failures next to it — they may be false positives — is worth its own line.
    let imprecise_count = report_guard
        .events
        .iter()
        .filter(|e| e.kind == "path-insensitive")
        .count();

    println!();
    println!("═══ Formal Verification Summary ═══");
    println!("  Total assertions checked: {}", total);
    println!("  ✅ PASS: {}", pass_count);
    println!("  ❌ FAIL: {}", fail_count);
    println!("  ⚠️  WARN: {}", warn_count);
    // Beside the assertion counts rather than inside them, for the same reason
    // the coverage headline keeps it out: a reader deciding whether "no findings"
    // means "cannot panic" needs the number, and needs it not to be summed with
    // the assertions it is not.
    if panic_sites > 0 {
        println!("  ⚠️  Panic sites not modelled: {panic_sites}");
    }
    if panic_sites_decided > 0 {
        // A decided site that panics is already in the FAIL line; saying how many
        // of the decided sites those were separates "the call cannot panic" from
        // "the call does panic", which the single count cannot.
        println!("  ✅ Panic sites decided: {panic_sites_decided} ({panic_fails} panicking)");
    }
    if fail_count > 0 {
        // The line a reader needs to decide what to do about the failures: the
        // ones that lean on the loop abstraction are the suspect ones.
        let mut by_class: Vec<(&str, usize)> = Vec::new();
        for event in report_guard.events.iter().filter(|e| e.status == "FAIL") {
            let class = event.defect_class.as_str();
            match by_class.iter_mut().find(|(name, _)| *name == class) {
                Some((_, count)) => *count += 1,
                None => by_class.push((class, 1)),
            }
        }
        by_class.sort_unstable();
        let rendered = by_class
            .iter()
            .map(|(class, count)| to_class_label(class, *count))
            .collect::<Vec<_>>()
            .join(", ");
        println!("      ↳ FAIL(s) by class: {rendered}");
    }
    if imprecise_count > 0 {
        println!(
            "      ↳ {} of them are path-insensitive fallbacks: a FAIL in that function may be a false positive",
            imprecise_count
        );
    }
    println!(
        "  Recorded to report.json: {} (FAIL+WARN)",
        fail_count + warn_count
    );
    if report_guard.coverage.obligation_solver_calls > 0 {
        println!(
            "  Elapsed: {:.2}s (solver: {:.2}s across {} calls)",
            elapsed.as_secs_f64(),
            (report_guard.coverage.solver_elapsed_ms as f64) / 1000.0,
            report_guard.coverage.obligation_solver_calls
        );
    } else {
        println!("  Elapsed: {:.2}s", elapsed.as_secs_f64());
    }
    println!("═══════════════════════════════════");
}
