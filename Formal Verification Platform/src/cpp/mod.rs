//! C++ Language Engine for Keller-Verify.
//!
//! Provides C++ frontend parsing, lowering to Keller-VIR SSA hypergraphs,
//! and verification of 7 critical defect classes:
//! - `cpp-iterator-invalidation`: Vector modification while iterating or using invalid iterators
//! - `cpp-pure-virtual-call`: Calling pure virtual method from constructor/destructor or deleted object
//! - `cpp-use-after-delete`: Accessing pointer after delete or delete[]
//! - `cpp-double-delete`: Deleting pointer twice without nulling
//! - `cpp-null-deref`: Pointer dereference without null check
//! - `cpp-vector-oob`: Unbounded vector index vec[i] or .at(i)
//! - `cpp-div-by-zero`: Division by variable that can evaluate to 0

pub mod ast;
pub mod parser;
pub mod lower;
pub mod analyzer;

#[allow(unused_imports)]
pub use ast::*;
pub use parser::CppParser;
#[allow(unused_imports)]
pub use lower::{lower_cpp, LowerResult, AuditReport};
pub use analyzer::Analyzer;

use crate::langaudit::{collect_files, LangAudit};
use std::fs;
use std::path::Path;

/// Entry point for the C++ frontend audit.
///
/// Audit-only by construction: reads source files and never modifies them.
pub fn audit_path(path: &Path) -> Result<LangAudit, String> {
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.display()));
    }

    let files = if path.is_file() {
        vec![path.to_path_buf()]
    } else {
        collect_files(path, &["cpp", "cc", "cxx", "h", "hpp"])
    };

    let mut audit = LangAudit::new("cpp", path);
    audit.notes.push(
        "C++ frontend with Keller-VIR SSA hypergraph invariant verification (iterator invalidation, pure virtual calls, use-after-delete, double-delete, null dereference, vector bounds, div-by-zero)".into()
    );

    for file in &files {
        audit.files_scanned += 1;
        let source = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                audit.files_unparsed += 1;
                audit.notes.push(format!("could not read {}: {}", file.display(), e));
                continue;
            }
        };

        let parser = CppParser::from_source(&source);
        let tu = parser.parse();

        // Check if file had function-like code but nothing parsed
        if tu.functions.is_empty() && tu.classes.is_empty() {
            if source.contains('(') && source.contains('{') {
                audit.files_unparsed += 1;
                audit.notes.push(format!("no functions or classes parsed in {}", file.display()));
            }
            continue;
        }

        audit.functions_lowered += tu.functions.len();

        let file_str = file.display().to_string();
        let mut analyzer = Analyzer::new(&file_str);
        analyzer.analyze_tu(&tu);

        audit.obligations_total += analyzer.obligations_total;
        audit.obligations_proved_safe += analyzer.obligations_proved_safe;
        audit.findings.extend(analyzer.findings);
    }

    Ok(audit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_audit_safe_cases() {
        let path = PathBuf::from("tests/cpp_cases/safe_cases.cpp");
        let audit = audit_path(&path).expect("audit should succeed");
        assert_eq!(audit.files_scanned, 1);
        assert_eq!(audit.files_unparsed, 0);
        assert!(audit.functions_lowered >= 5);
        assert!(audit.obligations_total > 0);
        assert!(audit.obligations_proved_safe > 0);

        // Safe cases should not have any proved defects
        let proved_defects: Vec<_> = audit
            .findings
            .iter()
            .filter(|f| f.verdict == "proved-defect")
            .collect();
        assert_eq!(
            proved_defects.len(),
            0,
            "expected 0 proved defects in safe cases, got: {:?}",
            proved_defects
        );
    }

    #[test]
    fn test_audit_vuln_cases() {
        let path = PathBuf::from("tests/cpp_cases/vuln_cases.cpp");
        let audit = audit_path(&path).expect("audit should succeed");
        assert_eq!(audit.files_scanned, 1);
        assert_eq!(audit.files_unparsed, 0);

        let classes: Vec<String> = audit.findings.iter().map(|f| f.class.clone()).collect();

        // Assert all 7 defect classes are detected
        assert!(
            classes.contains(&"cpp-iterator-invalidation".to_string()),
            "missing cpp-iterator-invalidation"
        );
        assert!(
            classes.contains(&"cpp-pure-virtual-call".to_string()),
            "missing cpp-pure-virtual-call"
        );
        assert!(
            classes.contains(&"cpp-use-after-delete".to_string()),
            "missing cpp-use-after-delete"
        );
        assert!(
            classes.contains(&"cpp-double-delete".to_string()),
            "missing cpp-double-delete"
        );
        assert!(
            classes.contains(&"cpp-null-deref".to_string()),
            "missing cpp-null-deref"
        );
        assert!(
            classes.contains(&"cpp-vector-oob".to_string()),
            "missing cpp-vector-oob"
        );
        assert!(
            classes.contains(&"cpp-div-by-zero".to_string()),
            "missing cpp-div-by-zero"
        );
    }

    #[test]
    fn test_iterator_invalidation_direct() {
        let code = r#"
            void test_inval() {
                std::vector<int> v = {1, 2, 3};
                for (auto &x : v) {
                    v.push_back(x);
                }
            }
        "#;
        let parser = CppParser::from_source(code);
        let tu = parser.parse();
        assert_eq!(tu.functions.len(), 1);

        let mut analyzer = Analyzer::new("test.cpp");
        analyzer.analyze_tu(&tu);
        assert!(analyzer.findings.iter().any(|f| f.class == "cpp-iterator-invalidation"));
    }

    #[test]
    fn test_pure_virtual_call_direct() {
        let code = r#"
            class Base {
            public:
                virtual void foo() = 0;
                Base() {
                    foo();
                }
            };
        "#;
        let parser = CppParser::from_source(code);
        let tu = parser.parse();
        assert_eq!(tu.classes.len(), 1);

        let mut analyzer = Analyzer::new("test.cpp");
        analyzer.analyze_tu(&tu);
        assert!(analyzer.findings.iter().any(|f| f.class == "cpp-pure-virtual-call"));
    }

    #[test]
    fn test_use_after_delete_direct() {
        let code = r#"
            void test_uad() {
                int *p = new int(10);
                delete p;
                *p = 20;
            }
        "#;
        let parser = CppParser::from_source(code);
        let tu = parser.parse();
        assert_eq!(tu.functions.len(), 1);

        let mut analyzer = Analyzer::new("test.cpp");
        analyzer.analyze_tu(&tu);
        assert!(analyzer.findings.iter().any(|f| f.class == "cpp-use-after-delete"));
    }

    #[test]
    fn test_double_delete_direct() {
        let code = r#"
            void test_dd() {
                int *p = new int(10);
                delete p;
                delete p;
            }
        "#;
        let parser = CppParser::from_source(code);
        let tu = parser.parse();
        assert_eq!(tu.functions.len(), 1);

        let mut analyzer = Analyzer::new("test.cpp");
        analyzer.analyze_tu(&tu);
        assert!(analyzer.findings.iter().any(|f| f.class == "cpp-double-delete"));
    }

    #[test]
    fn test_null_deref_direct() {
        let code = r#"
            void test_nd() {
                int *p = nullptr;
                *p = 42;
            }
        "#;
        let parser = CppParser::from_source(code);
        let tu = parser.parse();
        assert_eq!(tu.functions.len(), 1);

        let mut analyzer = Analyzer::new("test.cpp");
        analyzer.analyze_tu(&tu);
        assert!(analyzer.findings.iter().any(|f| f.class == "cpp-null-deref"));
    }

    #[test]
    fn test_vector_oob_direct() {
        let code = r#"
            void test_oob() {
                std::vector<int> v(5);
                v[5] = 10;
            }
        "#;
        let parser = CppParser::from_source(code);
        let tu = parser.parse();
        assert_eq!(tu.functions.len(), 1);

        let mut analyzer = Analyzer::new("test.cpp");
        analyzer.analyze_tu(&tu);
        assert!(analyzer.findings.iter().any(|f| f.class == "cpp-vector-oob"));
    }

    #[test]
    fn test_div_by_zero_direct() {
        let code = r#"
            void test_div0() {
                int x = 10 / 0;
            }
        "#;
        let parser = CppParser::from_source(code);
        let tu = parser.parse();
        assert_eq!(tu.functions.len(), 1);

        let mut analyzer = Analyzer::new("test.cpp");
        analyzer.analyze_tu(&tu);
        assert!(analyzer.findings.iter().any(|f| f.class == "cpp-div-by-zero"));
    }
}
