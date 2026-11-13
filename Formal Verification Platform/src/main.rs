#![feature(rustc_private)]
#![feature(box_patterns)]

extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

mod langaudit;
#[cfg(feature = "lang-asm")]
mod asm;
#[cfg(feature = "lang-c")]
mod c;
#[cfg(feature = "lang-cpp")]
mod cpp;
#[cfg(feature = "lang-csharp")]
mod csharp;
#[cfg(feature = "lang-go")]
mod go;
#[cfg(feature = "lang-java")]
mod java;
#[cfg(feature = "lang-sql")]
mod sql;
#[cfg(feature = "lang-ts")]
mod typescript;
mod cache;
mod compiler;
#[allow(dead_code)]
#[allow(dead_code)]
mod config;
mod constant_prop;
mod constraint;
mod contract;
mod diagnostics;
/// The same module `build.rs` links against. It is one file serving three
/// targets, so the parts this binary does not call are allowed to be unused --
/// the same reason `src/bin/keller-env.rs` gives for the same attribute.
#[allow(dead_code)]
#[path = "host_env.rs"]
mod host_env;
mod html_report;
#[allow(dead_code)]
mod path_state;
mod pointer;
mod provenance;
mod python;
mod reachability;
mod report;
mod reproduce;
mod run_context;
mod summary;
mod testgen;
mod types;
mod verifier;
pub mod vir;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use crate::report::{
    check_failures, listed_findings, merge_worker_reports, prepare_report_dir,
    print_coverage_headline, start_worker_report, summary_json, version, write_baseline,
    write_sarif, write_worker_report, VerifyOptions, TOOL_NAME,
};
use crate::types::SharedReport;
use crate::verifier::KellerVerifier;

/// Exit codes this binary promises.
///
/// They are part of the contract, not an implementation detail: a pipeline
/// branches on them, and "the author has a counterexample" is a different action
/// from "the code did not compile" or "the command line was wrong".
pub const EXIT_OK: i32 = 0;
pub const EXIT_GATE: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_SETUP: i32 = 3;

/// The status this binary reports to `cargo` when it ran as the rustc wrapper
/// and rustc itself failed.
///
/// Not one of the codes above: this process is standing in for rustc, so it owes
/// `cargo` a compiler status. `rustc_driver` returns an `ExitCode`, which does not
/// expose its numeric value, and cargo only distinguishes zero from non-zero — so
/// the exact code is not preserved and the point is only that a compiler which
/// failed is not reported to cargo as one that succeeded. Cargo turns this into
/// `EXIT_SETUP` at the top level, which is where the distinction is documented.
const WRAPPER_COMPILER_FAILED: i32 = 1;

/// The command line this binary accepts.
const USAGE: &str = "\
Usage: keller verify [OPTIONS]
       keller watch [OPTIONS]
       keller heal [OPTIONS] [REPORT_FILE]
       keller init [--force]
       keller explain <OBLIGATION.smt2>
       keller replay <OBLIGATION.smt2>
       keller reproduce <OBLIGATION.smt2> [--out <DIR>]
       keller schema [report|run]
       keller testgen [REPORT_FILE] [--out <FILE>]

Verifies the crate or workspace in the current directory. The command drives
`cargo check` with itself installed as the rustc wrapper, collects the bounds,
overflow and division assertions the compiler emits, and asks Z3 about each one
under the guards that have to hold to reach it.

Settings can also be checked in: a `keller.toml` in the project directory supplies
what the flags would, and `keller init` writes a starter one. The command line is
always applied on top of the file, so a flag someone typed is never overridden.

Options:
  --watch                  continuously watch files and re-verify on modification
  --profile <NAME>         named gate contract: `advisory` (default) gates only on a
                           counterexample, `strict` also fails on unsupported checks
                           and undecided verdicts. Explicit flags below still win.
  --solver <NAME>          SMT solver backend: `z3` (default), `cvc5`, `bitwuzla`,
                           or `portfolio` (races all available solvers concurrently)
  --python                 verify Python codebase directly (auto-detected if python files present)
  --lang <NAME>            explicit language frontend target (`rust`, `python`, etc.)
  --no-cache               use a private cold build rather than the dependency cache
  --verify-deps            also verify external dependencies (default: verify workspace only)
  --release                run verification in release profile mode (cargo --release)
  --keep-runs <COUNT>      retain this many completed evidence directories (default: 10)
  --strict-coverage        fail on unsupported/undecided checks, even if baselined
  --deny-warn              treat a WARN as a failure: an undecided verdict, or a
                           function whose dataflow lost path sensitivity
  --only <PATTERN>         report and gate only findings whose `crate::function`
                           contains PATTERN. The summary says `scope: partial`, because
                           a narrowed run is not a statement about the whole crate.
  --max-findings <COUNT>   list at most COUNT findings (console, SARIF, --json). The
                           gate still judges every finding, so this can make a first
                           run readable but can never pass a counterexample
  --solver-timeout <MS>    per-obligation solver budget (default: 30000); a budget
                           that runs out is an undecided (WARN) verdict, not a hang.
                           `0` disables the budget, for a caller who asks for it
  --run-timeout <SECONDS>  cap the whole analysis phase (default: 0, disabled). A run that
                           reaches the cap is stopped, its process tree is killed,
                           and it reaches no verdict: exit 3, with the stopped run's
                           state recorded. `0` removes the cap, for a caller who has
                           measured the crate and wants no bound
  --baseline <FILE>        report findings in FILE but do not fail on them
  --write-baseline <FILE>  record the current findings in FILE and exit 0
  --sarif <FILE>           also write a SARIF 2.1.0 log of the gated findings
  --html <FILE>            write an interactive HTML verification dashboard
  --error-format <FORMAT>  compiler diagnostics format: `human` (default) or `json`
  --json                   print the summary (counts and findings) as JSON
  --quiet                  print findings and the verdict, not progress
  --help                   print this message
  --version                print the version

Exit codes:
  0  analysis completed and the selected gate passed (advisory WARNs may remain)
  1  the gate failed: a counterexample, or a WARN under --deny-warn
  2  the command line was not understood
  3  setup/analysis/evidence failed (compiler, receipts, required artifact I/O, or a
     run stopped at `--run-timeout`, which reaches no verdict)
";

/// What the command line asked for.
#[derive(Debug, PartialEq, Eq)]
pub enum Invocation {
    Verify(VerifyOptions),
    Explain(PathBuf),
    Replay(PathBuf),
    /// Write a starter `keller.toml` (B8 / K19). `force` overwrites an existing
    /// one; without it the command refuses rather than replacing decisions someone
    /// wrote down by hand.
    Init {
        force: bool,
    },
    Schema {
        target: String,
    },
    TestGen {
        report: PathBuf,
        out: Option<PathBuf>,
    },
    /// P1.6's first slice: pin one site's identity, flags and obligation into a
    /// bundle and re-discharge it.
    Reproduce {
        obligation: PathBuf,
        out: Option<PathBuf>,
    },
    Heal {
        report: PathBuf,
        out: Option<PathBuf>,
    },
    /// Audit-only run of a non-Rust language engine; writes one report file and
    /// never touches the audited tree.
    Audit {
        lang: String,
        path: PathBuf,
        out: Option<PathBuf>,
    },
    Help,
    Version,
}

/// True when `argument` names the pipeline command instead of being an argument
/// for the rustc wrapper.
///
/// `--help` and `--version` count: they are the two things someone types before
/// they know what subcommand exists, and neither is a plausible rustc argument.
fn is_command_invocation(argument: &str) -> bool {
    matches!(
        argument,
        "keller"
            | "verify"
            | "watch"
            | "heal"
            | "audit"
            | "explain"
            | "replay"
            | "init"
            | "schema"
            | "testgen"
            | "--help"
            | "-h"
            | "--version"
            | "-V"
    )
}

/// Read the command line into an invocation, or explain why it cannot be read.
///
/// Deliberately free of side effects, and of the compiler, so it can be tested
/// without one: the flag surface is what a CI file and a person's fingers talk
/// to, and a flag that silently means something else is worse than one that does
/// not exist.
pub fn parse_args(args: &[String]) -> Result<Invocation, String> {
    parse_args_with_base(args, VerifyOptions::default())
}

/// The command line, applied on top of `keller.toml`.
///
/// The file is the *base* rather than a default that is merged afterwards, so a
/// profile or a flag given on the command line always wins — including when the
/// config sets the same thing the other way. A run policy that silently overrode
/// what someone typed would be a worse trap than no policy at all.
///
/// Only `verify` gates anything, so the other commands never read the file.
pub fn parse_args_for_run(args: &[String]) -> Result<Invocation, String> {
    let base = match std::env::current_dir() {
        Ok(dir) => config::base_options(&config::load(&dir)?.unwrap_or_default()),
        Err(_) => VerifyOptions::default(),
    };
    parse_args_with_base(args, base)
}

/// Read the command line into an invocation, starting from `base`.
///
/// Deliberately free of side effects, and of the compiler, so it can be tested
/// without one: the flag surface is what a CI file and a person's fingers talk
/// to, and a flag that silently means something else is worse than one that does
/// not exist.
pub fn parse_args_with_base(args: &[String], base: VerifyOptions) -> Result<Invocation, String> {
    let rest: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();

    // Before the subcommand as well as after it: `keller-verify --version` is the
    // first thing anyone types when checking an installation.
    match rest.first() {
        Some(&"--help") | Some(&"-h") => return Ok(Invocation::Help),
        Some(&"--version") | Some(&"-V") => return Ok(Invocation::Version),
        _ => {}
    }

    // `cargo keller verify` arrives as ["keller", "verify", ...]; the direct
    // binary arrives as ["verify", ...].
    let mut index = 0;
    if rest.first() == Some(&"keller") {
        index = 1;
    }
    let mut watch_mode = false;
    match rest.get(index) {
        Some(&"verify") => index += 1,
        Some(&"watch") => {
            index += 1;
            watch_mode = true;
        }
        // `keller init`: the adoption path needs a file to write decisions into
        // before anything else can be checked in.
        Some(&"init") => {
            let mut force = false;
            let mut cursor = index + 1;
            while let Some(argument) = rest.get(cursor) {
                cursor += 1;
                match *argument {
                    "--force" | "-f" => force = true,
                    other => return Err(format!("init takes no arguments, got '{other}'.")),
                }
            }
            return Ok(Invocation::Init { force });
        }
        Some(&"explain") | Some(&"replay") => {
            let command = rest[index];
            if rest.len() != index + 2 {
                return Err(format!("{command} expects exactly one obligation file."));
            }
            let path = PathBuf::from(rest[index + 1]);
            return Ok(if command == "explain" {
                Invocation::Explain(path)
            } else {
                Invocation::Replay(path)
            });
        }
        Some(&"reproduce") => {
            let mut obligation: Option<PathBuf> = None;
            let mut out: Option<PathBuf> = None;
            let mut cursor = index + 1;
            while let Some(arg) = rest.get(cursor) {
                cursor += 1;
                match *arg {
                    "--out" | "-o" => {
                        let val = rest
                            .get(cursor)
                            .ok_or_else(|| "--out expects a directory".to_string())?;
                        cursor += 1;
                        out = Some(PathBuf::from(val));
                    }
                    other if !other.starts_with('-') => {
                        if obligation.is_some() {
                            return Err(
                                "reproduce expects exactly one obligation file.".to_string()
                            );
                        }
                        obligation = Some(PathBuf::from(other));
                    }
                    other => return Err(format!("Unknown option for reproduce: '{other}'")),
                }
            }
            let obligation = obligation
                .ok_or_else(|| "reproduce expects exactly one obligation file.".to_string())?;
            return Ok(Invocation::Reproduce { obligation, out });
        }
        Some(&"audit") => {
            let mut lang: Option<String> = None;
            let mut path: Option<PathBuf> = None;
            let mut out: Option<PathBuf> = None;
            let mut cursor = index + 1;
            while let Some(arg) = rest.get(cursor) {
                cursor += 1;
                match *arg {
                    "--lang" | "-l" => {
                        let v = rest.get(cursor).ok_or_else(|| "--lang expects a language".to_string())?;
                        cursor += 1;
                        lang = Some(v.to_string());
                    }
                    "--out" | "-o" => {
                        let v = rest.get(cursor).ok_or_else(|| "--out expects a file path".to_string())?;
                        cursor += 1;
                        out = Some(PathBuf::from(v));
                    }
                    // Accepted so CI files can state the policy; audits are always audit-only.
                    "--audit-only" => {}
                    other if !other.starts_with('-') => path = Some(PathBuf::from(other)),
                    other => return Err(format!("Unknown option for audit: '{other}'")),
                }
            }
            let lang = lang.ok_or_else(|| {
                format!("audit expects --lang <{}>.", crate::langaudit::LANGS.join("|"))
            })?;
            let path = path.ok_or_else(|| "audit expects a path to scan.".to_string())?;
            return Ok(Invocation::Audit { lang, path, out });
        }
        Some(&"heal") => {            let mut report = PathBuf::from("report.json");
            let mut out = None;
            let mut cursor = index + 1;
            while let Some(arg) = rest.get(cursor) {
                cursor += 1;
                match *arg {
                    "--out" | "-o" => {
                        let val = rest
                            .get(cursor)
                            .ok_or_else(|| "--out expects a file or directory path".to_string())?;
                        cursor += 1;
                        out = Some(PathBuf::from(val));
                    }
                    other if !other.starts_with('-') => {
                        report = PathBuf::from(other);
                    }
                    other => return Err(format!("Unknown option for heal: '{other}'")),
                }
            }
            return Ok(Invocation::Heal { report, out });
        }
        Some(&"schema") => {
            let target = rest.get(index + 1).copied().unwrap_or("report").to_string();
            return Ok(Invocation::Schema { target });
        }
        Some(&"testgen") => {
            let mut report = PathBuf::from("report.json");
            let mut out = None;
            let mut cursor = index + 1;
            while let Some(arg) = rest.get(cursor) {
                cursor += 1;
                match *arg {
                    "--out" | "-o" => {
                        let val = rest
                            .get(cursor)
                            .ok_or_else(|| "--out expects a file path".to_string())?;
                        cursor += 1;
                        out = Some(PathBuf::from(val));
                    }
                    other if !other.starts_with('-') => {
                        report = PathBuf::from(other);
                    }
                    other => return Err(format!("Unknown option for testgen: '{other}'")),
                }
            }
            return Ok(Invocation::TestGen { report, out });
        }
        // `keller --help` and `keller --version`: the same two flags, one word
        // further out. Answering them here keeps the message about the flag the
        // person typed rather than calling it an unknown command.
        Some(&"--help") | Some(&"-h") => return Ok(Invocation::Help),
        Some(&"--version") | Some(&"-V") => return Ok(Invocation::Version),
        Some(other) => return Err(format!("Unknown command '{other}'.")),
        // A bare `keller`: there is nothing to run a default verification against,
        // so show what is possible instead of starting a run nobody asked for.
        None => return Ok(Invocation::Help),
    }

    let mut options = base;
    if watch_mode {
        options.watch = true;
    }
    while let Some(argument) = rest.get(index) {
        index += 1;
        match *argument {
            "--watch" => options.watch = true,
            "--deny-warn" => options.deny_warn = true,
            "--strict-coverage" => options.strict_coverage = true,
            // A named contract rather than two flags a reader has to look up. The
            // flags that follow can still tighten or loosen it, so a profile is a
            // starting point and not a mode that ignores what was typed next to it.
            "--profile" => {
                let raw = take_value(&rest, &mut index, "--profile")?;
                match raw {
                    "advisory" => {
                        options.deny_warn = false;
                        options.strict_coverage = false;
                    }
                    "strict" => {
                        options.deny_warn = true;
                        options.strict_coverage = true;
                    }
                    other => {
                        return Err(format!(
                            "Unknown profile '{other}'. Expected `advisory` or `strict`."
                        ));
                    }
                }
                options.gate_profile = Some(raw.to_string());
            }
            "--only" => options.only = Some(take_value(&rest, &mut index, "--only")?.to_string()),
            "--solver" => {
                let s = take_value(&rest, &mut index, "--solver")?;
                options.solver = Some(s.to_string());
            }
            "--python" => {
                options.error_format = Some("python".to_string());
            }
            "--lang" => {
                let lang = take_value(&rest, &mut index, "--lang")?;
                options.error_format = Some(lang.to_string());
            }
            "--max-findings" => {
                let raw = take_value(&rest, &mut index, "--max-findings")?;
                // `0` is refused rather than read as "list nothing": a cap of zero
                // shows no finding at all, which is never what someone wants to
                // read and is a silent way to hide a counterexample from a log.
                let count = raw.parse::<usize>().map_err(|_| {
                    format!("--max-findings expects a positive count, got '{raw}'.")
                })?;
                if count == 0 {
                    return Err(
                        "--max-findings must be at least 1; omit it to list every finding."
                            .to_string(),
                    );
                }
                options.max_findings = Some(count);
            }
            "--no-cache" => options.no_cache = true,
            "--verify-deps" => options.verify_deps = true,
            "--release" => options.release = true,
            "--keep-runs" => {
                let raw = take_value(&rest, &mut index, "--keep-runs")?;
                options.keep_runs = Some(raw.parse::<usize>().map_err(|_| {
                    format!("--keep-runs expects a nonnegative count, got '{raw}'.")
                })?);
            }
            "--json" => options.json = true,
            "--quiet" => options.quiet = true,
            "--help" | "-h" => return Ok(Invocation::Help),
            "--version" | "-V" => return Ok(Invocation::Version),
            "--sarif" => {
                options.sarif = Some(PathBuf::from(take_value(&rest, &mut index, "--sarif")?))
            }
            "--html" => {
                options.html = Some(PathBuf::from(take_value(&rest, &mut index, "--html")?))
            }
            "--error-format" => {
                let val = take_value(&rest, &mut index, "--error-format")?;
                if val != "human" && val != "json" {
                    return Err(format!(
                        "Unknown error-format '{val}'. Expected `human` or `json`."
                    ));
                }
                options.error_format = Some(val.to_string());
            }
            "--baseline" => {
                options.baseline = Some(PathBuf::from(take_value(&rest, &mut index, "--baseline")?))
            }
            "--write-baseline" => {
                options.write_baseline = Some(PathBuf::from(take_value(
                    &rest,
                    &mut index,
                    "--write-baseline",
                )?))
            }
            "--solver-timeout" => {
                let raw = take_value(&rest, &mut index, "--solver-timeout")?;
                let ms = raw.parse::<u64>().map_err(|_| {
                    format!("--solver-timeout expects whole milliseconds, got '{raw}'.")
                })?;
                // Kept, not filtered: `0` is how a caller asks for no limit now that
                // a default budget exists. Omitting the flag no longer means "no
                // limit", it means "use the default", and the two have to differ or
                // there is no way to say either one.
                options.solver_timeout_ms = Some(ms);
            }
            "--run-timeout" => {
                let raw = take_value(&rest, &mut index, "--run-timeout")?;
                let seconds = raw
                    .parse::<u64>()
                    .map_err(|_| format!("--run-timeout expects whole seconds, got '{raw}'."))?;
                // Kept for the same reason `--solver-timeout` keeps zero: silence
                // means the default cap, `0` means no cap, and a caller who wants a
                // linear run to finish has to be able to say so.
                options.run_timeout_seconds = Some(seconds);
            }
            // Refused rather than ignored: a misspelled flag that silently leaves
            // the stricter gate switched off is worse than no flag at all.
            other => {
                let hint = if other.starts_with("--deny") {
                    " Did you mean `--deny-warn`?"
                } else {
                    ""
                };
                return Err(format!("Unknown option '{other}'.{hint}"));
            }
        }
    }

    if options.baseline.is_some() && options.write_baseline.is_some() {
        return Err(
            "--baseline reads a baseline while --write-baseline records one; pick one.".to_string(),
        );
    }

    Ok(Invocation::Verify(options))
}

/// The value of a flag that needs one, or an explanation of what is missing.
fn take_value<'a>(rest: &[&'a str], index: &mut usize, flag: &str) -> Result<&'a str, String> {
    let value = rest
        .get(*index)
        .copied()
        .ok_or_else(|| format!("{flag} needs a value."))?;
    *index += 1;
    Ok(value)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let shared_report = Arc::new(Mutex::new(SharedReport::default()));

    if args.iter().any(|a| a == "--version") && !args.iter().any(|a| a == "rustc") {
        println!(
            "{} {} ({})",
            TOOL_NAME,
            version(),
            crate::compiler::this_build().describe()
        );
        std::process::exit(EXIT_OK);
    }

    if args.len() == 1 || (args.len() > 1 && is_command_invocation(&args[1])) {
        let is_verify = args.iter().any(|a| a == "verify" || a == "watch");
        let parsed = if is_verify {
            parse_args_for_run(&args)
        } else {
            parse_args(&args)
        };
        match parsed {
            Ok(Invocation::Verify(options)) => run_verify(&options),
            Ok(Invocation::Explain(path)) => run_obligation(&path, false),
            Ok(Invocation::Replay(path)) => run_obligation(&path, true),
            Ok(Invocation::Init { force }) => run_init(force),
            Ok(Invocation::Schema { target }) => run_schema(&target),
            Ok(Invocation::TestGen { report, out }) => run_testgen(&report, out.as_deref()),
            Ok(Invocation::Heal { report, out }) => run_heal(&report, out.as_deref()),
            Ok(Invocation::Audit { lang, path, out }) => {
                std::process::exit(crate::langaudit::run_audit(&lang, &path, out.as_deref()))
            }
            Ok(Invocation::Reproduce { obligation, out }) => {
                run_reproduce(&obligation, out.as_deref())
            }
            Ok(Invocation::Help) => {
                println!("{USAGE}");
                std::process::exit(EXIT_OK);
            }
            Ok(Invocation::Version) => {
                // The compiler identity is part of the version: with
                // `rustc_private` a version number alone does not say which MIR
                // dialect this binary can read.
                println!(
                    "{} {} ({})",
                    TOOL_NAME,
                    version(),
                    crate::compiler::this_build().describe()
                );
                std::process::exit(EXIT_OK);
            }
            Err(message) => {
                eprintln!("[Keller-Pipeline] {message}");
                println!("{USAGE}");
                std::process::exit(EXIT_USAGE);
            }
        }
    } else {
        // Normal compiler mode: run as rustc wrapper
        let mut compiler_args = args;
        let compiler_path = if compiler_args.len() > 1 && compiler_args[1].contains("rustc") {
            Some(compiler_args.remove(1))
        } else {
            None
        };

        let is_query_only = compiler_args
            .iter()
            .any(|arg| arg.starts_with("--print") || arg == "-vV" || arg == "--version" || arg == "-V" || arg == "___");

        if is_query_only {
            let real_compiler = compiler_path
                .as_ref()
                .map(|c| {
                    if std::path::Path::new(c).is_file() {
                        std::path::PathBuf::from(c)
                    } else {
                        crate::compiler::resolve_driver_rustc().unwrap_or_else(|_| std::path::PathBuf::from(c))
                    }
                })
                .or_else(|| crate::compiler::resolve_driver_rustc().ok());

            if let Some(ref compiler) = real_compiler {
                if let Ok(status) = Command::new(compiler).args(&compiler_args[1..]).status() {
                    if let Some(code) = status.code() {
                        std::process::exit(code);
                    }
                }
            }

            if !compiler_args.iter().any(|arg| arg == "-Zunstable-options") {
                compiler_args.push("-Zunstable-options".to_string());
            }

            let mut verifier = KellerVerifier {
                report: Arc::clone(&shared_report),
            };
            // A query answered by a compiler that failed is not an answer.
            let code = rustc_driver::catch_with_exit_code(move || {
                rustc_driver::run_compiler(&compiler_args, &mut verifier)
            });
            if code != std::process::ExitCode::SUCCESS {
                std::process::exit(WRAPPER_COMPILER_FAILED);
            }
            return;
        }

        // Cargo passes `--crate-name`. Read it before the closure below moves the
        // args, so this worker can label its findings with the crate they came from.
        let crate_name =
            crate_name_from_args(&compiler_args).unwrap_or_else(|| "<unknown-crate>".to_string());

        // Ensure rustc knows the toolchain sysroot when invoked from an arbitrary workspace wrapper path
        if !compiler_args.iter().any(|arg| arg == "--sysroot") {
            if let Ok(driver_rustc) = crate::compiler::resolve_driver_rustc() {
                if let Some(toolchain_root) = driver_rustc.parent().and_then(|p| p.parent()) {
                    compiler_args.push("--sysroot".to_string());
                    compiler_args.push(toolchain_root.to_string_lossy().to_string());
                }
            }
        }

        let should_verify = std::env::var("CARGO_PRIMARY_PACKAGE").is_ok()
            || std::env::var("KELLER_VERIFY_DEPS").is_ok()
            || std::env::var("KELLER_OUTPUT_WORKER").is_err();

        if !should_verify {
            struct PassiveCallbacks;
            impl rustc_driver::Callbacks for PassiveCallbacks {}
            let mut callbacks = PassiveCallbacks;
            let compiler_exit = rustc_driver::catch_with_exit_code(move || {
                rustc_driver::run_compiler(&compiler_args, &mut callbacks)
            });
            if compiler_exit != std::process::ExitCode::SUCCESS {
                std::process::exit(WRAPPER_COMPILER_FAILED);
            }
            return;
        }

        let compiler_version = compiler_path.as_ref().and_then(|path| {
            Command::new(path)
                .arg("-vV")
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        });
        shared_report.lock().unwrap().compiler = Some(serde_json::json!({
            "rustc": compiler_path,
            "version_verbose": compiler_version,
            "arguments": compiler_args.iter().skip(1).collect::<Vec<_>>(),
        }));
        if let Err(err) = start_worker_report(&crate_name) {
            eprintln!("[Keller-Pipeline] {err}");
            std::process::exit(WRAPPER_COMPILER_FAILED);
        }

        // Run the compiler with the Keller verifier callbacks
        let mut verifier = KellerVerifier {
            report: Arc::clone(&shared_report),
        };
        let compiler_exit = rustc_driver::catch_with_exit_code(move || {
            rustc_driver::run_compiler(&compiler_args, &mut verifier)
        });

        // Failed compilers leave only a start receipt, never a completion.
        if compiler_exit == std::process::ExitCode::SUCCESS {
            if let Err(err) = write_worker_report(&crate_name, &shared_report) {
                eprintln!("[Keller-Pipeline] {err}");
                std::process::exit(WRAPPER_COMPILER_FAILED);
            }
        }

        // A compiler that failed must not look like a clean run.
        //
        // Cargo sees only *this* wrapper's exit status, and the status used to be
        // dropped on the floor here. A crate the compiler rejected therefore came
        // out as exit 0 with an empty `report.json` — the analysis had found no
        // assertions in a body rustc refused to compile, and "no findings" is
        // indistinguishable from "nothing to check". `catch_with_exit_code` is
        // what tells the two apart, and it is propagated *before* the gate: a
        // verdict about code that did not compile is not a verdict.
        //
        // This also makes the wrapper agree with `cargo build`. The verifier reads
        // `optimized_mir` for every function, which forces runs of the MIR passes
        // that `cargo check` may skip — so a deny-by-default lint like
        // `arithmetic_overflow` is raised here for code `cargo check` accepts and
        // `cargo build` rejects. The wrapper follows the build.
        if compiler_exit != std::process::ExitCode::SUCCESS {
            std::process::exit(WRAPPER_COMPILER_FAILED);
        }

        if check_failures(&shared_report) {
            std::process::exit(EXIT_GATE);
        }
    }
}

/// Drive `cargo check` with this binary as the rustc wrapper, then gate on the
/// merged findings.
///
/// Every exit from here is one of the documented codes: a verdict, a gate
/// breach, a command line that does not make sense, or a setup that did not run.
/// A setup failure is never reported as a verdict — "could not check" and
/// "checked and found a counterexample" send the reader to different places.
fn run_single_verify(options: &VerifyOptions) -> i32 {
    let current_dir = std::env::current_dir().expect("Failed to detect active working directory");
    if options.error_format.as_deref() == Some("python") || crate::python::detect_python_project(&current_dir) {
        return crate::python::run_python_verify(options, &current_dir);
    }
    if !options.quiet && !options.json {
        println!("[Keller-Pipeline] Initiating Automated Verification Infrastructure...");
    }

    let current_exe = std::env::current_exe().expect("the running binary has a path");
    let current_dir = std::env::current_dir().expect("Failed to detect active working directory");

    let mut run =
        match run_context::RunContext::create(&current_dir, &current_exe, options.no_cache) {
            Ok(run) => run,
            Err(err) => {
                eprintln!("[Keller-Pipeline] {err}");
                return EXIT_SETUP;
            }
        };
    std::env::set_var("KELLER_REPORT_ROOT", &run.root);
    if let Err(err) = prepare_report_dir() {
        eprintln!("[Keller-Pipeline] {err}");
        return EXIT_SETUP;
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        // Clean up any stale wrapper processes left behind from earlier aborted runs
        // so that orphaned background instances cannot monopolize host RAM.
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "workspace-wrapper.exe"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }

    // The rustc recorded at build time, or the same toolchain resolved through
    // rustup when this binary was built elsewhere. `cargo` must drive this exact
    // compiler, not whichever one the project pins.
    let driver_rustc = match crate::compiler::resolve_driver_rustc() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("[Keller-Pipeline] {err}");
            return EXIT_SETUP;
        }
    };

    let mut cargo = Command::new("cargo");
    cargo
        .arg("check")
        .arg("-Zjson-target-spec")
        .arg("--target-dir")
        .arg(&run.target)
        .current_dir(&current_dir)
        .env("KELLER_REPORT_ROOT", &run.root)
        .env("RUSTC", &driver_rustc)
        .env_remove("RUSTC_WRAPPER")
        .env("RUSTC_WORKSPACE_WRAPPER", &run.wrapper)
        .env("KELLER_OUTPUT_WORKER", "1");

    // Ensure the toolchain bin and Z3 bin are on PATH so that
    // RUSTC_WORKSPACE_WRAPPER can locate rustc_driver.dll and libz3.dll at process startup on Windows.
    let discovered = crate::host_env::discover(&current_dir);
    let mut path_dirs = discovered.path_additions.clone();
    if let Some(toolchain_bin) = driver_rustc.parent() {
        if !path_dirs.iter().any(|p| p == toolchain_bin) {
            path_dirs.insert(0, toolchain_bin.to_path_buf());
        }
    }
    let z3_fallback = std::path::Path::new(r"C:\z3\bin");
    if z3_fallback.is_dir() && !path_dirs.iter().any(|p| p == z3_fallback) {
        path_dirs.push(z3_fallback.to_path_buf());
    }
    if !path_dirs.is_empty() {
        let additions = crate::host_env::join_paths(&path_dirs, crate::host_env::Flavour::Native);
        let current_path = std::env::var("PATH").unwrap_or_default();
        let sep = if cfg!(windows) { ";" } else { ":" };
        cargo.env("PATH", format!("{additions}{sep}{current_path}"));
    }

    #[cfg(windows)]
    inject_msvc_environment(&mut cargo);

    // Bound parallel compilation jobs on laptops/workstations to prevent memory exhaustion
    // caused by too many concurrent Z3 solver instances.
    let jobs = match std::env::var("KELLER_JOBS") {
        Ok(raw) => raw.parse::<usize>().ok(),
        Err(_) => {
            let cpus = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            Some((cpus / 2).max(1).min(4))
        }
    };
    if let Some(j) = jobs {
        cargo.arg(format!("-j{j}"));
    }

    // `cargo` is the rustup shim, and the shim follows the toolchain the project
    // under verification pins. A project on `stable` then refuses the `-Z` flag
    // before a single assertion is lowered, which reads as a setup failure on a
    // crate that compiles perfectly well. The verifier analyses the MIR one exact
    // compiler produces, so that compiler is the one cargo must drive.
    if let Some(toolchain) = driver_toolchain() {
        cargo.env("RUSTUP_TOOLCHAIN", toolchain);
    }

    // Before cargo runs: a toolchain that is not installed is a missing
    // prerequisite, not a verdict about the crate under verification.
    if let Err(err) = require_pinned_toolchain() {
        eprintln!("[Keller-Pipeline] {err}");
        return EXIT_SETUP;
    }

    // The worker processes cannot see this process's arguments, so the flags they
    // need travel in the environment. `--json` implies quiet: a machine reading
    // stdout must get the JSON and nothing else.
    if options.quiet || options.json {
        cargo.env("KELLER_QUIET", "1");
    }
    if options.no_cache {
        cargo.env("KELLER_NO_CACHE", "1");
    }
    if options.verify_deps {
        cargo.env("KELLER_VERIFY_DEPS", "1");
    }
    if options.release {
        cargo.arg("--release");
        cargo.env("KELLER_RELEASE", "1");
    }
    if let Some(ms) = options.solver_timeout_ms {
        cargo.env("KELLER_SOLVER_TIMEOUT_MS", ms.to_string());
    }
    // The per-verdict lines are printed by the workers, so the listing cap has to
    // reach them; the merge applies the same cap to the summary.
    if let Some(limit) = options.max_findings {
        cargo.env("KELLER_MAX_FINDINGS", limit.to_string());
    }

    // The analysis is bounded by default, and the bound is the difference between a
    // CI job that fails and a CI job that has to be killed by hand.
    //
    // A run that stops making progress is not bounded by `--solver-timeout`: the
    // solver is linked in-process, and a CPU-bound Z3 call can run past its budget
    // (K3; `op-gate` wedges on the default budget too). What a run can promise without that cause fixed is
    // that it stops taking wall-clock time, says where it stopped, and leaves its
    // state behind as evidence.
    let cap = match options.run_timeout_seconds {
        Some(0) => None,
        Some(seconds) => Some(std::time::Duration::from_secs(seconds)),
        None => match crate::report::DEFAULT_RUN_TIMEOUT_SECONDS {
            0 => None,
            s => Some(std::time::Duration::from_secs(s)),
        },
    };

    let build_started = std::time::Instant::now();
    // POSIX: a process group of its own, so the kill below reaches the wrappers
    // cargo started and not just cargo. Windows has no equivalent to set here;
    // `taskkill /T` walks the parent chain instead.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cargo.process_group(0);
    }
    let mut child = match cargo.spawn() {
        Ok(child) => child,
        Err(err) => {
            eprintln!("[Keller-Pipeline] Critical Error: could not run cargo ({err}).");
            return EXIT_SETUP;
        }
    };
    let status = match wait_bounded(&mut child, cap) {
        Ok(WaitOutcome::Exited(status)) => status,
        Ok(WaitOutcome::CapFired) => {
            let cap_seconds = cap.map(|cap| cap.as_secs()).unwrap_or(0);
            let tree = terminate_process_tree(child.id());
            if tree.is_err() {
                // A failed tree kill must still leave the direct child dead: the run
                // directory cannot be cleaned and the cache lock cannot be released
                // while a process holds them, and the *next* run is what pays for
                // that.
                let _ = child.kill();
            }
            let _ = child.wait();
            if let Err(err) = tree {
                eprintln!(
                    "[Keller-Pipeline] Could not kill the whole process tree: {err}. The direct child was terminated instead."
                );
            }
            let in_flight = crates_without_completion(&run.root);
            if let Err(err) = run.finish_build() {
                eprintln!("[Keller-Pipeline] Could not finalize build: {err}");
                return EXIT_SETUP;
            }
            // Evidence before the message: the message names where it is, so it has
            // to exist by the time anyone reads that sentence. A run that reached no
            // verdict is not published over the last completed report -- the
            // compatibility view keeps the last run that actually finished.
            if let Err(err) = run.finalize_timeout(options, cap_seconds, &in_flight) {
                eprintln!("[Keller-Pipeline] Could not record the stopped run: {err}");
                return EXIT_SETUP;
            }
            let flight = if in_flight.is_empty() {
                "no crate receipt was in flight".to_string()
            } else {
                format!("in flight: {}", in_flight.join(", "))
            };
            eprintln!(
                "[Keller-Pipeline] Stopped at the --run-timeout cap ({cap_seconds} s): the analysis did not finish, so this run reached no verdict and exits {EXIT_SETUP} ({flight}). What it did reach is recorded as status 'run-timeout' at '{}'; the last completed report was left in place. Raise or remove the cap with `--run-timeout <SECONDS>` (`0` removes it).",
                run.root.display()
            );
            return EXIT_SETUP;
        }
        Err(err) => {
            eprintln!("[Keller-Pipeline] Critical Error: could not run cargo ({err}).");
            return EXIT_SETUP;
        }
    };

    let build_elapsed = build_started.elapsed();
    if let Err(err) = run.finish_build() {
        eprintln!("[Keller-Pipeline] Could not finalize build: {err}");
        return EXIT_SETUP;
    }
    if !status.success() {
        eprintln!(
            "[Keller-Pipeline] Critical Error: `cargo check` did not complete, so no verification verdict was reached."
        );
        return EXIT_SETUP;
    }

    // Discovery supplies replay paths. The merge replaces compiler identity with
    // actual worker receipts; a directory's rustup override is not evidence of
    // which compiler Cargo invoked.
    let mut options = options.clone();
    options.env = Some(environment_of(&current_dir));
    options.run_id = Some(run.id.clone());
    options.run_directory = Some(run.root.clone());
    options.build_elapsed_ms = Some(build_elapsed.as_millis() as u64);

    // Exit-code contract: FAIL events make the pipeline exit 1, and under
    // `--deny-warn` so does a WARN — which now covers a lost path sensitivity
    // as well as an undecided verdict. Each worker is non-fatal and writes its
    // own report; merging them is what makes the gate reflect the whole
    // workspace rather than the last crate to finish.
    let mut outcome = match merge_worker_reports(&options) {
        Ok(outcome) => outcome,
        Err(err) => {
            eprintln!("[Keller-Pipeline] Incomplete verification: {err}");
            return EXIT_SETUP;
        }
    };
    if let Some(err) = outcome.baseline_error.clone() {
        outcome.run_status = "baseline-usage-error".to_string();
        if let Err(finalize_err) =
            run.finalize_gate(&options, &outcome, Some("baseline-usage-error"))
        {
            eprintln!("[Keller-Pipeline] Could not finalize run evidence: {finalize_err}");
            return EXIT_SETUP;
        }
        if let Err(prune_err) = run.prune(&current_dir, options.keep_runs.unwrap_or(10)) {
            eprintln!("[Keller-Pipeline] Could not prune retained runs: {prune_err}");
            return EXIT_SETUP;
        }
        if let Some(path) = &options.sarif {
            if let Err(sarif_err) = write_sarif(
                path,
                listed_findings(&outcome.gated, outcome.max_findings),
                &outcome.coverage.totals,
            ) {
                eprintln!("[Keller-Pipeline] Could not write SARIF: {sarif_err}");
                return EXIT_SETUP;
            }
        }
        if let Err(publish_err) = run.publish(&current_dir) {
            eprintln!(
                "[Keller-Pipeline] Evidence retained at '{}': {publish_err}",
                run.root.display()
            );
            return EXIT_SETUP;
        }
        eprintln!("[Keller-Pipeline] {err}");
        return EXIT_USAGE;
    }
    if let Some(path) = &options.sarif {
        if let Err(err) = write_sarif(
            path,
            listed_findings(&outcome.gated, outcome.max_findings),
            &outcome.coverage.totals,
        ) {
            eprintln!("[Keller-Pipeline] {err}");
            return EXIT_SETUP;
        }
    }
    // Interactive HTML verification dashboard
    let _ =
        crate::html_report::write_html_report(&run.root.join("report.html"), &outcome, &options);
    if let Some(path) = &options.html {
        if let Err(err) = crate::html_report::write_html_report(path, &outcome, &options) {
            eprintln!("[Keller-Pipeline] Could not write HTML report: {err}");
            return EXIT_SETUP;
        } else if !options.quiet && !options.json {
            println!(
                "[Keller-Pipeline] Interactive HTML dashboard generated at '{}'.",
                path.display()
            );
        }
    }
    // Compiler-style diagnostic stream annotations
    if options.error_format.as_deref() == Some("json") {
        crate::diagnostics::emit_json_diagnostics(&outcome.gated);
    }
    // Coverage is a property of the run, not of one crate: one line, printed once,
    // and printed before the verdict so a reader sees how much of the program the
    // verdict is about.
    print_coverage_headline(&outcome.coverage.totals);
    if outcome.filtered_out > 0 {
        eprintln!(
            "[Keller-Pipeline] --only '{}': {} finding(s) outside the filter were neither reported nor gated, so this run's scope is partial.",
            options.only.as_deref().unwrap_or_default(),
            outcome.filtered_out
        );
    }
    if outcome.not_listed > 0 {
        eprintln!(
            "[Keller-Pipeline] --max-findings {}: listing {} of {} finding(s). The gate judged every finding, so this cap changes what you read, not what passed.",
            options.max_findings.unwrap_or(0),
            outcome.findings.len() - outcome.not_listed,
            outcome.findings.len()
        );
    }
    outcome.run_status = if options.write_baseline.is_some() {
        "baseline-recording".to_string()
    } else {
        outcome.run_status.clone()
    };
    if let Err(err) = run.finalize_gate(&options, &outcome, None) {
        eprintln!("[Keller-Pipeline] Could not finalize run evidence: {err}");
        return EXIT_SETUP;
    }
    if options.keep_runs != Some(0) {
        if let Err(err) = run.prune(&current_dir, options.keep_runs.unwrap_or(10)) {
            eprintln!("[Keller-Pipeline] Could not prune old run directories: {err}");
            return EXIT_SETUP;
        }
    }
    if let Err(err) = run.publish(&current_dir) {
        eprintln!(
            "[Keller-Pipeline] Evidence retained at '{}'; compatibility publication failed: {err}",
            run.root.display()
        );
        return EXIT_SETUP;
    } else if !options.quiet && !options.json {
        println!("[Keller-Pipeline] Verification reports bundled at 'keller-report/'.");
    }
    if options.strict_coverage && outcome.coverage.totals.without_verdict > 0 {
        eprintln!(
            "[Keller-Pipeline] Strict coverage failed: {} assertions have no verdict.",
            outcome.coverage.totals.without_verdict
        );
        return EXIT_GATE;
    }

    if let Some(err) = &outcome.baseline_error {
        // A baseline that cannot be read is a usage problem. Carrying on with an
        // empty baseline would report every long-known finding as new.
        eprintln!("[Keller-Pipeline] {err}");
        return EXIT_USAGE;
    }

    if let Some(path) = &options.write_baseline {
        match write_baseline(path, &outcome.findings) {
            Ok(count) => {
                eprintln!(
                    "[Keller-Pipeline] Recorded {count} finding(s) in '{}'.",
                    path.display()
                );
                if options.keep_runs != Some(0) {
                    if let Err(err) = run.prune(&current_dir, options.keep_runs.unwrap_or(10)) {
                        eprintln!("[Keller-Pipeline] Could not prune retained runs: {err}");
                        return EXIT_SETUP;
                    }
                }
                if let Err(err) = run.publish(&current_dir) {
                    eprintln!(
                        "[Keller-Pipeline] Evidence retained at '{}': {err}",
                        run.root.display()
                    );
                    return EXIT_SETUP;
                }
            }
            Err(err) => {
                eprintln!("[Keller-Pipeline] {err}");
                return EXIT_SETUP;
            }
        }
        // Recording what is already known is not a verdict, and the adoption path
        // depends on it: record first, then start gating.
        return EXIT_OK;
    }

    if options.json {
        match summary_json(&outcome) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("[Keller-Pipeline] {err}");
                return EXIT_SETUP;
            }
        }
    } else {
        // A5, in the one place someone reading a terminal between two builds will
        // see it: printed on the way out of every run, pass or fail, so the claim
        // and the verdict are read together. Under `--json` the summary already
        // carries it as a field and the console stream stays machine-clean.
        println!(
            "[Keller-Pipeline] Claim checked: {} — {}",
            report::analysis_profile(options.release),
            report::analysis_profile_note(options.release)
        );
    }

    match &outcome.breach {
        None => {
            if !options.json {
                println!("[Keller-Pipeline] Formal Verification Completed Successfully.");
            }
            EXIT_OK
        }
        Some(breach) => {
            let mut message = breach.to_string();
            if outcome.suppressed > 0 {
                message.push_str(&format!(
                    " ({} finding(s) match the baseline and were not gated)",
                    outcome.suppressed
                ));
            }
            if options.strict_coverage && !options.deny_warn {
                message = message.replace("--deny-warn", "--strict-coverage");
            }
            eprintln!(
                "[Keller-Pipeline] Verification FAILED: {message}. See report.json in the working directory."
            );
            EXIT_GATE
        }
    }
}

fn collect_mtimes(root: &Path) -> std::collections::HashMap<PathBuf, std::time::SystemTime> {
    let mut map = std::collections::HashMap::new();
    fn scan_dir(dir: &Path, map: &mut std::collections::HashMap<PathBuf, std::time::SystemTime>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if name_str.starts_with('.') || name_str == "target" {
                continue;
            }
            if path.is_dir() {
                scan_dir(&path, map);
            } else if path
                .extension()
                .map_or(false, |ext| ext == "rs" || ext == "toml")
            {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        map.insert(path, mtime);
                    }
                }
            }
        }
    }
    scan_dir(root, &mut map);
    map
}

fn run_watch(options: &VerifyOptions) {
    println!("[Keller-Watch] Starting continuous formal verification mode...");
    let code = run_single_verify(options);
    println!(
        "[Keller-Watch] Initial verification completed (code: {code}). Watching for workspace changes... (Ctrl+C to stop)"
    );

    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut last_snapshot = collect_mtimes(&current_dir);

    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let new_snapshot = collect_mtimes(&current_dir);
        let mut changed_file = None;

        for (path, time) in &new_snapshot {
            if let Some(old_time) = last_snapshot.get(path) {
                if old_time != time {
                    changed_file = Some(path.clone());
                    break;
                }
            } else {
                changed_file = Some(path.clone());
                break;
            }
        }

        if changed_file.is_none() && new_snapshot.len() != last_snapshot.len() {
            changed_file = Some(PathBuf::from("<workspace files modified>"));
        }

        if let Some(path) = changed_file {
            std::thread::sleep(std::time::Duration::from_millis(200));
            last_snapshot = collect_mtimes(&current_dir);
            println!(
                "\n[Keller-Watch] Change detected: {}. Re-verifying...",
                path.display()
            );
            let code = run_single_verify(options);
            println!(
                "[Keller-Watch] Verification cycle finished (exit code {code}). Watching for changes..."
            );
        }
    }
}

fn run_verify(options: &VerifyOptions) {
    if options.watch {
        run_watch(options);
    } else {
        let code = run_single_verify(options);
        std::process::exit(code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Invocation, String> {
        let owned: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
        parse_args(&owned)
    }

    fn options(args: &[&str]) -> VerifyOptions {
        match parse(args) {
            Ok(Invocation::Verify(options)) => options,
            other => panic!("expected a verify invocation, got {other:?}"),
        }
    }

    #[test]
    fn help_and_version_work_before_and_after_the_subcommand() {
        for args in [
            vec!["keller-verify", "--help"],
            vec!["keller-verify", "-h"],
            vec!["keller-verify", "keller", "verify", "--help"],
            vec!["keller-verify", "keller", "--help"],
        ] {
            assert_eq!(parse(&args), Ok(Invocation::Help), "for {args:?}");
        }
        for args in [
            vec!["keller-verify", "--version"],
            vec!["keller-verify", "-V"],
            vec!["keller-verify", "keller", "verify", "--version"],
        ] {
            assert_eq!(parse(&args), Ok(Invocation::Version), "for {args:?}");
        }
    }

    #[test]
    fn strict_coverage_and_obligation_commands_are_parsed() {
        assert!(
            options(&["keller-verify", "keller", "verify", "--strict-coverage"]).strict_coverage
        );
        assert_eq!(
            parse(&["keller-verify", "keller", "explain", "goal.smt2"]),
            Ok(Invocation::Explain(PathBuf::from("goal.smt2")))
        );
        assert_eq!(
            parse(&["keller-verify", "keller", "replay", "goal.smt2"]),
            Ok(Invocation::Replay(PathBuf::from("goal.smt2")))
        );
        assert_eq!(
            parse(&["keller-verify", "keller", "schema"]),
            Ok(Invocation::Schema {
                target: "report".to_string()
            })
        );
        assert_eq!(
            parse(&["keller-verify", "keller", "schema", "run"]),
            Ok(Invocation::Schema {
                target: "run".to_string()
            })
        );
        assert_eq!(
            parse(&["keller-verify", "keller", "testgen"]),
            Ok(Invocation::TestGen {
                report: PathBuf::from("report.json"),
                out: None,
            })
        );
        assert_eq!(
            parse(&[
                "keller-verify",
                "keller",
                "testgen",
                "custom.json",
                "--out",
                "repro.rs"
            ]),
            Ok(Invocation::TestGen {
                report: PathBuf::from("custom.json"),
                out: Some(PathBuf::from("repro.rs")),
            })
        );
        assert!(parse(&["keller-verify", "keller", "replay"]).is_err());
        assert!(parse(&["keller-verify", "keller", "explain", "a", "b"]).is_err());
    }

    #[test]
    fn a_bare_command_shows_the_usage_rather_than_running_something() {
        assert_eq!(parse(&["keller-verify", "keller"]), Ok(Invocation::Help));
    }

    #[test]
    fn verify_defaults_to_gating_only_on_counterexamples() {
        // The default must not fail a build for an undecided verdict: that is what
        // `--deny-warn` is for, and a default that already did it would make the
        // flag unimplementable as a deliberate tightening.
        let options = options(&["keller-verify", "keller", "verify"]);
        assert_eq!(options, VerifyOptions::default());
        assert!(!options.deny_warn);
    }

    #[test]
    fn every_documented_flag_is_parsed() {
        let options = options(&[
            "keller-verify",
            "keller",
            "verify",
            "--watch",
            "--deny-warn",
            "--no-cache",
            "--verify-deps",
            "--release",
            "--keep-runs",
            "10",
            "--json",
            "--quiet",
            "--solver-timeout",
            "250",
            "--run-timeout",
            "90",
            "--sarif",
            "out.sarif",
            "--html",
            "out.html",
            "--error-format",
            "json",
            "--baseline",
            "base.json",
        ]);
        assert!(options.watch);
        assert!(options.deny_warn);
        assert!(options.no_cache);
        assert!(options.verify_deps);
        assert!(options.release);
        assert_eq!(options.keep_runs, Some(10));
        assert!(options.json);
        assert!(options.quiet);
        assert_eq!(options.solver_timeout_ms, Some(250));
        assert_eq!(options.run_timeout_seconds, Some(90));
        assert_eq!(options.sarif, Some(PathBuf::from("out.sarif")));
        assert_eq!(options.html, Some(PathBuf::from("out.html")));
        assert_eq!(options.error_format, Some("json".to_string()));
        assert_eq!(options.baseline, Some(PathBuf::from("base.json")));
        assert_eq!(options.write_baseline, None);
    }

    #[test]
    fn watch_subcommand_parses_as_watch_mode() {
        let options = options(&["keller-verify", "keller", "watch", "--release"]);
        assert!(options.watch);
        assert!(options.release);
    }

    #[test]
    fn a_solver_timeout_of_zero_means_no_budget_and_silence_means_the_default() {
        // The two have to differ, and they do: an absent flag is `None`, which the
        // worker turns into the default budget, while a computed `0` from a script
        // is the one way to ask for no limit at all. Before a default existed both
        // spellings meant "no limit", which is why a run could wedge a CI job.
        assert_eq!(
            options(&["keller-verify", "keller", "verify", "--solver-timeout", "0"])
                .solver_timeout_ms,
            Some(0)
        );
        assert_eq!(
            options(&["keller-verify", "keller", "verify"]).solver_timeout_ms,
            None
        );
    }

    #[test]
    fn a_run_cap_of_zero_removes_it_and_silence_means_the_default() {
        // The same three-way distinction as the solver budget, and it has to hold for
        // the same reason: a cap a caller cannot remove is a cap that stops honest
        // long runs, and a cap a caller cannot set is not a bound.
        assert_eq!(
            options(&["keller-verify", "keller", "verify", "--run-timeout", "0"])
                .run_timeout_seconds,
            Some(0)
        );
        assert_eq!(
            options(&["keller-verify", "keller", "verify", "--run-timeout", "90"])
                .run_timeout_seconds,
            Some(90)
        );
        assert_eq!(
            options(&["keller-verify", "keller", "verify"]).run_timeout_seconds,
            None
        );
        // Seconds, not milliseconds: a value that is refused is better than one that
        // silently runs for a thousandth of what was asked for.
        assert!(parse(&["keller-verify", "keller", "verify", "--run-timeout", "1.5"]).is_err());
        assert!(parse(&["keller-verify", "keller", "verify", "--run-timeout"]).is_err());
    }

    #[test]
    fn a_named_gate_profile_sets_both_flags_and_the_flags_still_win() {
        let strict = options(&["keller-verify", "keller", "verify", "--profile", "strict"]);
        assert!(strict.deny_warn && strict.strict_coverage);
        assert_eq!(strict.gate_profile.as_deref(), Some("strict"));

        let advisory = options(&["keller-verify", "keller", "verify", "--profile", "advisory"]);
        assert!(!advisory.deny_warn && !advisory.strict_coverage);

        // A profile is a starting point, not a mode that ignores what follows it.
        let loosened = options(&[
            "keller-verify",
            "keller",
            "verify",
            "--profile",
            "strict",
            "--no-cache",
        ]);
        assert!(loosened.deny_warn && loosened.strict_coverage);
        assert!(loosened.no_cache);

        let err = parse(&["keller-verify", "keller", "verify", "--profile", "stric"])
            .expect_err("an unknown profile is refused");
        assert!(err.contains("stric"), "got {err}");
    }

    #[test]
    fn the_listing_flags_parse_and_a_cap_of_zero_is_refused() {
        let options = options(&[
            "keller-verify",
            "keller",
            "verify",
            "--only",
            "crypto::",
            "--max-findings",
            "10",
        ]);
        assert_eq!(options.only.as_deref(), Some("crypto::"));
        assert_eq!(options.max_findings, Some(10));

        // A cap of zero would list nothing, which is a silent way to hide a
        // counterexample from a log rather than a narrowing of the report.
        let err = parse(&["keller-verify", "keller", "verify", "--max-findings", "0"])
            .expect_err("a cap of zero is refused");
        assert!(err.contains("at least 1"), "got {err}");
    }

    #[test]
    fn the_pinned_toolchain_is_matched_by_name_and_not_by_substring() {
        let listing = "stable-x86_64-pc-windows-gnu (default)\nnightly-2026-07-24-x86_64-pc-windows-gnu (active)\n";
        assert!(toolchain_is_installed(
            "nightly-2026-07-24-x86_64-pc-windows-gnu",
            listing
        ));
        assert!(!toolchain_is_installed("nightly-2026-07-24", listing));
        assert!(!toolchain_is_installed("nightly", listing));
        assert!(!toolchain_is_installed("nightly-2026-08-01", listing));
        // A bare `stable` is deliberately *not* a match for the triple'd entry.
        // rustup names the toolchains directory after the resolved toolchain, which
        // is what the driver records, so a mismatch here means the two are not the
        // same install — and treating them as equal is how a missing toolchain
        // would go back to being reported as a rustc version mismatch.
        assert!(!toolchain_is_installed("stable", listing));
        // A linked toolchain has a name of its own and does match it.
        assert!(toolchain_is_installed(
            "my-linked-toolchain",
            "my-linked-toolchain\n"
        ));
        assert!(!toolchain_is_installed("stable", ""));
    }

    #[test]
    fn an_unknown_option_is_refused_and_a_near_miss_says_so() {
        let err = parse(&["keller-verify", "keller", "verify", "--deny-warns"])
            .expect_err("a misspelled flag is refused");
        assert!(err.contains("--deny-warns"), "got {err}");
        assert!(
            err.contains("--deny-warn"),
            "the suggestion must name it: {err}"
        );

        let err = parse(&["keller-verify", "keller", "verify", "--nope"])
            .expect_err("an unknown flag is refused");
        assert!(err.contains("Unknown option"), "got {err}");
    }

    #[test]
    fn flags_that_need_a_value_say_what_is_missing() {
        for flag in [
            "--sarif",
            "--baseline",
            "--write-baseline",
            "--solver-timeout",
        ] {
            let err = parse(&["keller-verify", "keller", "verify", flag])
                .expect_err("a flag without its value is refused");
            assert!(
                err.contains(flag) && err.contains("needs a value"),
                "got {err}"
            );
        }
        let err = parse(&[
            "keller-verify",
            "keller",
            "verify",
            "--solver-timeout",
            "soon",
        ])
        .expect_err("a non-numeric budget is refused");
        assert!(err.contains("whole milliseconds"), "got {err}");
    }

    #[test]
    fn recording_and_reading_a_baseline_are_mutually_exclusive() {
        // They are opposites, so accepting both would mean one of them silently
        // did nothing — and either choice would surprise someone.
        let err = parse(&[
            "keller-verify",
            "keller",
            "verify",
            "--baseline",
            "b.json",
            "--write-baseline",
            "b.json",
        ])
        .expect_err("the combination is refused");
        assert!(err.contains("pick one"), "got {err}");
    }

    #[test]
    fn an_unknown_command_is_refused() {
        let err = parse(&["keller-verify", "keller", "unknown"])
            .expect_err("unknown commands are refused");
        assert!(err.contains("unknown"), "got {err}");
    }

    #[test]
    fn init_parses_with_and_without_force_and_refuses_anything_else() {
        assert_eq!(
            parse(&["keller-verify", "keller", "init"]),
            Ok(Invocation::Init { force: false })
        );
        assert_eq!(
            parse(&["keller-verify", "keller", "init", "--force"]),
            Ok(Invocation::Init { force: true })
        );
        assert!(parse(&["keller-verify", "keller", "init", "--nope"]).is_err());
    }

    #[test]
    fn the_config_file_is_only_a_base_for_the_command_line() {
        // The rule the whole config surface rests on: what someone typed wins.
        // A config that asked for `strict` must not survive an explicit
        // `--profile advisory`, and a flag the config never mentions must be
        // untouched by it.
        let base = crate::config::base_options(
            &crate::config::parse("profile = \"strict\"\nmax_findings = 25\n").unwrap(),
        );
        let options = match parse_args_with_base(
            &[
                "keller-verify".to_string(),
                "keller".to_string(),
                "verify".to_string(),
                "--profile".to_string(),
                "advisory".to_string(),
            ],
            base.clone(),
        ) {
            Ok(Invocation::Verify(options)) => options,
            other => panic!("expected a verify invocation, got {other:?}"),
        };
        assert!(!options.deny_warn, "the flag overrode the file");
        assert!(!options.strict_coverage, "the flag overrode the file");
        assert_eq!(
            options.max_findings,
            Some(25),
            "a setting the flag did not mention still comes from the file"
        );
        assert!(
            base.deny_warn && base.strict_coverage,
            "the base itself was not mutated by parsing"
        );
    }
}

/// `keller init`: write a starter `keller.toml` in the working directory.
///
/// The file it writes changes nothing until it is edited — every setting is
/// commented out — because an initialiser that starts gating on its first run
/// would make the tool look broken in the one place a new user first tries it.
/// It refuses to replace an existing file without `--force`: the file is where a
/// team records which of their crates are gated, and overwriting that silently
/// would throw away the decisions it exists to hold.
fn run_init(force: bool) -> ! {
    let dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("[Keller-Pipeline] Could not determine the working directory: {err}");
            std::process::exit(EXIT_SETUP);
        }
    };
    let path = dir.join(config::CONFIG_FILE);
    if path.exists() && !force {
        eprintln!(
            "[Keller-Pipeline] '{}' already exists; pass --force to overwrite it. Nothing was written.",
            config::CONFIG_FILE
        );
        std::process::exit(EXIT_USAGE);
    }
    if let Err(err) = std::fs::write(&path, config::default_text()) {
        eprintln!(
            "[Keller-Pipeline] Could not write '{}': {err}",
            path.display()
        );
        std::process::exit(EXIT_SETUP);
    }
    println!(
        "Wrote '{}'. Every setting in it is commented out, so the next run behaves exactly as it does now.",
        path.display()
    );
    std::process::exit(EXIT_OK);
}

fn run_schema(target: &str) -> ! {
    match target {
        "report" | "report.json" => {
            println!("{}", include_str!("../schema/report.schema.json"));
            std::process::exit(EXIT_OK);
        }
        "run" | "run.json" | "report.run.json" => {
            println!("{}", include_str!("../schema/run.schema.json"));
            std::process::exit(EXIT_OK);
        }
        other => {
            eprintln!(
                "[Keller-Pipeline] Unknown schema target '{other}'. Supported: 'report', 'run'."
            );
            std::process::exit(EXIT_USAGE);
        }
    }
}

fn run_testgen(report_path: &Path, out_path: Option<&Path>) -> ! {
    match testgen::generate_tests_from_file(report_path) {
        Ok(code) => {
            if let Some(dest) = out_path {
                if let Err(e) = std::fs::write(dest, &code) {
                    eprintln!(
                        "[Keller-Pipeline] Failed to write test file '{}': {e}",
                        dest.display()
                    );
                    std::process::exit(EXIT_SETUP);
                }
                println!(
                    "[Keller-Pipeline] Wrote reproduction tests to '{}'.",
                    dest.display()
                );
            } else {
                print!("{code}");
            }
            std::process::exit(EXIT_OK);
        }
        Err(e) => {
            eprintln!("[Keller-Pipeline] testgen error: {e}");
            std::process::exit(EXIT_USAGE);
        }
    }
}

fn run_heal(report_path: &Path, out_path: Option<&Path>) -> ! {
    let content = match std::fs::read_to_string(report_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[Keller-Heal] Could not read report file '{}': {e}", report_path.display());
            std::process::exit(EXIT_SETUP);
        }
    };

    let events: Vec<crate::types::VerificationEvent> = match serde_json::from_str(&content) {
        Ok(ev) => ev,
        Err(_) => {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                val.get("findings")
                    .and_then(|f| serde_json::from_value(f.clone()).ok())
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    };

    let fail_events: Vec<&crate::types::VerificationEvent> = events.iter().filter(|e| e.status == "FAIL").collect();
    if fail_events.is_empty() {
        println!("[Keller-Heal] No FAIL findings requiring repair found in report.");
        std::process::exit(EXIT_OK);
    }

    println!("[Keller-Heal] Synthesizing neuro-symbolic defensive patches for {} failure(s)...", fail_events.len());
    let mut patches = Vec::new();

    for ev in fail_events {
        let mut model = std::collections::HashMap::new();
        for w in &ev.witness {
            model.insert(w.symbol.clone(), w.value.clone());
        }
        let lang = if ev.file.ends_with(".py") {
            crate::vir::heal::TargetLanguage::Python
        } else {
            crate::vir::heal::TargetLanguage::Rust
        };
        let patch = crate::vir::heal::synthesize_patch(
            &ev.file,
            ev.line.unwrap_or(1),
            &ev.defect_class,
            &model,
            lang,
            None,
        );
        patches.push(patch);
    }

    let output_text = patches.iter().map(|p| format!("=== Patch for {} (line {}) ===\n{}\n{}\n", p.file, p.line, p.patch_code, p.unified_diff)).collect::<Vec<_>>().join("\n");

    if let Some(dest) = out_path {
        if let Err(e) = std::fs::write(dest, &output_text) {
            eprintln!("[Keller-Heal] Failed to write patch file '{}': {e}", dest.display());
            std::process::exit(EXIT_SETUP);
        }
        println!("[Keller-Heal] Wrote certified defensive patches to '{}'.", dest.display());
    } else {
        println!("{output_text}");
    }

    std::process::exit(EXIT_OK);
}

/// Explain metadata without pretending an SMT model is a concrete execution.
fn run_obligation(path: &Path, replay: bool) {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("Could not read '{}': {err}", path.display());
            std::process::exit(EXIT_SETUP);
        }
    };
    let expected = match crate::report::obligation_expected(&text) {
        Ok(expected) => expected,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(EXIT_USAGE);
        }
    };
    if !replay {
        // The header is already the answer to "what did it check and where", so
        // print it as written rather than re-summarising it: a summariser is one
        // more place for the artifact and the console to disagree.
        for line in text.lines().take_while(|line| line.starts_with(';')) {
            println!("{}", line.trim_start_matches(';').trim());
        }
        println!(
            "Expected replay: {expected}. The full formula and its guard are in {}.",
            path.display()
        );
        println!(
            "Evidence is relative to the encoded model; this is not concrete-execution validation."
        );
        std::process::exit(EXIT_OK);
    }
    let discovered = host_env::discover(&std::env::current_dir().unwrap());
    let Some(z3) = discovered.z3_bin else {
        eprintln!("No runnable Z3 found; set KELLER_Z3 or Z3_HOME.");
        std::process::exit(EXIT_SETUP);
    };
    let output = match Command::new(z3)
        .arg("-smt2")
        .arg("-T:10")
        .arg(path)
        .output()
    {
        Ok(output) => output,
        Err(err) => {
            eprintln!("Could not replay obligation: {err}");
            std::process::exit(EXIT_SETUP);
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() || !stderr.trim().is_empty() {
        eprintln!("Solver rejected the obligation: {stdout}{stderr}");
        std::process::exit(EXIT_SETUP);
    }
    match crate::report::check_replay(expected, &stdout) {
        Ok(()) => {
            println!("Replay matched the recorded verdict: {expected}.");
            std::process::exit(EXIT_OK);
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(EXIT_GATE);
        }
    }
}

/// Pin a site's identity, flags and obligation, re-discharge it, and say what the
/// bundle does not reconstruct. Exit 0 when the replay matched, 1 when it did not,
/// and 2/3 when the command could not be honoured at all.
fn run_reproduce(obligation: &Path, out: Option<&Path>) -> ! {
    match crate::reproduce::run(obligation, out) {
        Ok(outcome) => {
            println!(
                "[reproduce] site: {}::{} ({}) at {} — {}",
                outcome.site.crate_name,
                outcome.site.function,
                outcome.site.block,
                if outcome.site.file.is_empty() {
                    "unknown location"
                } else {
                    &outcome.site.file
                },
                outcome.site.kind
            );
            println!(
                "[reproduce] replay: solver answered '{}', recorded '{}' — {}",
                outcome.answer,
                outcome.recorded,
                if outcome.matched {
                    "matched"
                } else {
                    "MISMATCH"
                }
            );
            println!("[reproduce] bundle: {}", outcome.bundle.display());
            println!("[reproduce] not reconstructed:");
            for item in &outcome.cannot_reconstruct {
                println!("    - {item}");
            }
            std::process::exit(if outcome.matched { EXIT_OK } else { EXIT_GATE });
        }
        Err((code, message)) => {
            eprintln!("[reproduce] {message}");
            std::process::exit(code);
        }
    }
}

/// What a bounded wait ended as.
enum WaitOutcome {
    /// Cargo finished on its own.
    Exited(std::process::ExitStatus),
    /// The cap was reached while cargo was still running.
    CapFired,
}

/// Wait for `child`, but not past `cap`.
///
/// `Child::wait` has no timeout, and a run that never returns is the one failure a
/// pipeline cannot absorb: the job costs a runner until a person kills it. Polling
/// `try_wait` is what makes the wait bounded without a signal handler or a second
/// thread, and the poll interval is the worst-case overshoot past the cap.
fn wait_bounded(
    child: &mut std::process::Child,
    cap: Option<std::time::Duration>,
) -> Result<WaitOutcome, String> {
    let deadline = cap.map(|cap| std::time::Instant::now() + cap);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(WaitOutcome::Exited(status)),
            Ok(None) => {}
            Err(err) => return Err(format!("could not wait for cargo ({err})")),
        }
        if deadline.is_some_and(|deadline| std::time::Instant::now() >= deadline) {
            return Ok(WaitOutcome::CapFired);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Kill a process and everything it started.
///
/// Killing the direct child is not enough. `cargo` has children of its own -- the
/// toolchain's cargo, one workspace wrapper per crate, rustc under those -- and the
/// wrappers are the processes doing the analysis, so a plain kill leaves the
/// analysis running with nothing waiting for it. Measured on this repository:
/// `Stop-Process` on the top-level verifier left two crates' worth of `cargo` and
/// `workspace-wrapper` processes alive for hours, which is what a sweep script's
/// timeout used to leak per attempt.
///
/// Windows walks the parent chain with `taskkill /T`; POSIX signals the child's
/// process group, which is why the spawn above puts the child in one. Its message
/// is localization-dependent, so only the exit status is read.
#[cfg(windows)]
fn terminate_process_tree(root: u32) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let output = Command::new("taskkill")
        .args(["/T", "/F", "/PID", &root.to_string()])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|err| format!("could not run taskkill ({err})"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "taskkill exited with {} and reported: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

/// See the Windows definition above for why the tree, and not the child, is killed.
#[cfg(unix)]
fn terminate_process_tree(root: u32) -> Result<(), String> {
    // Negative: every process in the group led by `root`, which the spawn put the
    // child and its descendants in.
    let rc = unsafe { libc::kill(-(root as i32), libc::SIGKILL) };
    if rc == 0 {
        return Ok(());
    }
    let err = std::io::Error::last_os_error();
    // ESRCH: the group is already gone, which is the outcome this asked for -- a
    // child that exited on its own between the cap and the signal.
    if err.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(format!("could not signal the process group ({err})"))
}

/// The crates whose workers began and never finished.
///
/// This is what "where it stopped" means for a run cut short. A worker writes
/// `<crate>-<pid>.started` when it begins and a `<crate>-<pid>.json` when it
/// completes, so a start with no completion names a crate that was in flight when
/// the cap fired -- the one detail that turns "it hung" into a reproducible report
/// without knowing the cause. The names come out of the receipts themselves, because
/// the file name is a sanitised stem and a reader needs the crate name.
fn crates_without_completion(run_root: &Path) -> Vec<String> {
    let dir = run_root.join(crate::report::REPORT_DIR);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut started = std::collections::BTreeSet::new();
    let mut completed = std::collections::BTreeSet::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("started") => {
                started.insert(name.to_string());
            }
            Some("json") => {
                completed.insert(name.to_string());
            }
            _ => {}
        }
    }
    let mut in_flight = Vec::new();
    for name in started {
        // A worker that completed wrote its own name with a `json` extension.
        let stem = name.trim_end_matches(".started");
        if completed.contains(&format!("{stem}.json")) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(dir.join(&name)) else {
            continue;
        };
        let Ok(receipt) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        if let Some(crate_name) = receipt["crate_name"].as_str() {
            in_flight.push(crate_name.to_string());
        }
    }
    in_flight.sort();
    in_flight.dedup();
    in_flight
}

/// The toolchain this build was compiled against, when rustup installed it.
///
/// Absent for a system compiler, which is also the case where there is no
/// toolchain name to select and nothing to check for.
fn driver_toolchain() -> Option<&'static str> {
    option_env!("KELLER_DRIVER_TOOLCHAIN")
}

/// Refuse to start when the pinned toolchain is not installed.
///
/// This verifier analyses the MIR of one exact nightly and links against that
/// compiler's dylibs, so a machine without it cannot run at all. The failure it
/// used to produce came out of rustc as a version mismatch, which reads as a bug
/// in the crate under verification rather than as a missing prerequisite. Checked
/// once, before `cargo` starts, and reported with the command that fixes it.
fn require_pinned_toolchain() -> Result<(), String> {
    let Some(toolchain) = driver_toolchain() else {
        return Ok(());
    };
    let listing = match Command::new("rustup").arg("toolchain").arg("list").output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).into_owned()
        }
        // No rustup, or it would not answer: not this check's business to fail the
        // run. Cargo will say what is wrong if the compiler really is unusable.
        _ => return Ok(()),
    };
    if !toolchain_is_installed(toolchain, &listing) {
        return Err(format!(
            "The toolchain this verifier was built against, '{toolchain}', is not installed, and it cannot analyse a crate without it.\nInstall it with: rustup toolchain install {toolchain} --component rustc-dev --component llvm-tools-preview"
        ));
    }
    check_installed_compiler(toolchain)
}

/// Refuse a toolchain that is installed under the right name but is not the
/// compiler this binary was linked against.
///
/// A channel such as `nightly` can be updated in place, and `rustup toolchain
/// install` will happily replace the contents under the same name. The embedded
/// commit is the only thing that says whether the private MIR APIs this binary
/// calls are the ones present on disk. Asked through `rustup run` so the answer
/// is about the toolchain that will actually drive the run, and skipped when
/// rustup cannot answer: the compiler will produce a real error in that case,
/// and this check must not replace it with a guess.
fn check_installed_compiler(toolchain: &str) -> Result<(), String> {
    let output = match Command::new("rustup")
        .args(["run", toolchain, "rustc", "-vV"])
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return Ok(()),
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(loaded) = crate::host_env::parse_rustc_identity(&text) else {
        return Ok(());
    };
    crate::compiler::check_loaded(&crate::compiler::this_build(), &loaded)
}

/// True when `rustup toolchain list` names `toolchain`.
///
/// Matched against the name column only, and exactly: rustup prints
/// `<name> (active, default)` or a bare `<name>`, and a substring test would let
/// `nightly` in the listing satisfy a check for `nightly-2026-07-24` — the one
/// confusion that would turn a missing toolchain into a rustc version mismatch,
/// which is the failure this function exists to replace.
fn toolchain_is_installed(toolchain: &str, listing: &str) -> bool {
    listing
        .lines()
        .any(|line| line.split_whitespace().next().unwrap_or_default() == toolchain)
}

/// The crate currently being compiled, taken from rustc's own `--crate-name`.
///
/// Cargo always passes it. It is what lets a worker label its findings with the
/// crate they belong to, so a merged workspace report is not ambiguous.
fn crate_name_from_args(args: &[String]) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--crate-name" {
            return iter.next().cloned();
        }
    }
    None
}

fn z3_version_of(z3_bin: &Option<PathBuf>) -> Option<String> {
    let bin = z3_bin.as_ref()?;
    let output = std::process::Command::new(bin)
        .arg("--version")
        .output()
        .ok()?;
    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !text.is_empty() {
            return Some(text);
        }
    }
    None
}

/// Which toolchain and which solver this run was decided in.
///
/// Discovery supplies candidate paths, not proof of the loaded compiler/solver.
/// Actual compiler identity is recorded by each worker and applied by the merge;
/// standalone replay solver identity is intentionally distinct from library identity.
fn environment_of(manifest_dir: &Path) -> crate::report::EnvValue {
    let discovered = crate::host_env::discover(manifest_dir);
    let path = |p: &Option<PathBuf>| {
        p.as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    };
    let mut env = crate::report::run_environment(
        "src/host_env.rs (discovered paths, not worker compiler identity)",
        &path(&discovered.rustc),
        &discovered.host.clone().unwrap_or_default(),
        &path(&discovered.lib_dir),
        &path(&discovered.z3_bin),
        &discovered
            .z3_source
            .map(|source| source.label().to_string())
            .unwrap_or_default(),
        &crate::host_env::join_paths(&discovered.path_additions, crate::host_env::Flavour::Native),
    );
    if let Some(v) = z3_version_of(&discovered.z3_bin) {
        env["z3_version"] = serde_json::json!(v);
    }
    env["pointer_width"] = serde_json::json!(usize::BITS);
    env["tool_version"] = serde_json::json!(crate::report::version());
    env["schema_version"] = serde_json::json!(1);
    let is_release = std::env::var("KELLER_RELEASE").is_ok();
    env["analysis_profile"] = serde_json::json!(crate::report::analysis_profile(is_release));
    env["identity_note"] = serde_json::json!("Compiler identity and rustlib are replaced from actual worker receipts. path_add contains process-native discovery paths. z3_bin is the discovered replay executable, not attestation of the loaded solver library.");
    env
}

#[cfg(windows)]
fn inject_msvc_environment(cmd: &mut std::process::Command) {
    if std::env::var_os("INCLUDE").is_some() {
        return;
    }
    let prog_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".to_string());
    let vswhere = std::path::Path::new(&prog_files_x86).join(r"Microsoft Visual Studio\Installer\vswhere.exe");
    let mut vcvars: Option<std::path::PathBuf> = None;
    if vswhere.is_file() {
        if let Ok(output) = std::process::Command::new(&vswhere)
            .args(&["-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"])
            .output() 
        {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path_str.is_empty() {
                    let candidate = std::path::Path::new(&path_str).join(r"VC\Auxiliary\Build\vcvars64.bat");
                    if candidate.is_file() {
                        vcvars = Some(candidate);
                    }
                }
            }
        }
    }
    if vcvars.is_none() {
        let candidates = [
            r"C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvars64.bat",
            r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat",
            r"C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat",
            r"C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat",
            r"C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\VC\Auxiliary\Build\vcvars64.bat",
        ];
        for c in candidates {
            let p = std::path::Path::new(c);
            if p.is_file() {
                vcvars = Some(p.to_path_buf());
                break;
            }
        }
    }

    if let Some(bat) = vcvars {
        let bat_str = bat.to_string_lossy();
        if let Ok(output) = std::process::Command::new("cmd")
            .args(&["/c", &format!("call \"{bat_str}\" >nul 2>&1 && set")])
            .output()
        {
            if output.status.success() {
                let out_str = String::from_utf8_lossy(&output.stdout);
                for line in out_str.lines() {
                    if let Some((k, v)) = line.split_once('=') {
                        match k {
                            "INCLUDE" | "LIB" | "LIBPATH" | "VCINSTALLDIR" | "VCToolsInstallDir"
                            | "WindowsSdkDir" | "WindowsSDKVersion" | "UCRTVersion" | "UniversalCRTSdkDir" => {
                                cmd.env(k, v);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}