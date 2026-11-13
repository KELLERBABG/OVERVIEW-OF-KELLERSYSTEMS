//! Verification driver for Java bytecode: NPE, AIOOBE, CCE checks.

use std::path::{Path, PathBuf};

use crate::frontend::java::bytecode::{self, ClassFile};
use crate::report::{Finding, Report, Severity};

/// Result of a verification run.
pub struct VerifyResult {
    pub report: Report,
    pub class_files: Vec<PathBuf>,
}

impl VerifyResult {
    pub fn summary_line(&self) -> String {
        self.report.summary_line()
    }
}

/// Verify a single .class file or directory of .class files.
pub fn verify_class(
    class_path: &Path,
    output: &Path,
    audit_only: bool,
) -> anyhow::Result<VerifyResult> {
    let mut report = Report::new("java-verification");
    let mut class_files = Vec::new();

    // Collect .class files
    if class_path.is_dir() {
        collect_class_files(class_path, &mut class_files)?;
    } else if class_path.exists() {
        class_files.push(class_path.to_path_buf());
    } else {
        return Err(anyhow::anyhow!(
            "Path does not exist: {}",
            class_path.display()
        ));
    }

    if class_files.is_empty() {
        return Err(anyhow::anyhow!(
            "No .class files found at: {}",
            class_path.display()
        ));
    }

    for cf_path in &class_files {
        let cf_path = PathBuf::from(cf_path);
        match verify_single_class(&cf_path, &mut report) {
            Ok(()) => {}
            Err(e) => {
                report.add_finding(Finding {
                    id: format!("PARSE-{}", cf_path.display()),
                    severity: Severity::Error,
                    file: cf_path.display().to_string(),
                    line: 0,
                    message: format!("Failed to parse class file: {}", e),
                    invariant: String::from("parse-success"),
                });
            }
        }
    }

    // Populate provenance
    let opts = crate::provenance::EvidenceOptions::default();
    report.populate_provenance(&opts);

    // Write report (always, even in audit-only mode)
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    report.write_json(output)?;

    // In audit-only mode, we do NOT mutate or heal source.
    // We simply save the report and return.
    if !audit_only {
        // Future: apply neuro-symbolic auto-healing patches
        // For now, audit-only is the default safe behavior.
        let _ = audit_only;
    }

    Ok(VerifyResult {
        report,
        class_files,
    })
}

/// Verify a single class file for NPE, AIOOBE, CCE.
fn verify_single_class(cf_path: &Path, report: &mut Report) -> anyhow::Result<()> {
    let cf = bytecode::parse_class_file(cf_path)?;
    let file_str = cf_path.display().to_string();

    for method in &cf.methods {
        // Check 1: NullPointerException - look for aaload/aload followed by invokevirtual without null check
        check_null_dereference(&cf, method, &file_str, report);

        // Check 2: ArrayIndexOutOfBoundsException - look for iaload/ialastore without bounds check
        check_array_bounds(&cf, method, &file_str, report);

        // Check 3: ClassCastException - look for checkcast without instanceof guard
        check_class_cast(&cf, method, &file_str, report);
    }

    Ok(())
}

/// Detect potential null pointer dereferences.
fn check_null_dereference(cf: &ClassFile, method: &bytecode::Method, file: &str, report: &mut Report) {
    let code = &method.code.bytecode;
    let mut null_risk = false;
    let mut last_load_offset: u32 = 0;

    for (i, &op) in code.iter().enumerate() {
        // aconst_null (0x05) pushes null onto stack
        if op == 0x05 {
            last_load_offset = i as u32;
        }
        // aload_0..3 (0x2A-0x2D), aload (0x19) - load reference
        if (0x2A..=0x2D).contains(&op) || op == 0x19 {
            last_load_offset = i as u32;
        }
        // invokevirtual (0xB6), invokeinterface (0xB9), invokespecial (0xB7)
        if (0xB6..=0xB9).contains(&op) {
            // If we loaded a reference and immediately invoked without null check, flag it
            if last_load_offset > 0 && (i as u32 - last_load_offset) <= 4 {
                null_risk = true;
                break;
            }
        }
    }

    if null_risk {
        report.add_finding(Finding {
            id: format!("NPE-{}-{}", cf.class_name, method.name),
            severity: Severity::Warning,
            file: file.to_string(),
            line: 0,
            message: format!(
                "Potential NullPointerException in {}.{}: reference loaded without null guard before virtual call",
                cf.class_name, method.name
            ),
            invariant: format!("this != null && {} != null", method.name),
        });
    }
}

/// Detect potential array index out of bounds.
fn check_array_bounds(cf: &ClassFile, method: &bytecode::Method, file: &str, report: &mut Report) {
    let code = &method.code.bytecode;
    let mut has_array_access = false;
    let mut has_bounds_check = false;

    for (i, &op) in code.iter().enumerate() {
        // Array load/store opcodes: iaload(0x2E), laload(0x32), faload(0x36),
        // daload(0x3A), aaload(0x3E), baload(0x31), caload(0x35), saload(0x33)
        if matches!(op, 0x2E | 0x32 | 0x36 | 0x3A | 0x3E | 0x31 | 0x35 | 0x33) {
            has_array_access = true;
        }
        // Array store opcodes: iastore(0x50), lastore(0x54), fastore(0x58),
        // dastore(0x5C), aastore(0x5F)
        if matches!(op, 0x50 | 0x54 | 0x58 | 0x5C | 0x5F) {
            has_array_access = true;
        }
        // Look for a comparison (if_icmpge, if_icmplt, etc.) near array access
        if matches!(op, 0x99 | 0x9A | 0x9B | 0x9C | 0x9D | 0x9E) {
            has_bounds_check = true;
        }
        // arraylength (0xB4) often precedes bounds check
        if op == 0xB4 {
            has_bounds_check = true;
        }
    }

    if has_array_access && !has_bounds_check {
        report.add_finding(Finding {
            id: format!("AIOOBE-{}-{}", cf.class_name, method.name),
            severity: Severity::Warning,
            file: file.to_string(),
            line: 0,
            message: format!(
                "Potential ArrayIndexOutOfBoundsException in {}.{}: array access without visible bounds check",
                cf.class_name, method.name
            ),
            invariant: format!("0 <= index < {}.length", method.name),
        });
    }
}

/// Detect potential ClassCastException.
fn check_class_cast(cf: &ClassFile, method: &bytecode::Method, file: &str, report: &mut Report) {
    let code = &method.code.bytecode;
    let mut has_checkcast = false;
    let mut has_instanceof = false;

    for &op in code.iter() {
        // checkcast (0xC1)
        if op == 0xC1 {
            has_checkcast = true;
        }
        // instanceof (0xC0)
        if op == 0xC0 {
            has_instanceof = true;
        }
    }

    if has_checkcast && !has_instanceof {
        report.add_finding(Finding {
            id: format!("CCE-{}-{}", cf.class_name, method.name),
            severity: Severity::Warning,
            file: file.to_string(),
            line: 0,
            message: format!(
                "Potential ClassCastException in {}.{}: checkcast without preceding instanceof guard",
                cf.class_name, method.name
            ),
            invariant: format!("obj instanceof TargetType (before cast in {})", method.name),
        });
    }
}

/// Recursively collect .class files from a directory.
fn collect_class_files(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_class_files(&path, out)?;
        } else if path.extension().map(|e| e == "class").unwrap_or(false) {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_nonexistent_path() {
        let result = verify_class(Path::new("/nonexistent/path"), Path::new("/tmp/out.json"), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_report_generation() {
        let mut report = Report::new("test");
        report.add_finding(Finding {
            id: "NPE-001".into(),
            severity: Severity::Warning,
            file: "Test.class".into(),
            line: 0,
            message: "test".into(),
            invariant: "x != null".into(),
        });
        assert_eq!(report.total_checks, 1);
    }
}
