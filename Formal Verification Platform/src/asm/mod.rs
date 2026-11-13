//! Assembly Micro-Op Verifier (TASK-05)
//!
//! Analyzes x86_64 (AT&T & Intel syntax) and AArch64 assembly sources,
//! verifying memory safety, stack frame integrity, callee-saved register
//! preservation, alignment invariants, and arithmetic safety under C-RMM.
//!
//! Entry point: `audit_path(path: &Path) -> Result<LangAudit, String>`.
//! Audit-only by construction: reads source files and never mutates them.

#![allow(unused_imports, dead_code)]

pub mod ast;
pub mod parser;
pub mod lower;

pub use ast::{Arch, Function, Instruction, Operand, Reg, RegClass};
pub use parser::{ParseError, Parser};
pub use lower::{LowerError, LoweringResult, MicroOp, OpKind, Verifier};

use std::fs;
use std::path::Path;
use crate::langaudit::{collect_files, LangAudit};

/// Audit a path (.s/.S file or directory of assembly files) in audit-only mode.
pub fn audit_path(path: &Path) -> Result<LangAudit, String> {
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.display()));
    }

    let files = if path.is_file() {
        vec![path.to_path_buf()]
    } else {
        collect_files(path, &["s", "S"])
    };

    let mut audit = LangAudit::new("asm", path);
    audit.notes.push("Assembly (x86_64 / AArch64) C-RMM static verification engine".into());

    for file in &files {
        audit.files_scanned += 1;
        let file_str = file.display().to_string();

        let content = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                audit.files_unparsed += 1;
                audit.notes.push(format!("could not read {}: {}", file_str, e));
                continue;
            }
        };

        let arch = Parser::detect_arch(&file_str, &content);
        let (functions, unparsed_lines) = Parser::parse_file(arch, &content);

        if functions.is_empty() {
            // Check if source contains executable assembly instructions that could not be parsed
            let has_code_heuristics = content.lines().any(|l| {
                let t = l.trim();
                !t.is_empty()
                    && !t.starts_with('#')
                    && !t.starts_with('.')
                    && !t.starts_with("/*")
                    && !t.starts_with("//")
                    && (t.contains(':') || t.contains("ret") || t.contains("mov") || t.contains("push"))
            });

            if has_code_heuristics || unparsed_lines > 5 {
                audit.files_unparsed += 1;
                audit.notes.push(format!("no functions parsed from {}", file_str));
            }
            continue;
        }

        let verifier = Verifier::new(arch);
        for func in &functions {
            let res = verifier.verify_function(func, &file_str);
            audit.functions_lowered += res.functions_lowered;
            audit.obligations_total += res.obligations_total;
            audit.obligations_proved_safe += res.obligations_proved_safe;
            audit.findings.extend(res.findings);
        }
    }

    Ok(audit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_stack_smash_vuln_entry() {
        let src = r#"
        .globl vuln_entry_smash
        vuln_entry_smash:
            movq $0, (%rsp)
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::X86_64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::X86_64);
        let res = verifier.verify_function(&funcs[0], "test.s");
        let smash = res.findings.iter().find(|f| f.class == "asm-stack-smash");
        assert!(smash.is_some(), "expected stack-smash finding for write to (%rsp)");
        assert_eq!(smash.unwrap().verdict, "proved-defect");
    }

    #[test]
    fn test_stack_smash_vuln_overflow() {
        let src = r#"
        .globl vuln_overflow
        vuln_overflow:
            subq $32, %rsp
            movq %rax, 40(%rsp)
            addq $32, %rsp
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::X86_64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::X86_64);
        let res = verifier.verify_function(&funcs[0], "test.s");
        let smash = res.findings.iter().find(|f| f.class == "asm-stack-smash");
        assert!(smash.is_some(), "expected stack-smash finding for write exceeding frame");
        assert_eq!(smash.unwrap().verdict, "proved-defect");
    }

    #[test]
    fn test_stack_smash_safe() {
        let src = r#"
        .globl safe_stack
        safe_stack:
            subq $32, %rsp
            movq %rax, 16(%rsp)
            addq $32, %rsp
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::X86_64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::X86_64);
        let res = verifier.verify_function(&funcs[0], "test.s");
        let smash = res.findings.iter().find(|f| f.class == "asm-stack-smash");
        assert!(smash.is_none(), "unexpected stack smash finding in safe code");
        assert!(res.obligations_proved_safe > 0);
    }

    #[test]
    fn test_unaligned_simd_vuln() {
        let src = r#"
        .globl vuln_simd
        vuln_simd:
            subq $32, %rsp
            movaps 8(%rsp), %xmm0
            addq $32, %rsp
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::X86_64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::X86_64);
        let res = verifier.verify_function(&funcs[0], "test.s");
        let simd = res.findings.iter().find(|f| f.class == "asm-unaligned-simd");
        assert!(simd.is_some(), "expected unaligned SIMD finding");
        assert_eq!(simd.unwrap().verdict, "proved-defect");
    }

    #[test]
    fn test_unaligned_simd_safe() {
        let src = r#"
        .globl safe_simd
        safe_simd:
            pushq %rbp
            movq %rsp, %rbp
            subq $32, %rsp
            movaps -16(%rbp), %xmm0
            leave
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::X86_64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::X86_64);
        let res = verifier.verify_function(&funcs[0], "test.s");
        let simd = res.findings.iter().find(|f| f.class == "asm-unaligned-simd");
        assert!(simd.is_none(), "unexpected unaligned SIMD finding in safe aligned code");
    }

    #[test]
    fn test_callee_saved_clobber_vuln() {
        let src = r#"
        .globl vuln_clobber
        vuln_clobber:
            movq $42, %rbx
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::X86_64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::X86_64);
        let res = verifier.verify_function(&funcs[0], "test.s");
        let clobber = res.findings.iter().find(|f| f.class == "asm-callee-saved-clobber");
        assert!(clobber.is_some(), "expected callee-saved clobber finding for rbx");
        assert_eq!(clobber.unwrap().verdict, "proved-defect");
    }

    #[test]
    fn test_callee_saved_clobber_safe() {
        let src = r#"
        .globl safe_clobber
        safe_clobber:
            pushq %rbx
            movq $42, %rbx
            popq %rbx
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::X86_64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::X86_64);
        let res = verifier.verify_function(&funcs[0], "test.s");
        let clobber = res.findings.iter().find(|f| f.class == "asm-callee-saved-clobber");
        assert!(clobber.is_none(), "unexpected callee-saved clobber finding in safe code");
    }

    #[test]
    fn test_stack_imbalance_vuln() {
        let src = r#"
        .globl vuln_imbalance
        vuln_imbalance:
            subq $16, %rsp
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::X86_64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::X86_64);
        let res = verifier.verify_function(&funcs[0], "test.s");
        let imbal = res.findings.iter().find(|f| f.class == "asm-stack-imbalance");
        assert!(imbal.is_some(), "expected stack imbalance finding");
        assert_eq!(imbal.unwrap().verdict, "proved-defect");
    }

    #[test]
    fn test_stack_imbalance_safe() {
        let src = r#"
        .globl safe_balance
        safe_balance:
            subq $16, %rsp
            addq $16, %rsp
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::X86_64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::X86_64);
        let res = verifier.verify_function(&funcs[0], "test.s");
        let imbal = res.findings.iter().find(|f| f.class == "asm-stack-imbalance");
        assert!(imbal.is_none(), "unexpected stack imbalance finding in safe code");
    }

    #[test]
    fn test_div_by_zero_vuln_constant() {
        let src = r#"
        .globl vuln_div0
        vuln_div0:
            xorq %rcx, %rcx
            idivq %rcx
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::X86_64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::X86_64);
        let res = verifier.verify_function(&funcs[0], "test.s");
        let div0 = res.findings.iter().find(|f| f.class == "asm-div-by-zero");
        assert!(div0.is_some(), "expected div by zero finding");
        assert_eq!(div0.unwrap().verdict, "proved-defect");
    }

    #[test]
    fn test_div_by_zero_vuln_unguarded() {
        let src = r#"
        .globl vuln_div_unguarded
        vuln_div_unguarded:
            idivq %rsi
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::X86_64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::X86_64);
        let res = verifier.verify_function(&funcs[0], "test.s");
        let div0 = res.findings.iter().find(|f| f.class == "asm-div-by-zero");
        assert!(div0.is_some(), "expected div by zero finding for unguarded divisor");
        assert_eq!(div0.unwrap().verdict, "possible-defect");
    }

    #[test]
    fn test_div_by_zero_safe_guarded() {
        let src = r#"
        .globl safe_div
        safe_div:
            testq %rsi, %rsi
            jz 1f
            idivq %rsi
        1:
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::X86_64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::X86_64);
        let res = verifier.verify_function(&funcs[0], "test.s");
        let div0 = res.findings.iter().find(|f| f.class == "asm-div-by-zero");
        assert!(div0.is_none(), "unexpected div by zero finding in guarded code");
    }

    #[test]
    fn test_intel_syntax() {
        let src = r#"
        vuln_intel:
            sub rsp, 32
            mov [rsp + 40], rax
            add rsp, 32
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::X86_64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::X86_64);
        let res = verifier.verify_function(&funcs[0], "intel.s");
        let smash = res.findings.iter().find(|f| f.class == "asm-stack-smash");
        assert!(smash.is_some(), "expected stack smash in Intel syntax");
    }

    #[test]
    fn test_aarch64_clobber_and_imbalance() {
        let src = r#"
        vuln_aarch64:
            sub sp, sp, #32
            mov x19, #42
            sdiv x0, x1, xzr
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::AArch64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::AArch64);
        let res = verifier.verify_function(&funcs[0], "aarch64.s");

        assert!(res.findings.iter().any(|f| f.class == "asm-stack-imbalance"));
        assert!(res.findings.iter().any(|f| f.class == "asm-callee-saved-clobber"));
        assert!(res.findings.iter().any(|f| f.class == "asm-div-by-zero"));
    }

    #[test]
    fn test_aarch64_safe() {
        let src = r#"
        safe_aarch64:
            stp x29, x30, [sp, #-32]!
            stp x19, x20, [sp, #16]
            mov x19, #42
            ldp x19, x20, [sp, #16]
            ldp x29, x30, [sp], #32
            ret
        "#;
        let (funcs, _) = Parser::parse_file(Arch::AArch64, src);
        assert_eq!(funcs.len(), 1);
        let verifier = Verifier::new(Arch::AArch64);
        let res = verifier.verify_function(&funcs[0], "safe_aarch64.s");
        assert!(res.findings.is_empty(), "expected 0 findings in safe aarch64 code: {:?}", res.findings);
    }

    #[test]
    fn test_asm_cases_suite() {
        let path = Path::new("tests/asm_cases");
        let audit = audit_path(path).expect("audit_path failed on tests/asm_cases");
        assert_eq!(audit.files_scanned, 2);
        assert_eq!(audit.files_unparsed, 0);
        assert!(audit.functions_lowered >= 15, "functions: {}", audit.functions_lowered);
        assert!(audit.obligations_total > 0);
        assert!(audit.obligations_proved_safe > 0);

        let classes: std::collections::HashSet<&str> = audit.findings.iter().map(|f| f.class.as_str()).collect();
        assert!(classes.contains("asm-stack-smash"), "missing asm-stack-smash: {:?}", classes);
        assert!(classes.contains("asm-unaligned-simd"), "missing asm-unaligned-simd: {:?}", classes);
        assert!(classes.contains("asm-callee-saved-clobber"), "missing asm-callee-saved-clobber: {:?}", classes);
        assert!(classes.contains("asm-stack-imbalance"), "missing asm-stack-imbalance: {:?}", classes);
        assert!(classes.contains("asm-div-by-zero"), "missing asm-div-by-zero: {:?}", classes);
    }
}
