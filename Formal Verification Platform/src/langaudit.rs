//! Shared audit contract for the non-Rust language frontends.
//!
//! Every language engine (`src/<lang>/`) exposes one entry point,
//! `audit_path(&Path) -> Result<LangAudit, String>`, and nothing else is needed
//! to plug it into the CLI. The command is `keller-verify audit --lang <l> <path>`.
//!
//! **Audit-only by construction.** An audit reads source files and writes exactly
//! one report file. It never opens a scanned file for writing, and it refuses an
//! output path that lies inside the tree being scanned.

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

pub const LANGS: &[&str] = &["c", "cpp", "asm", "go", "java", "csharp", "ts", "sql"];

/// One defect (or unproven obligation) with the place it was found.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub file: String,
    pub line: usize,
    /// Stable defect class, e.g. `c-buffer-overflow`, `go-nil-deref`.
    pub class: String,
    /// `proved-defect` (solver found a witness), `possible-defect` (heuristic /
    /// unproven), or `info`.
    pub verdict: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct LangAudit {
    pub language: String,
    pub root: String,
    pub files_scanned: usize,
    /// Files the frontend could not parse. Reported, never silently dropped.
    pub files_unparsed: usize,
    pub functions_lowered: usize,
    pub obligations_total: usize,
    pub obligations_proved_safe: usize,
    pub findings: Vec<Finding>,
    /// Honest limits of this run (unsupported constructs, partial parses, ...).
    pub notes: Vec<String>,
}

impl LangAudit {
    pub fn new(language: &str, root: &Path) -> Self {
        LangAudit {
            language: language.to_string(),
            root: root.display().to_string(),
            ..Default::default()
        }
    }

    pub fn summary_line(&self) -> String {
        format!(
            "[keller-audit] {}: {} files ({} unparsed), {} functions, {}/{} obligations proved safe, {} findings",
            self.language,
            self.files_scanned,
            self.files_unparsed,
            self.functions_lowered,
            self.obligations_proved_safe,
            self.obligations_total,
            self.findings.len()
        )
    }
}

/// Recursively collect files with one of `exts` under `root` (or `root` itself
/// if it is a file), skipping VCS and build directories. Sorted for stable output.
pub fn collect_files(root: &Path, exts: &[&str]) -> Vec<PathBuf> {
    const SKIP: &[&str] = &[
        ".git", "node_modules", "target", "build", "dist", "bin", "obj", ".venv", "vendor",
    ];
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        if path.is_file() {
            let ok = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| exts.iter().any(|x| x.eq_ignore_ascii_case(e)))
                .unwrap_or(false);
            if ok {
                out.push(path);
            }
            continue;
        }
        let Ok(entries) = fs::read_dir(&path) else { continue };
        for entry in entries.flatten() {
            let p = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if p.is_dir() && SKIP.contains(&name.as_str()) {
                continue;
            }
            stack.push(p);
        }
    }
    out.sort();
    out
}

/// Dispatch to the engine for `lang`.
pub fn audit(lang: &str, path: &Path) -> Result<LangAudit, String> {
    match lang {
        #[cfg(feature = "lang-c")]
        "c" => crate::c::audit_path(path),
        #[cfg(feature = "lang-cpp")]
        "cpp" | "c++" => crate::cpp::audit_path(path),
        #[cfg(feature = "lang-asm")]
        "asm" => crate::asm::audit_path(path),
        #[cfg(feature = "lang-go")]
        "go" => crate::go::audit_path(path),
        #[cfg(feature = "lang-java")]
        "java" => crate::java::audit_path(path),
        #[cfg(feature = "lang-csharp")]
        "csharp" | "cs" | "c#" => crate::csharp::audit_path(path),
        #[cfg(feature = "lang-ts")]
        "ts" | "js" | "typescript" | "javascript" => crate::typescript::audit_path(path),
        #[cfg(feature = "lang-sql")]
        "sql" => crate::sql::audit_path(path),
        other if LANGS.contains(&other) => Err(format!(
            "the '{other}' engine is not built into this binary (enable cargo feature `lang-{other}`)"
        )),
        other => Err(format!(
            "unknown language '{other}'. Expected one of: {}",
            LANGS.join(", ")
        )),
    }
}

/// `keller-verify audit --lang <l> <path> [--out <file>]`. Returns the exit code.
///
/// Exit 0: audit completed (findings are data, not failure). Exit 2: could not run.
pub fn run_audit(lang: &str, path: &Path, out: Option<&Path>) -> i32 {
    if !path.exists() {
        eprintln!("[keller-audit] path does not exist: {}", path.display());
        return 2;
    }
    if let Some(out) = out {
        let root = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let parent = out.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let _ = fs::create_dir_all(parent);
        let out_abs = fs::canonicalize(parent)
            .map(|p| p.join(out.file_name().unwrap_or_default()))
            .unwrap_or_else(|_| out.to_path_buf());
        if out_abs.starts_with(&root) {
            eprintln!(
                "[keller-audit] refusing to write the report inside the audited tree ({}): audits never touch the source.",
                root.display()
            );
            return 2;
        }
    }
    match audit(lang, path) {
        Ok(report) => {
            println!("{}", report.summary_line());
            if let Some(out) = out {
                match serde_json::to_string_pretty(&report) {
                    Ok(json) => {
                        if let Err(e) = fs::write(out, json) {
                            eprintln!("[keller-audit] could not write {}: {e}", out.display());
                            return 2;
                        }
                        println!("[keller-audit] report: {}", out.display());
                    }
                    Err(e) => {
                        eprintln!("[keller-audit] could not serialise report: {e}");
                        return 2;
                    }
                }
            }
            0
        }
        Err(e) => {
            eprintln!("[keller-audit] {e}");
            2
        }
    }
}
