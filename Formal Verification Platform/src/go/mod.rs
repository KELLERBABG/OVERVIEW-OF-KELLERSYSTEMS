#![allow(dead_code, unused_variables, unused_imports, private_interfaces)]

pub mod ast;
pub mod parser;
pub mod lower;

use std::path::{Path, PathBuf};
use std::fs;

/// Errors produced during Go frontend analysis.
#[derive(Debug)]
pub enum GoError {
    Parse(String),
    Lower(String),
    Io(std::io::Error),
}

impl std::fmt::Display for GoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoError::Parse(msg) => write!(f, "parse error: {msg}"),
            GoError::Lower(msg) => write!(f, "lowering error: {msg}"),
            GoError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for GoError {}

/// A single diagnostic finding from the verifier.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub category: DiagCategory,
    pub message: String,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiagCategory {
    ChannelDeadlock,
    DataRace,
    NilDeref,
    SlicePanic,
    DivByZero,
}

/// Audit-only verification report. Never mutates source.
#[derive(Debug, Clone)]
pub struct VerificationReport {
    pub file: PathBuf,
    pub diagnostics: Vec<Diagnostic>,
    pub goroutine_count: usize,
    pub channel_count: usize,
    pub functions_lowered: usize,
    pub obligations_total: usize,
    pub obligations_proved_safe: usize,
    pub passed: bool,
}

/// Entry point for the Go frontend verifier.
pub struct GoVerifier {
    audit_only: bool,
}

impl GoVerifier {
    pub fn new() -> Self {
        Self { audit_only: true }
    }

    /// Verify a single Go source file in audit-only mode.
    pub fn verify_file(&self, path: &Path) -> Result<VerificationReport, GoError> {
        let source = fs::read_to_string(path).map_err(GoError::Io)?;
        let ast = parser::parse(&source).map_err(GoError::Parse)?;

        let lower_result = lower::lower(&ast);
        let diagnostics = lower_result.diagnostics;

        let passed = !diagnostics.iter().any(|d| d.severity == Severity::Error);

        Ok(VerificationReport {
            file: path.to_path_buf(),
            diagnostics,
            goroutine_count: lower_result.goroutine_count,
            channel_count: lower_result.channel_count,
            functions_lowered: lower_result.functions_lowered,
            obligations_total: lower_result.obligations_total,
            obligations_proved_safe: lower_result.obligations_proved_safe,
            passed,
        })
    }

    /// Verify all `.go` files in a directory (non-recursive).
    pub fn verify_dir(&self, dir: &Path) -> Result<Vec<VerificationReport>, GoError> {
        let mut reports = Vec::new();
        for entry in fs::read_dir(dir).map_err(GoError::Io)? {
            let entry = entry.map_err(GoError::Io)?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("go") {
                reports.push(self.verify_file(&path)?);
            }
        }
        Ok(reports)
    }

    /// Verify a benchmark repo in audit-only mode. Returns combined report.
    pub fn verify_repo(&self, root: &Path) -> Result<Vec<VerificationReport>, GoError> {
        let mut reports = Vec::new();
        walk_go_files(root, &mut reports, self)?;
        Ok(reports)
    }

    pub fn audit_only(&self) -> bool {
        self.audit_only
    }
}

impl Default for GoVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Audit a path (.go file or directory of .go files) in audit-only mode.
pub fn audit_path(path: &Path) -> Result<crate::langaudit::LangAudit, String> {
    let mut audit = crate::langaudit::LangAudit::new("go", path);
    let files = crate::langaudit::collect_files(path, &["go"]);
    audit.files_scanned = files.len();

    let verifier = GoVerifier::new();
    for file in files {
        match verifier.verify_file(&file) {
            Ok(report) => {
                audit.functions_lowered += report.functions_lowered;
                audit.obligations_total += report.obligations_total;
                audit.obligations_proved_safe += report.obligations_proved_safe;

                for d in report.diagnostics {
                    let class = match d.category {
                        DiagCategory::NilDeref => "go-nil-deref",
                        DiagCategory::SlicePanic => "go-slice-oob",
                        DiagCategory::DivByZero => "go-div-by-zero",
                        DiagCategory::ChannelDeadlock => "go-channel-deadlock",
                        DiagCategory::DataRace => "go-data-race",
                    };
                    let verdict = match d.severity {
                        Severity::Error => "proved-defect",
                        Severity::Warning => "possible-defect",
                        Severity::Info => "info",
                    };
                    audit.findings.push(crate::langaudit::Finding {
                        file: file.display().to_string(),
                        line: d.line,
                        class: class.to_string(),
                        verdict: verdict.to_string(),
                        message: d.message,
                    });
                }
            }
            Err(e) => {
                audit.files_unparsed += 1;
                audit.notes.push(format!("{}: {}", file.display(), e));
            }
        }
    }

    Ok(audit)
}

fn walk_go_files(
    dir: &Path,
    reports: &mut Vec<VerificationReport>,
    verifier: &GoVerifier,
) -> Result<(), GoError> {
    for entry in fs::read_dir(dir).map_err(GoError::Io)? {
        let entry = entry.map_err(GoError::Io)?;
        let path = entry.path();
        if path.is_dir() {
            walk_go_files(&path, reports, verifier)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("go") {
            reports.push(verifier.verify_file(&path)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nil_deref_vulnerable() {
        let src = r#"
        package main
        func badNilAssign() {
            var p *int = nil
            *p = 42
        }
        func badParam(p *int) {
            *p = 10
        }
        "#;
        let pkg = parser::parse(src).expect("parse failed");
        let res = lower::lower(&pkg);
        let nil_errs: Vec<_> = res.diagnostics.iter().filter(|d| d.category == DiagCategory::NilDeref).collect();
        assert!(!nil_errs.is_empty(), "expected nil deref diagnostics");
        assert!(nil_errs.iter().any(|d| d.severity == Severity::Error));
    }

    #[test]
    fn test_nil_deref_safe() {
        let src = r#"
        package main
        func safeCheck(p *int) {
            if p != nil {
                *p = 42
            }
        }
        func safeGuardReturn(p *int) {
            if p == nil {
                return
            }
            *p = 99
        }
        func safeAddressOf() {
            x := 10
            p := &x
            *p = 20
        }
        "#;
        let pkg = parser::parse(src).expect("parse failed");
        println!("FUNCS: {:#?}", pkg.functions);
        let res = lower::lower(&pkg);
        let nil_errs: Vec<_> = res.diagnostics.iter().filter(|d| d.category == DiagCategory::NilDeref).collect();
        assert!(nil_errs.is_empty(), "expected no nil deref diagnostics in safe code");
        assert!(res.obligations_proved_safe > 0);
    }

    #[test]
    fn test_slice_oob_vulnerable() {
        let src = r#"
        package main
        func badNegative() {
            s := []int{1, 2, 3}
            _ = s[-1]
        }
        func badUnbounded(s []int, i int) int {
            return s[i]
        }
        "#;
        let pkg = parser::parse(src).expect("parse failed");
        println!("FUNCS: {:#?}", pkg.functions);
        let res = lower::lower(&pkg);
        let oob_errs: Vec<_> = res.diagnostics.iter().filter(|d| d.category == DiagCategory::SlicePanic).collect();
        assert!(!oob_errs.is_empty(), "expected slice oob diagnostics");
        assert!(oob_errs.iter().any(|d| d.severity == Severity::Error));
    }

    #[test]
    fn test_slice_oob_safe() {
        let src = r#"
        package main
        func safeGuarded(s []int, i int) int {
            if i < len(s) {
                return s[i]
            }
            return 0
        }
        func safeRange(s []int) {
            for i := range s {
                _ = s[i]
            }
        }
        func safeForLen(s []int) {
            for i := 0; i < len(s); i++ {
                _ = s[i]
            }
        }
        "#;
        let pkg = parser::parse(src).expect("parse failed");
        println!("FUNCS: {:#?}", pkg.functions);
        let res = lower::lower(&pkg);
        let oob_errs: Vec<_> = res.diagnostics.iter().filter(|d| d.category == DiagCategory::SlicePanic).collect();
        assert!(oob_errs.is_empty(), "expected no slice oob diagnostics in safe code");
        assert!(res.obligations_proved_safe > 0);
    }

    #[test]
    fn test_div_by_zero_vulnerable() {
        let src = r#"
        package main
        func divZeroConst() int {
            x := 42 / 0
            return x
        }
        func modZeroConst() int {
            return 10 % 0
        }
        func divParam(a, b int) int {
            return a / b
        }
        "#;
        let pkg = parser::parse(src).expect("parse failed");
        println!("FUNCS: {:#?}", pkg.functions);
        let res = lower::lower(&pkg);
        let div_errs: Vec<_> = res.diagnostics.iter().filter(|d| d.category == DiagCategory::DivByZero).collect();
        assert!(!div_errs.is_empty(), "expected div-by-zero diagnostics");
        assert!(div_errs.iter().any(|d| d.severity == Severity::Error));
    }

    #[test]
    fn test_div_by_zero_safe() {
        let src = r#"
        package main
        func safeDivConst(x int) int {
            return x / 2
        }
        func safeDivGuarded(a, b int) int {
            if b != 0 {
                return a / b
            }
            return 0
        }
        func safeDivGuardExit(a, b int) int {
            if b == 0 {
                return 0
            }
            return a / b
        }
        "#;
        let pkg = parser::parse(src).expect("parse failed");
        println!("FUNCS: {:#?}", pkg.functions);
        let res = lower::lower(&pkg);
        let div_errs: Vec<_> = res.diagnostics.iter().filter(|d| d.category == DiagCategory::DivByZero).collect();
        assert!(div_errs.is_empty(), "expected no div-by-zero diagnostics in safe code");
        assert!(res.obligations_proved_safe > 0);
    }

    #[test]
    fn test_data_race_vulnerable() {
        let src = r#"
        package main
        func raceLoop() {
            for i := 0; i < 10; i++ {
                go func() {
                    println(i)
                }()
            }
        }
        func raceSharedVar() {
            count := 0
            go func() {
                count = 1
            }()
        }
        "#;
        let pkg = parser::parse(src).expect("parse failed");
        println!("FUNCS: {:#?}", pkg.functions);
        let res = lower::lower(&pkg);
        let race_errs: Vec<_> = res.diagnostics.iter().filter(|d| d.category == DiagCategory::DataRace).collect();
        assert!(!race_errs.is_empty(), "expected data race diagnostics");
    }

    #[test]
    fn test_channel_deadlock_vulnerable() {
        let src = r#"
        package main
        func sendClosed() {
            ch := make(chan int)
            close(ch)
            ch <- 1
        }
        func unbufferedSingle() {
            ch := make(chan int)
            ch <- 42
        }
        "#;
        let pkg = parser::parse(src).expect("parse failed");
        println!("FUNCS: {:#?}", pkg.functions);
        let res = lower::lower(&pkg);
        let dl_errs: Vec<_> = res.diagnostics.iter().filter(|d| d.category == DiagCategory::ChannelDeadlock).collect();
        assert!(!dl_errs.is_empty(), "expected channel deadlock diagnostics");
        assert!(dl_errs.iter().any(|d| d.severity == Severity::Error));
    }

    #[test]
    fn test_channel_deadlock_safe() {
        let src = r#"
        package main
        func safeBuffered() {
            ch := make(chan int, 10)
            ch <- 42
        }
        func safeSelect() {
            ch := make(chan int)
            select {
            case ch <- 42:
            default:
            }
        }
        "#;
        let pkg = parser::parse(src).expect("parse failed");
        let res = lower::lower(&pkg);
        let dl_errs: Vec<_> = res.diagnostics.iter().filter(|d| d.category == DiagCategory::ChannelDeadlock).collect();
        assert!(dl_errs.is_empty(), "expected no channel deadlock diagnostics in safe code");
    }

    #[test]
    fn test_benchmark_dynamic() {
        let p = Path::new("benchmarks/repos/go/dynamic");
        if p.exists() {
            let res = audit_path(p).expect("audit failed");
            println!("Dynamic audit: {}", res.summary_line());
        }
    }
}
