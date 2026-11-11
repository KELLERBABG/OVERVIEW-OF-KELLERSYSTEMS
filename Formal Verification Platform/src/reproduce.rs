//! `keller reproduce`: pin one failing site into a bundle another machine can
//! check, and say plainly what the bundle does not reconstruct.
//!
//! P1.6 asks for concrete replay as its own evidence level. This is the smallest
//! honest part of it. A `FAIL` already writes an obligation (a header naming the
//! crate, function, block, `file:line` and verdict, plus the SMT formula), and
//! `keller replay` re-discharges that formula — but neither artifact says *which
//! build* decided it, *under which flags*, or carries those facts together with
//! the site in one file. `report.run.json` carries `provenance` since P1.4, so the
//! three can be bound and compared *before* anything is re-discharged: a bundle
//! produced by a different compiler is refused rather than presented as a
//! reproduction of what this binary would say.
//!
//! What this is not: a native reproduction. It re-checks the encoded obligation,
//! which is exactly as strong as the encoding — the same claim `keller replay`
//! makes — so the bundle lists, item by item, what it cannot reconstruct, instead
//! of letting a matched `sat` read as an executed program.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::provenance;

/// Exit codes, mirroring the documented contract (see `docs/RELEASE.md`).
const EXIT_USAGE: i32 = 2;
const EXIT_SETUP: i32 = 3;

/// How many directories above the obligation a run record may sit. Obligations
/// live at `<run-root>/obligations/<crate>/<file>.smt2`, but a retained run under
/// `.keller/runs/<id>/` is one level deeper, so the search is a walk rather than a
/// fixed count of `..` segments.
const MAX_RUN_RECORD_DEPTH: usize = 8;

/// What the obligation header said about the site.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObligationHeader {
    pub crate_name: String,
    pub function: String,
    pub block: String,
    pub kind: String,
    pub file: String,
    pub details: String,
    pub verdict: String,
    pub reachable: Option<String>,
    pub witness: Vec<String>,
}

/// Read the `; key: value` header of an obligation.
///
/// The writer is `report::write_smt_obligation`; this parses exactly the keys it
/// emits and ignores everything else, so a header that grows a field does not stop
/// an older bundle from being read.
pub fn parse_header(text: &str) -> ObligationHeader {
    let mut header = ObligationHeader::default();
    for line in text.lines() {
        if !line.starts_with(';') {
            break;
        }
        let body = line.trim_start_matches(';').trim();
        let Some((key, value)) = body.split_once(':') else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim() {
            "crate" => header.crate_name = value,
            "function" => header.function = value,
            "block" => header.block = value,
            "kind" => header.kind = value,
            "file" => header.file = value,
            "details" => header.details = value,
            "verdict" => header.verdict = value,
            "reachable" => header.reachable = Some(value),
            "witness" => header.witness.push(value),
            // `pinned:` and the banner line are deliberately not matched: the
            // pinned file is a second obligation, not a field of this one.
            _ => {}
        }
    }
    header
}

/// The nearest `report.run.json` at or above `start`.
fn find_run_record(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };
    for _ in 0..MAX_RUN_RECORD_DEPTH {
        let candidate = dir.join(crate::report::RUN_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// What a successful `reproduce` produced.
pub struct Outcome {
    pub bundle: PathBuf,
    pub recorded: String,
    pub answer: String,
    pub matched: bool,
    pub site: ObligationHeader,
    /// The facts this bundle deliberately does not establish.
    pub cannot_reconstruct: Vec<String>,
}

/// The list a reader needs before treating a matched replay as a reproduction.
///
/// Each entry names something the bundle does *not* contain, because the failure
/// mode this guards against is a reader assuming that a re-discharged formula is a
/// re-executed program.
fn cannot_reconstruct() -> Vec<String> {
    [
        "a concrete execution: the bundle re-checks the encoded formula, so a matched \
         answer says what the model says, not what the program does",
        "heap contents and allocation addresses: they are free symbols bounded by their \
         type, or not modelled at all",
        "results of unmodelled calls: a value from one is a free symbol, so no input for \
         it can be reconstructed",
        "loop trip counts: loop-carried values are widened, so a proof and a counterexample \
         both describe a bounded unrolling rather than a particular run",
        "the source expression behind an operand: `file:line` names the site, while MIR \
         temporaries are what the formula is stated over",
        "compiler flags the crate's own build sets: only RUSTFLAGS and \
         CARGO_ENCODED_RUSTFLAGS are recorded, and a build script can add its own",
        "the compiler's build contents: release, commit and host pin the compiler, not the \
         bits of that particular rustc executable",
    ]
    .iter()
    .map(|item| item.replace("\\\n", ""))
    .collect()
}

/// Produce the bundle for one obligation and re-discharge it.
///
/// `Err` carries the exit code and the sentence to print, because the caller is a
/// `!` function: a refusal (missing identity, different compiler) must not be
/// mistaken for a gate failure on the code under test.
pub fn run(obligation: &Path, out: Option<&Path>) -> Result<Outcome, (i32, String)> {
    let text = std::fs::read_to_string(obligation).map_err(|err| {
        (
            EXIT_SETUP,
            format!("Could not read '{}': {err}", obligation.display()),
        )
    })?;
    let header = parse_header(&text);
    // The header spells a verdict as `FAIL (sat)` / `PASS (unsat)` / `WARN
    // (unknown)`; `report` owns that parse and `keller replay` already uses it, so
    // reproduce asks the same function rather than growing a second rule that
    // could disagree about what a recorded verdict is.
    let expected = crate::report::obligation_expected(&text).map_err(|err| (EXIT_USAGE, err))?;

    let run_record = find_run_record(obligation).ok_or_else(|| {
        (
            EXIT_SETUP,
            format!(
                "No '{}' above '{}', so the build that decided this obligation cannot be \
                 pinned. Reproduce needs the run record the obligation was written under.",
                crate::report::RUN_FILE,
                obligation.display()
            ),
        )
    })?;
    let record: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&run_record).map_err(|err| {
            (
                EXIT_SETUP,
                format!("Could not read '{}': {err}", run_record.display()),
            )
        })?)
        .map_err(|err| {
            (
                EXIT_SETUP,
                format!("Could not parse '{}': {err}", run_record.display()),
            )
        })?;

    let provenance_record = &record["provenance"];
    let recorded_build = &provenance_record["build"];
    if recorded_build.is_null() {
        return Err((
            EXIT_SETUP,
            format!(
                "'{}' carries no `provenance`: it was written before build identity was \
                 recorded, so this obligation cannot be pinned to a compiler.",
                run_record.display()
            ),
        ));
    }

    // The refusal that makes the bundle worth something: a replay from a different
    // compiler is a different analysis, and presenting it as a reproduction of this
    // one would be a claim nobody checked.
    let current = crate::compiler::this_build().as_json();
    let mut identity_matched = true;
    let mut mismatches = Vec::new();
    for field in ["release", "commit", "host"] {
        let recorded = recorded_build[format!("rustc_{field}")]
            .as_str()
            .unwrap_or("<missing>");
        let current = current[field].as_str().unwrap_or("<missing>");
        if recorded != current {
            identity_matched = false;
            mismatches.push(format!(
                "{field}: recorded {recorded}, this binary {current}"
            ));
        }
    }
    if !identity_matched {
        return Err((
            EXIT_SETUP,
            format!(
                "Refusing to reproduce: the obligation was decided by a different compiler \
                 ({}). Rebuild the verifier with the pinned toolchain, or reproduce on the \
                 machine that wrote the record.",
                mismatches.join("; ")
            ),
        ));
    }

    // The run root is where the record was written, so it is also where a
    // project-local `.z3/` sits for that run.
    let run_root = run_record.parent().unwrap_or(Path::new("."));
    // The solver that re-discharges has to be one this machine can run. First the
    // executable the recorded run itself discovered (`env.z3_bin`), so a bundle
    // re-uses the same binary rather than whatever happens to be on PATH; then a
    // fresh discovery from the run root, then from the current directory.
    let recorded_z3 = record["env"]["z3_bin"]
        .as_str()
        .map(PathBuf::from)
        .filter(|path| path.is_file());
    let z3 = recorded_z3
        .or_else(|| crate::host_env::discover(run_root).z3_bin)
        .or_else(|| crate::host_env::discover(&std::env::current_dir().unwrap_or_default()).z3_bin)
        .ok_or_else(|| {
            (
                EXIT_SETUP,
                "No runnable Z3 found: the record names none, and neither Z3_HOME nor a \
                 project-local .z3/ resolves to an executable."
                    .to_string(),
            )
        })?;
    let replay_version = Command::new(&z3)
        .arg("--version")
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|version| !version.is_empty());

    let output = Command::new(&z3)
        .arg("-smt2")
        .arg("-T:10")
        .arg(obligation)
        .output()
        .map_err(|err| (EXIT_SETUP, format!("Could not replay obligation: {err}")))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() || !stderr.trim().is_empty() {
        return Err((
            EXIT_SETUP,
            format!("Solver rejected the obligation: {stdout}{stderr}"),
        ));
    }
    let answer = stdout.trim().to_string();
    let matched = crate::report::check_replay(&expected, &stdout).is_ok();

    let stem = format!(
        "{}_{}",
        sanitize(&header.crate_name),
        sanitize(&header.block)
    );
    let bundle_dir = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| run_root.join("reproduce").join(&stem));
    std::fs::create_dir_all(&bundle_dir).map_err(|err| {
        (
            EXIT_SETUP,
            format!("Could not create '{}': {err}", bundle_dir.display()),
        )
    })?;

    let obligation_text_bytes = std::fs::read(obligation).unwrap_or_default();
    let cannot = cannot_reconstruct();
    let bundle = serde_json::json!({
        "schema_version": 1,
        "tool": "keller-verify",
        "version": env!("CARGO_PKG_VERSION"),
        "site": {
            "crate": header.crate_name,
            "function": header.function,
            "block": header.block,
            "kind": header.kind,
            "file": header.file,
            "details": header.details,
            "recorded_verdict": header.verdict,
            "reachable": header.reachable,
            "witness": header.witness,
        },
        "identity": {
            "recorded_build": recorded_build,
            "recorded_solver": provenance_record["solver"],
            "recorded_config_hash": provenance_record["config_hash"],
            "flags": {
                "rustflags": record["settings"]["rustflags"],
                "encoded_rustflags": record["settings"]["encoded_rustflags"],
                "analysis_profile": record["analysis_profile"],
                "settings_profile": record["settings"]["profile"],
            },
            "recorded_by": run_record.display().to_string(),
            "this_binary_build": current,
            "this_binary_loaded_solver": provenance::solver_version(),
            "identity_matched": identity_matched,
        },
        "obligation": {
            "path": obligation.display().to_string(),
            "fnv1a": provenance::fnv1a_hex(&obligation_text_bytes),
            "expected_answer": expected,
        },
        "replay": {
            "command": [z3.display().to_string(), "-smt2".to_string(), "-T:10".to_string(), obligation.display().to_string()],
            "replay_executable": z3.display().to_string(),
            "replay_version": replay_version,
            "answer": answer,
            "matched": matched,
        },
        "reproduce_command": format!("keller reproduce '{}'", obligation.display()),
        "native_reproduction": "not-attempted",
        "native_reproduction_reason": "this bundle re-discharges the encoded obligation; it does not execute the program",
        "cannot_reconstruct": cannot,
    });
    let bundle_path = bundle_dir.join("reproduce.json");
    std::fs::write(
        &bundle_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&bundle)
                .map_err(|err| (EXIT_SETUP, format!("Could not serialise the bundle: {err}")))?
        ),
    )
    .map_err(|err| {
        (
            EXIT_SETUP,
            format!("Could not write '{}': {err}", bundle_path.display()),
        )
    })?;

    Ok(Outcome {
        bundle: bundle_path,
        recorded: header.verdict.clone(),
        answer,
        matched,
        site: header,
        cannot_reconstruct: cannot,
    })
}

/// A path-safe stem for the bundle directory.
fn sanitize(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if cleaned.is_empty() {
        "site".to_string()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "; keller-verify proof obligation\n\
        ; crate:    overflow_fail\n\
        ; function: add\n\
        ; block:    bb1\n\
        ; kind:     overflow\n\
        ; file:     src/main.rs:4\n\
        ; details:  Overflow Add Analysis\n\
        ; verdict:  FAIL (sat)\n\
        ; reachable: true\n\
        ; witness:  x = 255 (copy _2)\n\
        (declare-fun x () (_ BitVec 8))\n\
        (assert (= x #xff))\n\
        (check-sat)\n";

    #[test]
    fn the_header_is_read_as_the_writer_meant_it() {
        let header = parse_header(HEADER);
        assert_eq!(header.crate_name, "overflow_fail");
        assert_eq!(header.function, "add");
        assert_eq!(header.block, "bb1");
        assert_eq!(header.kind, "overflow");
        assert_eq!(header.file, "src/main.rs:4");
        assert_eq!(header.verdict, "FAIL (sat)");
        assert_eq!(header.reachable.as_deref(), Some("true"));
        assert_eq!(header.witness, vec!["x = 255 (copy _2)".to_string()]);
    }

    /// The site is what a reader acts on, so a formula with no verdict is refused
    /// rather than reproduced against a guess.
    #[test]
    fn an_obligation_without_a_verdict_is_refused() {
        let dir =
            std::env::temp_dir().join(format!("keller-reproduce-noverdict-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("site.smt2");
        std::fs::write(&path, "(check-sat)\n").unwrap();
        let outcome = run(&path, Some(&dir));
        std::fs::remove_dir_all(&dir).ok();
        match outcome {
            Err((code, message)) => {
                assert_eq!(code, EXIT_USAGE);
                assert!(!message.is_empty());
            }
            Ok(_) => panic!("an obligation without a verdict must be refused"),
        }
    }

    /// The refusal that keeps the bundle honest: no run record means no identity,
    /// and a replay without identity is not evidence about a particular build.
    #[test]
    fn reproducing_without_a_run_record_is_a_setup_failure() {
        let dir =
            std::env::temp_dir().join(format!("keller-reproduce-norecord-{}", std::process::id()));
        let crate_dir = dir.join("obligations").join("crate");
        std::fs::create_dir_all(&crate_dir).unwrap();
        let path = crate_dir.join("site.smt2");
        std::fs::write(&path, HEADER).unwrap();
        let outcome = run(&path, Some(&dir));
        std::fs::remove_dir_all(&dir).ok();
        match outcome {
            Err((code, message)) => {
                assert_eq!(code, EXIT_SETUP, "got {code}: {message}");
                assert!(message.contains("cannot be pinned"), "{message}");
            }
            Ok(_) => panic!("a missing run record must be a setup failure"),
        }
    }

    /// A run record written before provenance existed cannot pin a build, so it is
    /// refused with the reason rather than reproduced against `null`.
    #[test]
    fn reproducing_a_record_without_provenance_is_a_setup_failure() {
        let dir =
            std::env::temp_dir().join(format!("keller-reproduce-noprov-{}", std::process::id()));
        let crate_dir = dir.join("obligations").join("crate");
        std::fs::create_dir_all(&crate_dir).unwrap();
        std::fs::write(
            dir.join(crate::report::RUN_FILE),
            r#"{"schema_version":1,"completed":true}"#,
        )
        .unwrap();
        let path = crate_dir.join("site.smt2");
        std::fs::write(&path, HEADER).unwrap();
        let outcome = run(&path, Some(&dir));
        std::fs::remove_dir_all(&dir).ok();
        match outcome {
            Err((code, message)) => {
                assert_eq!(code, EXIT_SETUP, "got {code}: {message}");
                assert!(message.contains("no `provenance`"), "{message}");
            }
            Ok(_) => panic!("a record without provenance must be refused"),
        }
    }

    /// The list is the point of the command: if it ever became empty, the bundle
    /// would be claiming a native reproduction it never performed.
    #[test]
    fn the_bundle_enumerates_what_it_does_not_reconstruct() {
        let cannot = cannot_reconstruct();
        assert!(cannot.len() >= 5);
        assert!(cannot
            .iter()
            .any(|item| item.contains("concrete execution")));
        assert!(cannot.iter().any(|item| item.contains("loop")));
        assert!(cannot.iter().any(|item| item.contains("compiler flags")));
    }
}
