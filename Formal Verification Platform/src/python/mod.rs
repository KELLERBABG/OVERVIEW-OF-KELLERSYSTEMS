pub mod solver;
pub mod testgen;

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::{EXIT_GATE, EXIT_OK, EXIT_SETUP};
use crate::report::{
    write_sarif, ObligationRecord, VerifyOptions,
    DEFAULT_SOLVER_TIMEOUT_MS, OBLIGATION_MANIFEST_FILE,
};
use crate::types::{Coverage, VerificationEvent, Witness};
use solver::{
    check_call_site_precondition, resolve_z3_binary, CallSiteInfo, PyExpr, PythonObligation,
    PythonScanResult, SolverVerdict,
};

pub const PY_AST_SCRIPT: &str = include_str!("keller_ast_dump.py");

/// Check whether the specified path represents a Python project.
pub fn detect_python_project(path: &Path) -> bool {
    if path.is_file() {
        return path.extension().map_or(false, |ext| ext == "py");
    }

    if !path.is_dir() {
        return false;
    }

    // Manifests that definitively identify a Python project
    if path.join("pyproject.toml").exists()
        || path.join("setup.py").exists()
        || path.join("requirements.txt").exists()
        || path.join("Pipfile").exists()
    {
        return true;
    }

    // If there is a Cargo.toml, it is a Rust project
    if path.join("Cargo.toml").exists() {
        return false;
    }

    // Check if directory contains .py files
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().map_or(false, |ext| ext == "py") {
                return true;
            }
        }
    }

    false
}

/// Locate a runnable python interpreter.
pub fn resolve_python_interpreter() -> Result<String, String> {
    for candidate in &["python", "python3", "py"] {
        if let Ok(output) = Command::new(candidate).arg("--version").output() {
            if output.status.success() {
                return Ok(candidate.to_string());
            }
        }
    }
    Err("Python interpreter not found on PATH. Please ensure Python 3.8+ is installed.".to_string())
}

/// Run Python formal verification.
pub fn run_python_verify(options: &VerifyOptions, target_path: &Path) -> i32 {
    let start_time = Instant::now();
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    if !options.quiet && !options.json {
        println!("[Keller-Pipeline] Initiating Python Formal Verification Engine...");
        println!("[Keller-Pipeline] Target: {}", target_path.display());
    }

    let python_bin = match resolve_python_interpreter() {
        Ok(bin) => bin,
        Err(err) => {
            eprintln!("[Keller-Pipeline] Setup error: {err}");
            return EXIT_SETUP;
        }
    };

    let z3_bin = match resolve_z3_binary(&current_dir) {
        Ok(bin) => bin,
        Err(err) => {
            eprintln!("[Keller-Pipeline] Setup error: {err}");
            return EXIT_SETUP;
        }
    };

    if !options.quiet && !options.json {
        println!("[Keller-Pipeline] Solver: Z3 at {}", z3_bin.display());
    }

    // Run Python AST extractor
    let mut ast_cmd = Command::new(&python_bin);
    ast_cmd.arg("-").arg(target_path);
    ast_cmd.stdin(Stdio::piped());
    ast_cmd.stdout(Stdio::piped());
    ast_cmd.stderr(Stdio::piped());

    let mut child = match ast_cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[Keller-Pipeline] Failed to spawn Python AST scanner: {e}");
            return EXIT_SETUP;
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(PY_AST_SCRIPT.as_bytes());
    }

    let output = match child.wait_with_output() {
        Ok(out) => out,
        Err(e) => {
            eprintln!("[Keller-Pipeline] Failed waiting for Python scanner: {e}");
            return EXIT_SETUP;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("[Keller-Pipeline] Python AST extraction failed: {stderr}");
        return EXIT_SETUP;
    }

    let scan_result: PythonScanResult = match serde_json::from_slice(&output.stdout) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("[Keller-Pipeline] Failed to parse AST scanner output: {e}");
            return EXIT_SETUP;
        }
    };

    if !options.quiet && !options.json {
        println!(
            "[Keller-Pipeline] Scanned {} files, discovered {} formal obligations",
            scan_result.files_scanned, scan_result.total_obligations
        );
    }

    let timeout_ms = options.solver_timeout_ms.unwrap_or(DEFAULT_SOLVER_TIMEOUT_MS);
    let mut findings: Vec<VerificationEvent> = Vec::new();
    let mut obligation_records: Vec<ObligationRecord> = Vec::new();

    let mut proved_count = 0;
    let mut fail_count = 0;
    let mut warn_count = 0;
    let mut handled_count = 0;
    let mut panic_sites_count = 0;

    let obligations_root = PathBuf::from("obligations");
    let _ = fs::create_dir_all(&obligations_root);

    let project_name = target_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "python_project".to_string());

    // PASS: Discover Interprocedural Helper Preconditions (S7 / precond_probe parity)
    // For each helper function with an assertion or safety precondition,
    // if all discovered call sites satisfy it, analyse the helper under that proved precondition.
    let mut helper_summaries: HashMap<String, (PyExpr, String)> = HashMap::new();

    let mut call_sites_by_callee: HashMap<String, Vec<&CallSiteInfo>> = HashMap::new();
    for site in &scan_result.call_sites {
        call_sites_by_callee
            .entry(site.callee.clone())
            .or_default()
            .push(site);
    }

    let mut obls_by_func: HashMap<String, Vec<&solver::RawObligation>> = HashMap::new();
    for obl in &scan_result.obligations {
        obls_by_func
            .entry(obl.function.clone())
            .or_default()
            .push(obl);
    }

    for (callee_name, sites) in &call_sites_by_callee {
        if sites.is_empty() {
            continue;
        }

        let Some(func_obls) = obls_by_func.get(callee_name) else {
            continue;
        };

        for obl in func_obls {
            let (candidate_precond, precond_str) = match obl.kind.as_str() {
                "assert" => {
                    let precond = match &obl.target {
                        PyExpr::Unary { op, arg } if op == "not" => (**arg).clone(),
                        other => PyExpr::Unary {
                            op: "not".to_string(),
                            arg: Box::new(other.clone()),
                        },
                    };
                    let desc = if !obl.source_line.is_empty() {
                        obl.source_line.clone()
                    } else {
                        "precondition assertion".to_string()
                    };
                    (precond, desc)
                }
                "div_zero" | "index_bounds" | "none_deref" | "key_error" | "type_error" | "stop_iteration" | "eval_exec" | "unpacking_error" | "format_error" | "concurrency_race" => {
                    let precond = PyExpr::Unary {
                        op: "not".to_string(),
                        arg: Box::new(obl.target.clone()),
                    };
                    (precond, format!("safe {}", obl.kind))
                }
                _ => continue,
            };

            let param_names = if !obl.param_order.is_empty() {
                obl.param_order.clone()
            } else {
                let mut keys: Vec<String> = obl.params.keys().cloned().collect();
                keys.sort();
                keys
            };

            let all_satisfy = sites.iter().all(|site| {
                check_call_site_precondition(site, &param_names, &candidate_precond, &z3_bin, timeout_ms)
            });

            if all_satisfy {
                if !options.quiet && !options.json {
                    println!(
                        "  [PASS] Helper summary in {callee_name}: `{precond_str}` is checked at all {} call site(s), so this body is analysed under it",
                        sites.len()
                    );
                }

                for site in sites {
                    proved_count += 1;
                    if !options.quiet && !options.json {
                        println!(
                            "  [PASS] Helper precondition in {} (line {}): `{precond_str}` holds for `{callee_name}`",
                            site.caller_function, site.line
                        );
                    }
                    obligation_records.push(ObligationRecord {
                        crate_name: site.caller_module.clone(),
                        function: site.caller_function.clone(),
                        block: format!("L{}", site.line),
                        status: "PASS".to_string(),
                        kind: "helper-precondition".to_string(),
                        verdict: "PASS (helper-precondition)".to_string(),
                        artifact: "".to_string(),
                        pinned_artifact: "".to_string(),
                        file: obl.file.clone(),
                        line: Some(site.line),
                    });
                }

                helper_summaries.insert(callee_name.clone(), (candidate_precond, precond_str));
                break;
            }
        }
    }

    for raw in scan_result.obligations {
        let extra_guards: Vec<PyExpr> = if let Some((ref precond, _)) = helper_summaries.get(&raw.function) {
            vec![precond.clone()]
        } else {
            Vec::new()
        };

        let obl = PythonObligation::from_raw_with_extra_guards(raw.clone(), &extra_guards);
        let verdict = obl.solve(&z3_bin, timeout_ms);

        let module_dir = obligations_root.join(&raw.module);
        let _ = fs::create_dir_all(&module_dir);

        let is_handled = raw.handled;
        if is_handled {
            panic_sites_count += 1;
        }

        let (status_str, verdict_hdr, defect_class, details, witness_list) = match verdict {
            SolverVerdict::Pass => {
                proved_count += 1;
                if is_handled {
                    handled_count += 1;
                }
                ("PASS", "PASS (unsat)", "".to_string(), "Formally proved safe by Z3 SMT solver".to_string(), Vec::new())
            }
            SolverVerdict::Fail { ref model } => {
                if is_handled {
                    handled_count += 1;
                    proved_count += 1;
                    ("PASS", "PASS (handled)", "".to_string(), format!("Handled exception: {} caught by try/except at line {}", raw.kind, raw.line), Vec::new())
                } else {
                    fail_count += 1;
                    let defect = match raw.kind.as_str() {
                        "div_zero" => "python-zero-division",
                        "index_bounds" => "python-index-out-of-bounds",
                        "assert" => "python-assertion-failure",
                        "none_deref" => "python-attribute-error",
                        "key_error" => "python-key-error",
                        "type_error" => "python-type-error",
                        "stop_iteration" => "python-stop-iteration",
                        "eval_exec" => "python-dynamic-eval",
                        "unpacking_error" => "python-unpacking-mismatch",
                        "format_error" => "python-format-error",
                        "concurrency_race" => "python-concurrency-race",
                        _ => "python-runtime-defect",
                    };

                    let mut w_list = Vec::new();
                    for (k, v) in model {
                        w_list.push(Witness {
                            input: format!("{k} (arg)"),
                            symbol: k.clone(),
                            value: v.clone(),
                        });
                    }
                    w_list.sort_by(|a, b| a.symbol.cmp(&b.symbol));

                    let details = match raw.kind.as_str() {
                        "div_zero" => format!("ZeroDivisionError: division or modulo by zero reachable at line {}", raw.line),
                        "index_bounds" => format!("IndexError: sequence index out of bounds reachable at line {}", raw.line),
                        "assert" => format!("AssertionError: assertion can fail at line {}", raw.line),
                        "none_deref" => format!("AttributeError: 'NoneType' object attribute dereference reachable at line {}", raw.line),
                        "key_error" => format!("KeyError: dictionary key missing reachable at line {}", raw.line),
                        "type_error" => format!("TypeError: incompatible operand or argument type at line {}", raw.line),
                        "stop_iteration" => format!("StopIteration: iterator exhaustion without default reachable at line {}", raw.line),
                        "eval_exec" => format!("SecurityRisk: unconstrained dynamic code execution (eval/exec) reachable at line {}", raw.line),
                        "unpacking_error" => format!("ValueError: sequence length does not match unpacking target at line {}", raw.line),
                        "format_error" => format!("TypeError/ValueError: format string argument count/type mismatch at line {}", raw.line),
                        "concurrency_race" => format!("RuntimeError/RaceRisk: unconstrained thread spawning without synchronization lock at line {}", raw.line),
                        _ => format!("Defect reachable at line {}", raw.line),
                    };

                    ("FAIL", "FAIL (sat)", defect.to_string(), details, w_list)
                }
            }
            SolverVerdict::Warn { ref reason } => {
                warn_count += 1;
                ("WARN", "WARN (undecided)", "".to_string(), reason.clone(), Vec::new())
            }
        };

        let base_art = format!("{}.smt2", raw.id);
        let art_path = module_dir.join(&base_art);

        let mut header_lines = Vec::new();
        header_lines.push("; keller-verify proof obligation".to_string());
        header_lines.push(format!("; verdict: {verdict_hdr}"));
        header_lines.push(format!("; crate: {}", raw.module));
        header_lines.push(format!("; function: {}", raw.function));
        header_lines.push(format!("; block: L{}", raw.line));
        header_lines.push(format!("; kind: {}", raw.kind));
        header_lines.push(format!("; file: {}:{}", raw.file, raw.line));
        header_lines.push(format!("; details: {details}"));
        for w in &witness_list {
            header_lines.push(format!("; witness: {} = {} (arg)", w.symbol, w.value));
        }

        let full_smt_content = format!("{}\n{}", header_lines.join("\n"), obl.smt_content);
        let _ = fs::write(&art_path, &full_smt_content);

        let mut pinned_art = String::new();
        if let SolverVerdict::Fail { ref model } = verdict {
            if !is_handled {
                let pinned_content = format!("{}\n{}", header_lines.join("\n"), obl.generate_pinned_witness_smt2(model));
                let p_art = format!("{}.witness.smt2", raw.id);
                let _ = fs::write(module_dir.join(&p_art), pinned_content);
                pinned_art = p_art;
            }
        }

        let event = VerificationEvent {
            crate_name: raw.module.clone(),
            function: raw.function.clone(),
            block: format!("L{}", raw.line),
            status: status_str.to_string(),
            kind: match raw.kind.as_str() {
                "div_zero" => "divzero".to_string(),
                "index_bounds" => "bounds".to_string(),
                "assert" => "assert".to_string(),
                "none_deref" => "none_deref".to_string(),
                "key_error" => "key_error".to_string(),
                "type_error" => "type_error".to_string(),
                other => other.to_string(),
            },
            defect_class,
            file: raw.file.clone(),
            line: Some(raw.line),
            details: details.clone(),
            witness: witness_list.clone(),
            witness_kind: if !witness_list.is_empty() { "pinned".to_string() } else { String::new() },
        };

        if status_str != "PASS" {
            findings.push(event.clone());
        }

        obligation_records.push(ObligationRecord {
            crate_name: raw.module.clone(),
            function: raw.function.clone(),
            block: format!("L{}", raw.line),
            status: status_str.to_string(),
            kind: raw.kind.clone(),
            verdict: verdict_hdr.to_string(),
            artifact: base_art,
            pinned_artifact: pinned_art,
            file: raw.file.clone(),
            line: Some(raw.line),
        });

        if !options.quiet && !options.json {
            match status_str {
                "PASS" => {
                    if is_handled {
                        println!("  [PASS] {}:{} (line {}) [handled exception caught by try/except]", raw.module, raw.function, raw.line);
                    } else if helper_summaries.contains_key(&raw.function) {
                        println!("  [PASS] {}:{} (line {}) [proved under caller precondition]", raw.module, raw.function, raw.line);
                    } else {
                        println!("  [PASS] {}:{} (line {})", raw.module, raw.function, raw.line);
                    }
                }
                "FAIL" => {
                    let w_str: Vec<String> = witness_list.iter().map(|w| format!("{} = {}", w.symbol, w.value)).collect();
                    println!("  \x1b[31m[FAIL]\x1b[0m {}:{} (line {}) -> {}", raw.module, raw.function, raw.line, details);
                    if !w_str.is_empty() {
                        println!("         \x1b[33mwitness inputs:\x1b[0m {}", w_str.join(", "));
                    }
                    println!("         \x1b[90mobligation: obligations/{}/{}.smt2\x1b[0m", raw.module, raw.id);
                }
                _ => {
                    println!("  \x1b[33m[WARN]\x1b[0m {}:{} (line {}) -> {}", raw.module, raw.function, raw.line, details);
                }
            }
        }
    }

    // Write index.json in obligations root
    if let Ok(manifest_json) = serde_json::to_string_pretty(&obligation_records) {
        let _ = fs::write(obligations_root.join(OBLIGATION_MANIFEST_FILE), manifest_json);
    }

    let elapsed = start_time.elapsed();
    let total_assertions = obligation_records.len();

    let coverage = Coverage {
        functions: scan_result.files_scanned,
        reachable_assertions: total_assertions,
        supported_assertions: total_assertions,
        unsupported_assertions: 0,
        unsupported_kinds: Vec::new(),
        passed: proved_count,
        failed: fail_count,
        warnings: warn_count,
        without_verdict: 0,
        panic_sites: panic_sites_count,
        panic_sites_decided: handled_count,
        obligation_solver_calls: total_assertions,
        solver_elapsed_ms: elapsed.as_millis() as u64,
    };

    // Write report.json
    let report_data = serde_json::json!({
        "schema_version": 1,
        "crate": project_name,
        "language": "python",
        "completed": true,
        "coverage": coverage,
        "findings": findings,
        "evidence": [],
        "obligations": obligation_records,
        "build_elapsed_ms": elapsed.as_millis() as u64
    });

    if let Ok(json_str) = serde_json::to_string_pretty(&report_data) {
        let _ = fs::write("report.json", json_str);
    }

    // Write report.environment.json
    let env_data = serde_json::json!({
        "language": "python",
        "python_bin": python_bin,
        "z3_bin": z3_bin.display().to_string(),
        "files_scanned": scan_result.files_scanned,
        "total_obligations": total_assertions
    });
    if let Ok(env_str) = serde_json::to_string_pretty(&env_data) {
        let _ = fs::write("report.environment.json", env_str);
    }

    // Write SARIF if requested
    if let Some(ref sarif_path) = options.sarif {
        let _ = write_sarif(sarif_path, &findings, &coverage);
    }

    if options.json {
        let summary = serde_json::json!({
            "language": "python",
            "coverage": coverage,
            "findings_count": findings.len(),
            "findings": findings
        });
        println!("{}", serde_json::to_string_pretty(&summary).unwrap_or_default());
    } else {
        println!();
        println!("------------------------------------------------------------");
        println!(
            "Keller Formal Verification Summary [Python]: {} checked | {} proved | {} failed | {} undecided",
            coverage.supported_assertions, coverage.passed, coverage.failed, coverage.warnings
        );
        if coverage.panic_sites_decided > 0 {
            println!("  ✅ Panic sites / exceptions decided: {} handled", coverage.panic_sites_decided);
        }
        println!("Completed in {:.2}s", elapsed.as_secs_f64());
        println!("------------------------------------------------------------");
    }

    // Judge gate
    let is_strict = options.strict_coverage || options.deny_warn;
    if fail_count > 0 {
        EXIT_GATE
    } else if is_strict && warn_count > 0 {
        EXIT_GATE
    } else {
        EXIT_OK
    }
}
