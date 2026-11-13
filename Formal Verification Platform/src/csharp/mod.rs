#![allow(dead_code, unused_variables, unused_imports, private_interfaces)]

//! C# source language frontend engine for Keller-VIR formal verification.
//!
//! Provides static verification of 5 critical defect classes in C#:
//! 1. `cs-null-reference`: Dereferencing nullable `T?` or unverified null without null-check (`if (x != null)`) or null-coalescing (`??`)
//! 2. `cs-index-out-of-range`: Array indexing `arr[i]` where index is not provably within `0 <= i < arr.Length`
//! 3. `cs-span-oob`: `Span<T>` or `ReadOnlySpan<T>` slicing/indexing beyond length
//! 4. `cs-div-by-zero`: Division/modulo by integer variable that can be zero
//! 5. `cs-invalid-cast`: Explicit casting without preceding `is` or `as` check

pub mod ast;
pub mod lower;
pub mod parser;

use std::fs;
use std::path::{Path, PathBuf};

/// Result of an audit-only verification pass over a single C# target.
#[derive(Debug, Clone)]
pub struct AuditReport {
    pub source: PathBuf,
    pub null_violations: Vec<NullViolation>,
    pub bounds_violations: Vec<BoundsViolation>,
    pub xunit_fixture: String,
}

#[derive(Debug, Clone)]
pub struct NullViolation {
    pub location: ast::SourceLoc,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct BoundsViolation {
    pub location: ast::SourceLoc,
    pub message: String,
}

/// Audit a path (.cs file or directory containing .cs files) in audit-only mode.
pub fn audit_path(path: &Path) -> Result<crate::langaudit::LangAudit, String> {
    let mut audit = crate::langaudit::LangAudit::new("csharp", path);
    let files = crate::langaudit::collect_files(path, &["cs"]);
    audit.files_scanned = files.len();

    for file in files {
        let file_str = file.display().to_string();
        let bytes = match fs::read(&file) {
            Ok(b) => b,
            Err(e) => {
                audit.files_unparsed += 1;
                audit.notes.push(format!("{}: io error: {e}", file_str));
                continue;
            }
        };

        let src = String::from_utf8_lossy(&bytes);
        match parser::parse(&src) {
            Ok(unit) => {
                let mut verifier = lower::Verifier::new(&file_str);
                let res = verifier.verify_unit(&unit);
                audit.functions_lowered += res.functions_lowered;
                audit.obligations_total += res.obligations_total;
                audit.obligations_proved_safe += res.obligations_proved_safe;
                audit.findings.extend(res.findings);
            }
            Err(e) => {
                audit.files_unparsed += 1;
                audit.notes.push(format!("{}: parse error: {e}", file_str));
            }
        }
    }

    Ok(audit)
}

/// Backward compatibility entry point: parse + lower + audit a single C# source file.
pub fn audit_source(path: &PathBuf) -> Result<AuditReport, String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;

    let unit = parser::parse(&source).map_err(|e| format!("parse error: {}", e))?;
    let mut verifier = lower::Verifier::new(&path.display().to_string());
    let res = verifier.verify_unit(&unit);

    let mut null_violations = Vec::new();
    let mut bounds_violations = Vec::new();

    for f in res.findings {
        if f.class == "cs-null-reference" {
            null_violations.push(NullViolation {
                location: ast::SourceLoc::new(f.line, 1),
                message: f.message,
            });
        } else if f.class == "cs-index-out-of-range" || f.class == "cs-span-oob" {
            bounds_violations.push(BoundsViolation {
                location: ast::SourceLoc::new(f.line, 1),
                message: f.message,
            });
        }
    }

    Ok(AuditReport {
        source: path.clone(),
        null_violations,
        bounds_violations,
        xunit_fixture: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_reference_vulnerable() {
        let src = r#"
        public class Test {
            class Node { public int Val; }

            public int BadParam(Node n) {
                return n.Val;
            }

            public int BadNullable(Node? n) {
                return n.Val;
            }

            public int BadExplicitNull() {
                Node n = null;
                return n.Val;
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.cs");
        let res = verifier.verify_unit(&unit);
        let findings: Vec<_> = res
            .findings
            .iter()
            .filter(|f| f.class == "cs-null-reference")
            .collect();
        assert_eq!(findings.len(), 3, "expected 3 null reference findings");
        assert!(findings.iter().any(|f| f.verdict == "proved-defect"));
        assert!(findings.iter().any(|f| f.verdict == "possible-defect"));
    }

    #[test]
    fn test_null_reference_safe() {
        let src = r#"
        public class Test {
            class Node { public int Val; }

            public int SafeIf(Node n) {
                if (n != null) {
                    return n.Val;
                }
                return 0;
            }

            public int SafeGuardReturn(Node n) {
                if (n == null) return 0;
                return n.Val;
            }

            public int SafeNullConditional(Node? n) {
                return n?.Val ?? 0;
            }

            public int SafeNew() {
                Node n = new Node();
                return n.Val;
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.cs");
        let res = verifier.verify_unit(&unit);
        let null_findings: Vec<_> = res
            .findings
            .iter()
            .filter(|f| f.class == "cs-null-reference")
            .collect();
        assert!(
            null_findings.is_empty(),
            "expected no null findings, got: {:?}",
            null_findings
        );
        assert!(res.obligations_proved_safe > 0);
    }

    #[test]
    fn test_index_out_of_range_vulnerable() {
        let src = r#"
        public class Test {
            public int BadNeg(int[] arr) {
                return arr[-1];
            }

            public int BadParam(int[] arr, int i) {
                return arr[i];
            }

            public int BadSize() {
                int[] arr = new int[5];
                return arr[10];
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.cs");
        let res = verifier.verify_unit(&unit);
        let findings: Vec<_> = res
            .findings
            .iter()
            .filter(|f| f.class == "cs-index-out-of-range")
            .collect();
        assert_eq!(findings.len(), 3, "expected 3 index findings");
        assert!(findings.iter().any(|f| f.verdict == "proved-defect"));
    }

    #[test]
    fn test_index_out_of_range_safe() {
        let src = r#"
        public class Test {
            public int SafeForLoop(int[] arr) {
                int sum = 0;
                for (int i = 0; i < arr.Length; i++) {
                    sum += arr[i];
                }
                return sum;
            }

            public int SafeGuarded(int[] arr, int i) {
                if (i < arr.Length) {
                    return arr[i];
                }
                return 0;
            }

            public int SafeKnownSize() {
                int[] arr = new int[5];
                return arr[2];
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.cs");
        let res = verifier.verify_unit(&unit);
        let findings: Vec<_> = res
            .findings
            .iter()
            .filter(|f| f.class == "cs-index-out-of-range")
            .collect();
        assert!(
            findings.is_empty(),
            "expected no index findings, got: {:?}",
            findings
        );
    }

    #[test]
    fn test_span_oob_vulnerable() {
        let src = r#"
        public class Test {
            public int BadSpanIndex(Span<int> span, int i) {
                return span[i];
            }

            public Span<byte> BadSpanSlice(ReadOnlySpan<byte> spanBytes, int start) {
                return spanBytes.Slice(start);
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.cs");
        let res = verifier.verify_unit(&unit);
        let findings: Vec<_> = res
            .findings
            .iter()
            .filter(|f| f.class == "cs-span-oob")
            .collect();
        assert_eq!(findings.len(), 2, "expected 2 span findings");
    }

    #[test]
    fn test_span_oob_safe() {
        let src = r#"
        public class Test {
            public int SafeSpanLoop(Span<int> span) {
                int sum = 0;
                for (int i = 0; i < span.Length; i++) {
                    sum += span[i];
                }
                return sum;
            }

            public int SafeSpanGuarded(Span<int> span, int i) {
                if (i < span.Length) {
                    return span[i];
                }
                return 0;
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.cs");
        let res = verifier.verify_unit(&unit);
        let findings: Vec<_> = res
            .findings
            .iter()
            .filter(|f| f.class == "cs-span-oob")
            .collect();
        assert!(
            findings.is_empty(),
            "expected no span findings, got: {:?}",
            findings
        );
    }

    #[test]
    fn test_div_by_zero_vulnerable() {
        let src = r#"
        public class Test {
            public int BadConst(int x) {
                return x / 0;
            }

            public int BadVar(int a, int b) {
                return a / b;
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.cs");
        let res = verifier.verify_unit(&unit);
        let findings: Vec<_> = res
            .findings
            .iter()
            .filter(|f| f.class == "cs-div-by-zero")
            .collect();
        assert_eq!(findings.len(), 2, "expected 2 div-by-zero findings");
        assert!(findings.iter().any(|f| f.verdict == "proved-defect"));
        assert!(findings.iter().any(|f| f.verdict == "possible-defect"));
    }

    #[test]
    fn test_div_by_zero_safe() {
        let src = r#"
        public class Test {
            public int SafeConst(int x) {
                return x / 2;
            }

            public int SafeGuarded(int a, int b) {
                if (b != 0) {
                    return a / b;
                }
                return 0;
            }

            public int SafeGuardReturn(int a, int b) {
                if (b == 0) return 0;
                return a / b;
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.cs");
        let res = verifier.verify_unit(&unit);
        let findings: Vec<_> = res
            .findings
            .iter()
            .filter(|f| f.class == "cs-div-by-zero")
            .collect();
        assert!(
            findings.is_empty(),
            "expected no div findings, got: {:?}",
            findings
        );
    }

    #[test]
    fn test_invalid_cast_vulnerable() {
        let src = r#"
        public class Test {
            public string BadCast(object obj) {
                return (string)obj;
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.cs");
        let res = verifier.verify_unit(&unit);
        let findings: Vec<_> = res
            .findings
            .iter()
            .filter(|f| f.class == "cs-invalid-cast")
            .collect();
        assert_eq!(findings.len(), 1, "expected 1 cast finding");
    }

    #[test]
    fn test_invalid_cast_safe() {
        let src = r#"
        public class Test {
            public string SafeIs(object obj) {
                if (obj is string) {
                    return (string)obj;
                }
                return "";
            }

            public int SafeNumericCast(double d) {
                return (int)d;
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.cs");
        let res = verifier.verify_unit(&unit);
        let findings: Vec<_> = res
            .findings
            .iter()
            .filter(|f| f.class == "cs-invalid-cast")
            .collect();
        assert!(
            findings.is_empty(),
            "expected no cast findings, got: {:?}",
            findings
        );
    }

    #[test]
    fn test_audit_path_csharp_cases() {
        let path = Path::new("tests/csharp_cases");
        let audit = audit_path(path).expect("audit_path failed");
        assert_eq!(audit.files_scanned, 2);
        assert_eq!(audit.files_unparsed, 0);
        assert!(audit.functions_lowered >= 10);
        assert!(audit.obligations_total > 0);
        assert!(audit.obligations_proved_safe > 0);
        assert!(!audit.findings.is_empty());

        let vuln_findings: Vec<_> = audit
            .findings
            .iter()
            .filter(|f| f.file.contains("VulnCases.cs"))
            .collect();
        assert!(
            vuln_findings.iter().any(|f| f.class == "cs-null-reference"),
            "expected cs-null-reference findings in VulnCases"
        );
        assert!(
            vuln_findings.iter().any(|f| f.class == "cs-index-out-of-range"),
            "expected cs-index-out-of-range findings in VulnCases"
        );
        assert!(
            vuln_findings.iter().any(|f| f.class == "cs-span-oob"),
            "expected cs-span-oob findings in VulnCases"
        );
        assert!(
            vuln_findings.iter().any(|f| f.class == "cs-div-by-zero"),
            "expected cs-div-by-zero findings in VulnCases"
        );
        assert!(
            vuln_findings.iter().any(|f| f.class == "cs-invalid-cast"),
            "expected cs-invalid-cast findings in VulnCases"
        );

        let safe_findings: Vec<_> = audit
            .findings
            .iter()
            .filter(|f| f.file.contains("SafeCases.cs"))
            .collect();
        assert!(
            safe_findings.is_empty(),
            "expected 0 findings in SafeCases, got: {:?}",
            safe_findings
        );
    }
}
