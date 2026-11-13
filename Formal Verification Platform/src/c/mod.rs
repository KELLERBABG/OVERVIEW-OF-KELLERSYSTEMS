//! C Language Engine for Keller-Verify.
//!
//! Provides C frontend parsing, lowering to C-RMM (Reduced Memory Model),
//! and verification of spatial & temporal memory safety invariants:
//! - `c-buffer-overflow`
//! - `c-use-after-free`
//! - `c-double-free`
//! - `c-null-deref`
//! - `c-div-by-zero`
//! - `c-signed-overflow`

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod crmm;
pub mod analyzer;

#[allow(unused_imports)]
pub use ast::*;
pub use lexer::Lexer;
pub use parser::Parser;
pub use crmm::CrmmLowering;
pub use analyzer::Analyzer;

use crate::langaudit::{collect_files, LangAudit};
use std::fs;
use std::path::Path;

/// Entry point for the C frontend audit.
///
/// Audit-only by construction: reads source files and never modifies them.
pub fn audit_path(path: &Path) -> Result<LangAudit, String> {
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.display()));
    }

    let files = if path.is_file() {
        vec![path.to_path_buf()]
    } else {
        collect_files(path, &["c"])
    };

    let mut audit = LangAudit::new("c", path);
    audit.notes.push(
        "C-RMM reduced memory model with abstract interpretation & spatial/temporal invariant checking".into()
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

        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        let tu = parser.parse_translation_unit();

        if tu.functions.is_empty() {
            // Check if source contains function-like declarations that failed to parse
            if source.contains('(') && source.contains('{') {
                audit.files_unparsed += 1;
                audit.notes.push(format!("no functions parsed in {}", file.display()));
            }
            continue;
        }

        let file_str = file.display().to_string();

        for func in &tu.functions {
            audit.functions_lowered += 1;

            // Lower to Keller-VIR C-RMM
            let mut crmm_lowering = CrmmLowering::new();
            crmm_lowering.lower_function(func);

            // Analyze invariants using C-RMM
            let mut analyzer = Analyzer::new(&file_str);
            analyzer.analyze_function(func);

            audit.obligations_total += analyzer.obligations_total;
            audit.obligations_proved_safe += analyzer.obligations_proved_safe;
            audit.findings.extend(analyzer.findings);
        }
    }

    Ok(audit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_audit_safe_cases() {
        let path = PathBuf::from("tests/c_cases/safe_cases.c");
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
        assert_eq!(proved_defects.len(), 0, "expected 0 proved defects in safe cases, got: {:?}", proved_defects);
    }

    #[test]
    fn test_audit_vuln_cases() {
        let path = PathBuf::from("tests/c_cases/vuln_cases.c");
        let audit = audit_path(&path).expect("audit should succeed");
        assert_eq!(audit.files_scanned, 1);
        assert_eq!(audit.files_unparsed, 0);

        let classes: Vec<String> = audit.findings.iter().map(|f| f.class.clone()).collect();

        // Assert all 6 defect classes are detected
        assert!(classes.contains(&"c-buffer-overflow".to_string()), "missing c-buffer-overflow");
        assert!(classes.contains(&"c-use-after-free".to_string()), "missing c-use-after-free");
        assert!(classes.contains(&"c-double-free".to_string()), "missing c-double-free");
        assert!(classes.contains(&"c-null-deref".to_string()), "missing c-null-deref");
        assert!(classes.contains(&"c-div-by-zero".to_string()), "missing c-div-by-zero");
        assert!(classes.contains(&"c-signed-overflow".to_string()), "missing c-signed-overflow");
    }

    #[test]
    fn test_buffer_overflow_direct() {
        let code = r#"
            void test_overflow() {
                int arr[5];
                arr[5] = 10;
            }
        "#;
        let mut lexer = Lexer::new(code);
        let mut parser = Parser::new(lexer.tokenize());
        let tu = parser.parse_translation_unit();
        assert_eq!(tu.functions.len(), 1);

        let mut analyzer = Analyzer::new("test.c");
        analyzer.analyze_function(&tu.functions[0]);
        assert!(analyzer.findings.iter().any(|f| f.class == "c-buffer-overflow"));
    }

    #[test]
    fn test_use_after_free_direct() {
        let code = r#"
            void test_uaf() {
                int *p = malloc(10);
                free(p);
                *p = 1;
            }
        "#;
        let mut lexer = Lexer::new(code);
        let mut parser = Parser::new(lexer.tokenize());
        let tu = parser.parse_translation_unit();
        assert_eq!(tu.functions.len(), 1);

        let mut analyzer = Analyzer::new("test.c");
        analyzer.analyze_function(&tu.functions[0]);
        assert!(analyzer.findings.iter().any(|f| f.class == "c-use-after-free"));
    }

    #[test]
    fn test_double_free_direct() {
        let code = r#"
            void test_df() {
                int *p = malloc(10);
                free(p);
                free(p);
            }
        "#;
        let mut lexer = Lexer::new(code);
        let mut parser = Parser::new(lexer.tokenize());
        let tu = parser.parse_translation_unit();
        assert_eq!(tu.functions.len(), 1);

        let mut analyzer = Analyzer::new("test.c");
        analyzer.analyze_function(&tu.functions[0]);
        assert!(analyzer.findings.iter().any(|f| f.class == "c-double-free"));
    }

    #[test]
    fn test_null_deref_direct() {
        let code = r#"
            void test_null() {
                int *p = NULL;
                *p = 42;
            }
        "#;
        let mut lexer = Lexer::new(code);
        let mut parser = Parser::new(lexer.tokenize());
        let tu = parser.parse_translation_unit();
        assert_eq!(tu.functions.len(), 1);

        let mut analyzer = Analyzer::new("test.c");
        analyzer.analyze_function(&tu.functions[0]);
        assert!(analyzer.findings.iter().any(|f| f.class == "c-null-deref"));
    }

    #[test]
    fn test_div_by_zero_direct() {
        let code = r#"
            void test_div0() {
                int x = 10 / 0;
            }
        "#;
        let mut lexer = Lexer::new(code);
        let mut parser = Parser::new(lexer.tokenize());
        let tu = parser.parse_translation_unit();
        assert_eq!(tu.functions.len(), 1);

        let mut analyzer = Analyzer::new("test.c");
        analyzer.analyze_function(&tu.functions[0]);
        assert!(analyzer.findings.iter().any(|f| f.class == "c-div-by-zero"));
    }

    #[test]
    fn test_signed_overflow_direct() {
        let code = r#"
            void test_overflow() {
                int x = 2147483647 + 1;
            }
        "#;
        let mut lexer = Lexer::new(code);
        let mut parser = Parser::new(lexer.tokenize());
        let tu = parser.parse_translation_unit();
        assert_eq!(tu.functions.len(), 1);

        let mut analyzer = Analyzer::new("test.c");
        analyzer.analyze_function(&tu.functions[0]);
        assert!(analyzer.findings.iter().any(|f| f.class == "c-signed-overflow"));
    }
}
