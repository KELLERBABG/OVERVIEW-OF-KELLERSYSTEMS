//! `keller testgen`: generate executable Rust `#[test]` reproduction skeletons
//! from verification counterexamples (O10).
//!
//! When formal verification flags a `FAIL`, it computes a concrete counterexample
//! witness (values for operands that trigger the overflow, bounds violation or
//! division by zero). This module turns those witnesses into runnable Rust test
//! functions that developers can paste directly into their test suites to reproduce
//! and debug the failure under native cargo test execution.

use crate::types::VerificationEvent;
use std::path::Path;

/// Parse a single .witness.smt2 file into a VerificationEvent.
pub fn parse_witness_str(content: &str) -> Option<VerificationEvent> {
    let mut crate_name = String::new();
    let mut function = String::new();
    let mut block = String::new();
    let mut kind = String::new();
    let mut file = String::new();
    let mut line = None;
    let mut details = String::new();
    let mut status = "FAIL".to_string();
    let mut witnesses = Vec::new();

    for line_str in content.lines() {
        let trimmed = line_str.trim();
        if !trimmed.starts_with(';') {
            continue;
        }
        let comment = trimmed.trim_start_matches(';').trim();
        if let Some(rest) = comment.strip_prefix("crate:") {
            crate_name = rest.trim().to_string();
        } else if let Some(rest) = comment.strip_prefix("function:") {
            function = rest.trim().to_string();
        } else if let Some(rest) = comment.strip_prefix("block:") {
            block = rest.trim().to_string();
        } else if let Some(rest) = comment.strip_prefix("kind:") {
            kind = rest.trim().to_string();
        } else if let Some(rest) = comment.strip_prefix("file:") {
            let f = rest.trim();
            if let Some((path_part, line_part)) = f.rsplit_once(':') {
                file = path_part.to_string();
                line = line_part.parse().ok();
            } else {
                file = f.to_string();
            }
        } else if let Some(rest) = comment.strip_prefix("details:") {
            details = rest.trim().to_string();
        } else if let Some(rest) = comment.strip_prefix("verdict:") {
            let v = rest.trim();
            if v.starts_with("FAIL") {
                status = "FAIL".to_string();
            } else {
                status = v.to_string();
            }
        } else if let Some(rest) = comment.strip_prefix("witness:") {
            let w_str = rest.trim();
            if let Some((left, right)) = w_str.split_once('=') {
                let symbol = left.trim().to_string();
                let (val, input) = if let (Some(l_idx), Some(r_idx)) = (right.find('('), right.rfind(')')) {
                    if l_idx < r_idx {
                        (right[..l_idx].trim().to_string(), right[l_idx + 1..r_idx].trim().to_string())
                    } else {
                        (right.trim().to_string(), String::new())
                    }
                } else {
                    (right.trim().to_string(), String::new())
                };
                witnesses.push(crate::types::Witness {
                    symbol,
                    input,
                    value: val,
                });
            }
        }
    }

    if function.is_empty() {
        return None;
    }

    Some(VerificationEvent {
        crate_name,
        function,
        block,
        status,
        kind,
        defect_class: "counterexample".to_string(),
        file,
        line,
        details,
        witness: witnesses,
        witness_kind: "pinned".to_string(),
    })
}

/// Generate Rust reproduction test code from a report file or .witness.smt2 file.
pub fn generate_tests_from_file(report_path: &Path) -> Result<String, String> {
    if report_path.is_dir() {
        let mut events = Vec::new();
        if let Ok(entries) = std::fs::read_dir(report_path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map_or(false, |ext| ext == "smt2") {
                    if let Ok(content) = std::fs::read_to_string(&p) {
                        if let Some(event) = parse_witness_str(&content) {
                            events.push(event);
                        }
                    }
                }
            }
        }
        if events.is_empty() {
            let rep = report_path.join("report.json");
            if rep.exists() {
                return generate_tests_from_file(&rep);
            }
        }
        return Ok(generate_tests(&events));
    }

    let content = std::fs::read_to_string(report_path)
        .map_err(|e| format!("Could not read report at '{}': {e}", report_path.display()))?;

    if content.starts_with("; keller-verify") || report_path.extension().map_or(false, |ext| ext == "smt2") {
        if let Some(event) = parse_witness_str(&content) {
            return Ok(generate_tests(&[event]));
        } else {
            return Err(format!("Could not parse .witness.smt2 at '{}'", report_path.display()));
        }
    }

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
        if val.get("language").and_then(|l| l.as_str()) == Some("python") {
            let findings: Vec<VerificationEvent> = val
                .get("findings")
                .and_then(|f| serde_json::from_value(f.clone()).ok())
                .unwrap_or_default();
            let temp_out = std::env::temp_dir().join("test_keller_generated.py");
            let _ = crate::python::testgen::generate_pytest_file(&findings, &temp_out);
            return std::fs::read_to_string(&temp_out).map_err(|e| e.to_string());
        }
        if let Some(findings_val) = val.get("findings") {
            if let Ok(events) = serde_json::from_value::<Vec<VerificationEvent>>(findings_val.clone()) {
                return Ok(generate_tests(&events));
            }
        }
    }

    let events: Vec<VerificationEvent> = serde_json::from_str(&content).map_err(|e| {
        format!(
            "Could not parse report JSON at '{}': {e}",
            report_path.display()
        )
    })?;
    Ok(generate_tests(&events))
}

/// Format a concrete counterexample value into valid Rust literal syntax.
/// Handles scalar integers, bools, slices `[1, 2, 3]` -> `&[1, 2, 3]`, and tuples `(1, 2)`.
pub fn format_rust_literal(val: &str) -> String {
    let trimmed = val.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        format!("&{trimmed}")
    } else if trimmed.starts_with('&') {
        trimmed.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Generate Rust test source code for all `FAIL` events with witnesses.
pub fn generate_tests(events: &[VerificationEvent]) -> String {
    let mut out = String::new();
    out.push_str(
        "//! Auto-generated counterexample reproduction tests from Keller-Verify (O10).\n",
    );
    out.push_str(
        "//! Run with `cargo test` to reproduce formal verification failures natively.\n\n",
    );
    out.push_str("#[cfg(test)]\n");
    out.push_str("mod keller_counterexample_tests {\n");

    let fail_events: Vec<&VerificationEvent> =
        events.iter().filter(|e| e.status == "FAIL").collect();

    if fail_events.is_empty() {
        out.push_str("    // No FAIL counterexamples were found in the verification report.\n");
        out.push_str("}\n");
        return out;
    }

    for (idx, event) in fail_events.iter().enumerate() {
        let fn_ident = sanitize_ident(&format!(
            "repro_{}_{}_{}",
            event.crate_name, event.function, event.block
        ));
        let test_name = if fn_ident.is_empty() {
            format!("repro_failure_{idx}")
        } else {
            fn_ident
        };

        out.push_str("\n    #[test]\n");
        out.push_str("    #[should_panic]\n");
        out.push_str(&format!("    fn {test_name}() {{\n"));
        out.push_str(&format!(
            "        // Crate: {}, Function: `{}` ({})\n",
            event.crate_name, event.function, event.block
        ));
        if !event.file.is_empty() {
            out.push_str(&format!(
                "        // Source: {}:{}\n",
                event.file,
                event
                    .line
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "?".to_string())
            ));
        }
        out.push_str(&format!(
            "        // Obligation: {} ({})\n",
            event.kind,
            if event.defect_class.is_empty() {
                "unclassified"
            } else {
                &event.defect_class
            }
        ));
        out.push_str(&format!(
            "        // Details: {}\n",
            event.details.replace('\n', " ")
        ));
        if !event.witness_kind.is_empty() {
            out.push_str(&format!(
                "        // Witness certainty: {}\n",
                event.witness_kind
            ));
        }

        let mut arg_vars = Vec::new();
        if event.witness.is_empty() {
            out.push_str(
                "        // Note: No concrete witness symbols were captured for this failure.\n",
            );
        } else {
            out.push_str("        // Counterexample witness assignments:\n");
            for w in &event.witness {
                let var_name = if !w.input.is_empty() {
                    let clean = w.input.split('(').next().unwrap_or("").trim();
                    let clean = sanitize_ident(clean);
                    if !clean.is_empty() && !clean.chars().next().unwrap().is_ascii_digit() {
                        clean
                    } else {
                        format!("var_{}", sanitize_ident(&w.symbol))
                    }
                } else {
                    format!("var_{}", sanitize_ident(&w.symbol))
                };
                let formatted_val = format_rust_literal(&w.value);
                out.push_str(&format!(
                    "        let {var_name} = {formatted_val}; // input: {}\n",
                    w.input
                ));
                arg_vars.push(var_name);
            }
        }

        out.push_str("        // Trigger function under test with counterexample inputs:\n");
        let args_joined = arg_vars.join(", ");
        out.push_str(&format!(
            "        let _ = {}::{}({args_joined});\n",
            event.crate_name, event.function
        ));
        out.push_str("    }\n");
    }

    out.push_str("}\n");
    out
}

/// Replace characters invalid in Rust identifiers with underscores.
fn sanitize_ident(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            result.push(c);
        } else {
            result.push('_');
        }
    }
    // Collapse consecutive underscores
    let mut dedup = String::new();
    let mut prev_underscore = false;
    for c in result.chars() {
        if c == '_' {
            if !prev_underscore {
                dedup.push(c);
                prev_underscore = true;
            }
        } else {
            dedup.push(c);
            prev_underscore = false;
        }
    }
    dedup.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Witness;

    #[test]
    fn test_empty_report_generates_empty_module() {
        let text = generate_tests(&[]);
        assert!(text.contains("mod keller_counterexample_tests"));
        assert!(text.contains("No FAIL counterexamples were found"));
    }

    #[test]
    fn test_fail_event_generates_test_function() {
        let events = vec![
            VerificationEvent {
                crate_name: "my_crate".to_string(),
                function: "checked_sum".to_string(),
                block: "bb2".to_string(),
                status: "FAIL".to_string(),
                kind: "overflow".to_string(),
                defect_class: "unguarded".to_string(),
                file: "src/lib.rs".to_string(),
                line: Some(42),
                details: "attempt to add with overflow".to_string(),
                witness: vec![
                    Witness {
                        input: "a (_1)".to_string(),
                        symbol: "_1".to_string(),
                        value: "2147483647".to_string(),
                    },
                    Witness {
                        input: "b (_2)".to_string(),
                        symbol: "_2".to_string(),
                        value: "1".to_string(),
                    },
                ],
                witness_kind: "pinned".to_string(),
            },
            VerificationEvent {
                crate_name: "my_crate".to_string(),
                function: "safe_calc".to_string(),
                block: "bb0".to_string(),
                status: "PASS".to_string(),
                kind: "overflow".to_string(),
                defect_class: String::new(),
                file: "src/lib.rs".to_string(),
                line: Some(10),
                details: "proven safe".to_string(),
                witness: vec![],
                witness_kind: String::new(),
            },
        ];

        let text = generate_tests(&events);
        assert!(text.contains("fn repro_my_crate_checked_sum_bb2()"));
        assert!(text.contains("#[should_panic]"));
        assert!(text.contains("let a = 2147483647;"));
        assert!(text.contains("let b = 1;"));
        assert!(text.contains("let _ = my_crate::checked_sum(a, b);"));
        assert!(text.contains("Source: src/lib.rs:42"));
        assert!(!text.contains("safe_calc"));
    }

    #[test]
    fn test_parse_witness_file() {
        let content = r#"; keller-verify proof obligation
; crate:    buggy_project
; function: check_overflow
; block:    bb0
; kind:     overflow
; file:     src/main.rs:6
; details:  Arithmetic Overflow Detection (Add) -> Exploit path found! [x (_1) = 255, y (_2) = 1]
; verdict:  FAIL (sat)
; witness:  overflow_l = 255 (x (_1))
; witness:  overflow_r = 1 (y (_2))
(assert (= overflow_l 255))
"#;
        let event = parse_witness_str(content).expect("parse failed");
        assert_eq!(event.crate_name, "buggy_project");
        assert_eq!(event.function, "check_overflow");
        assert_eq!(event.status, "FAIL");
        assert_eq!(event.witness.len(), 2);
        assert_eq!(event.witness[0].value, "255");
        assert_eq!(event.witness[1].value, "1");

        let code = generate_tests(&[event]);
        assert!(code.contains("let x = 255;"));
        assert!(code.contains("let y = 1;"));
        assert!(code.contains("let _ = buggy_project::check_overflow(x, y);"));
    }

    #[test]
    fn test_slice_and_tuple_counterexample_synthesis() {
        let events = vec![VerificationEvent {
            crate_name: "slice_crate".to_string(),
            function: "process_slice".to_string(),
            block: "bb1".to_string(),
            status: "FAIL".to_string(),
            kind: "bounds_check".to_string(),
            defect_class: "index_oob".to_string(),
            file: "src/lib.rs".to_string(),
            line: Some(15),
            details: "index out of bounds".to_string(),
            witness: vec![
                Witness {
                    input: "slice (_1)".to_string(),
                    symbol: "_1".to_string(),
                    value: "[10, 20, 30]".to_string(),
                },
                Witness {
                    input: "pair (_2)".to_string(),
                    symbol: "_2".to_string(),
                    value: "(42, true)".to_string(),
                },
            ],
            witness_kind: "pinned".to_string(),
        }];

        let text = generate_tests(&events);
        assert!(text.contains("let slice = &[10, 20, 30];"));
        assert!(text.contains("let pair = (42, true);"));
        assert!(text.contains("let _ = slice_crate::process_slice(slice, pair);"));
    }
}
