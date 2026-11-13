#![allow(dead_code, unused_variables, unused_imports, private_interfaces)]

//! Java source language frontend for Keller-VIR formal verification.
//!
//! Provides static verification of 5 critical defect classes in Java:
//! 1. `java-npe`: Null pointer dereferences without null check.
//! 2. `java-array-oob`: Array indexing without bounds check.
//! 3. `java-div-by-zero`: Integer division/modulo by zero without guard.
//! 4. `java-class-cast`: Downcast without preceding instanceof guard.
//! 5. `java-negative-array-size`: Array creation with potentially negative dimension.

pub mod ast;
pub mod lower;
pub mod parser;

use std::fs;
use std::path::Path;

/// Audit a path (.java file or directory containing .java files) in audit-only mode.
pub fn audit_path(path: &Path) -> Result<crate::langaudit::LangAudit, String> {
    let mut audit = crate::langaudit::LangAudit::new("java", path);
    let files = crate::langaudit::collect_files(path, &["java"]);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npe_vulnerable() {
        let src = r#"
        public class Test {
            static class Node { int val; }

            public int badParam(Node n) {
                return n.val;
            }

            public int badNull() {
                Node n = null;
                return n.val;
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.java");
        let res = verifier.verify_unit(&unit);
        let npe_findings: Vec<_> = res.findings.iter().filter(|f| f.class == "java-npe").collect();
        assert!(!npe_findings.is_empty(), "expected NPE findings");
        assert!(npe_findings.iter().any(|f| f.verdict == "proved-defect"));
        assert!(npe_findings.iter().any(|f| f.verdict == "possible-defect"));
    }

    #[test]
    fn test_npe_safe() {
        let src = r#"
        public class Test {
            static class Node { int val; }

            public int safeGuardIf(Node n) {
                if (n != null) {
                    return n.val;
                }
                return 0;
            }

            public int safeGuardReturn(Node n) {
                if (n == null) {
                    return 0;
                }
                return n.val;
            }

            public int safeNew() {
                Node n = new Node();
                return n.val;
            }

            public int safeThis() {
                return this.hashCode();
            }

            public int safeStatic() {
                return Math.max(1, 2);
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.java");
        let res = verifier.verify_unit(&unit);
        let npe_findings: Vec<_> = res.findings.iter().filter(|f| f.class == "java-npe").collect();
        assert!(npe_findings.is_empty(), "expected no NPE findings in safe code");
        assert!(res.obligations_proved_safe > 0);
    }

    #[test]
    fn test_array_oob_vulnerable() {
        let src = r#"
        public class Test {
            public int badNegative(int[] arr) {
                return arr[-1];
            }

            public int badUnchecked(int[] arr, int i) {
                return arr[i];
            }

            public int badConstExceed() {
                int[] arr = new int[5];
                return arr[10];
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.java");
        let res = verifier.verify_unit(&unit);
        let oob_findings: Vec<_> = res.findings.iter().filter(|f| f.class == "java-array-oob").collect();
        assert!(!oob_findings.is_empty(), "expected array OOB findings");
        assert!(oob_findings.iter().any(|f| f.verdict == "proved-defect"));
    }

    #[test]
    fn test_array_oob_safe() {
        let src = r#"
        public class Test {
            public int safeForLoop(int[] arr) {
                int sum = 0;
                for (int i = 0; i < arr.length; i++) {
                    sum += arr[i];
                }
                return sum;
            }

            public int safeGuarded(int[] arr, int i) {
                if (i >= 0 && i < arr.length) {
                    return arr[i];
                }
                return -1;
            }

            public int safeConst() {
                int[] arr = new int[5];
                return arr[2];
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.java");
        let res = verifier.verify_unit(&unit);
        let oob_findings: Vec<_> = res.findings.iter().filter(|f| f.class == "java-array-oob").collect();
        assert!(oob_findings.is_empty(), "expected no array OOB findings in safe code");
        assert!(res.obligations_proved_safe > 0);
    }

    #[test]
    fn test_div_by_zero_vulnerable() {
        let src = r#"
        public class Test {
            public int badConst(int x) {
                return x / 0;
            }

            public int badMod(int x) {
                return x % 0;
            }

            public int badVar(int a, int b) {
                return a / b;
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.java");
        let res = verifier.verify_unit(&unit);
        let div_findings: Vec<_> = res.findings.iter().filter(|f| f.class == "java-div-by-zero").collect();
        assert!(!div_findings.is_empty(), "expected div-by-zero findings");
        assert!(div_findings.iter().any(|f| f.verdict == "proved-defect"));
    }

    #[test]
    fn test_div_by_zero_safe() {
        let src = r#"
        public class Test {
            public int safeConst(int x) {
                return x / 2;
            }

            public int safeGuarded(int a, int b) {
                if (b != 0) {
                    return a / b;
                }
                return 0;
            }

            public int safeGuardReturn(int a, int b) {
                if (b == 0) {
                    return 0;
                }
                return a / b;
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.java");
        let res = verifier.verify_unit(&unit);
        let div_findings: Vec<_> = res.findings.iter().filter(|f| f.class == "java-div-by-zero").collect();
        assert!(div_findings.is_empty(), "expected no div-by-zero findings in safe code");
        assert!(res.obligations_proved_safe > 0);
    }

    #[test]
    fn test_class_cast_vulnerable() {
        let src = r#"
        public class Test {
            public String badCast(Object obj) {
                return (String) obj;
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.java");
        let res = verifier.verify_unit(&unit);
        let cast_findings: Vec<_> = res.findings.iter().filter(|f| f.class == "java-class-cast").collect();
        assert!(!cast_findings.is_empty(), "expected class-cast findings");
    }

    #[test]
    fn test_class_cast_safe() {
        let src = r#"
        public class Test {
            public String safeCastIf(Object obj) {
                if (obj instanceof String) {
                    return (String) obj;
                }
                return "";
            }

            public Object safeUpcast(String s) {
                return (Object) s;
            }

            public int safePrimitiveCast(double d) {
                return (int) d;
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.java");
        let res = verifier.verify_unit(&unit);
        let cast_findings: Vec<_> = res.findings.iter().filter(|f| f.class == "java-class-cast").collect();
        assert!(cast_findings.is_empty(), "expected no class-cast findings in safe code");
        assert!(res.obligations_proved_safe > 0);
    }

    #[test]
    fn test_negative_array_size_vulnerable() {
        let src = r#"
        public class Test {
            public int[] badConst() {
                return new int[-5];
            }

            public int[] badVar(int n) {
                return new int[n];
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.java");
        let res = verifier.verify_unit(&unit);
        let size_findings: Vec<_> = res.findings.iter().filter(|f| f.class == "java-negative-array-size").collect();
        assert!(!size_findings.is_empty(), "expected negative-array-size findings");
        assert!(size_findings.iter().any(|f| f.verdict == "proved-defect"));
    }

    #[test]
    fn test_negative_array_size_safe() {
        let src = r#"
        public class Test {
            public int[] safeConst() {
                return new int[10];
            }

            public int[] safeZero() {
                return new int[0];
            }

            public int[] safeGuarded(int n) {
                if (n >= 0) {
                    return new int[n];
                }
                return new int[0];
            }

            public int[] safeGuardReturn(int n) {
                if (n < 0) {
                    return new int[0];
                }
                return new int[n];
            }

            public int[] safeFromLength(int[] arr) {
                return new int[arr.length];
            }
        }
        "#;
        let unit = parser::parse(src).expect("parse failed");
        let mut verifier = lower::Verifier::new("Test.java");
        let res = verifier.verify_unit(&unit);
        let size_findings: Vec<_> = res.findings.iter().filter(|f| f.class == "java-negative-array-size").collect();
        assert!(size_findings.is_empty(), "expected no negative-array-size findings in safe code");
        assert!(res.obligations_proved_safe > 0);
    }

    #[test]
    fn test_audit_path_on_java_cases() {
        let cases_dir = Path::new("tests/java_cases");
        let audit = audit_path(cases_dir).expect("audit_path failed");
        assert_eq!(audit.files_scanned, 2);
        assert_eq!(audit.files_unparsed, 0);
        assert!(audit.functions_lowered > 0);
        assert!(audit.obligations_total > 0);
        assert!(audit.obligations_proved_safe > 0);

        let safe_findings: Vec<_> = audit
            .findings
            .iter()
            .filter(|f| f.file.ends_with("SafeCases.java"))
            .collect();
        assert!(
            safe_findings.is_empty(),
            "expected 0 findings in SafeCases.java, got: {:#?}",
            safe_findings
        );

        let vuln_findings: Vec<_> = audit
            .findings
            .iter()
            .filter(|f| f.file.ends_with("VulnCases.java"))
            .collect();
        assert!(
            !vuln_findings.is_empty(),
            "expected findings in VulnCases.java"
        );

        let classes: std::collections::HashSet<_> =
            vuln_findings.iter().map(|f| f.class.as_str()).collect();
        assert!(classes.contains("java-npe"));
        assert!(classes.contains("java-array-oob"));
        assert!(classes.contains("java-div-by-zero"));
        assert!(classes.contains("java-class-cast"));
        assert!(classes.contains("java-negative-array-size"));
    }
}
