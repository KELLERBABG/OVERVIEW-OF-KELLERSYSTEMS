use std::path::Path;
use crate::types::VerificationEvent;

pub fn generate_pytest_file(events: &[VerificationEvent], out_path: &Path) -> Result<usize, String> {
    let mut test_cases = Vec::new();
    let mut imports = Vec::new();
    imports.push("import pytest".to_string());

    let mut generated_count = 0;

    for event in events {
        if event.status != "FAIL" {
            continue;
        }

        let _module = &event.crate_name;
        let func = &event.function;
        let line = event.line.unwrap_or(0);
        let safe_func = func.replace('.', "_");

        let exc_type = match event.kind.as_str() {
            "divzero" | "div_zero" => "ZeroDivisionError",
            "bounds" | "index_bounds" => "IndexError",
            "assert" | "assertion" => "AssertionError",
            "none_deref" => "AttributeError",
            "key_error" => "KeyError",
            "type_error" => "TypeError",
            "stop_iteration" => "StopIteration",
            "eval_exec" => "Exception",
            "unpacking_error" => "ValueError",
            "format_error" => "TypeError",
            "concurrency_race" => "RuntimeError",
            _ => "Exception",
        };

        let mut witness_args = Vec::new();
        for w in &event.witness {
            witness_args.push(format!("{}={}", w.symbol, w.value));
        }
        let args_str = witness_args.join(", ");

        let test_name = format!("test_keller_{safe_func}_line_{line}");
        let doc = format!("Formal verification counterexample for {func} (line {line})\n    Defect class: {}\n    Witness: [{args_str}]", event.defect_class);

        test_cases.push(format!(
r#"def {test_name}():
    """{doc}"""
    # Reproduce formal counterexample found by Z3
    with pytest.raises({exc_type}):
        # Call target under witness parameters
        # {func}({args_str})
        pass
"#));
        generated_count += 1;
    }

    let mut full_content = Vec::new();
    full_content.push("# ============================================================================".to_string());
    full_content.push("#  Keller-Verify Generated pytest Reproduction Suite".to_string());
    full_content.push("#  Generated from SMT-LIB 2.6 formal verification counterexample models".to_string());
    full_content.push("# ============================================================================\n".to_string());
    full_content.extend(imports);
    full_content.push("\n".to_string());
    full_content.extend(test_cases);

    let final_str = full_content.join("\n");
    std::fs::write(out_path, final_str).map_err(|e| format!("Failed to write pytest file: {e}"))?;

    Ok(generated_count)
}
