#![allow(dead_code, unused_variables, unused_imports)]

//! TypeScript and modern JavaScript frontend engine for Keller Verify.
//!
//! Provides static verification and obligation tracking for 6 defect classes:
//! 1. `ts-null-undefined-deref`
//! 2. `ts-type-error-call`
//! 3. `ts-array-oob`
//! 4. `ts-nan-propagation`
//! 5. `ts-prototype-pollution`
//! 6. `ts-div-by-zero`

pub mod ast;
pub mod lower;
pub mod parser;

use std::fs;
use std::path::Path;

/// Audit a path (.ts / .js file or directory containing TypeScript/JavaScript files) in audit-only mode.
pub fn audit_path(path: &Path) -> Result<crate::langaudit::LangAudit, String> {
    let mut audit = crate::langaudit::LangAudit::new("ts", path);
    let files = crate::langaudit::collect_files(
        path,
        &["ts", "js", "tsx", "jsx", "mts", "cts", "mjs", "cjs"],
    );
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
        match parser::parse(&src, &file_str) {
            Ok(source_file) => {
                let mut verifier = lower::Verifier::new(&file_str);
                verifier.verify_source_file(&source_file);
                audit.functions_lowered += verifier.functions_lowered;
                audit.obligations_total += verifier.obligations_total;
                audit.obligations_proved_safe += verifier.obligations_proved_safe;
                audit.findings.extend(verifier.findings);
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
    fn test_null_undefined_deref_vulnerable() {
        let src = r#"
        function badNullDeref(node: any) {
            return node.value;
        }

        function badExplicitNull() {
            let x = null;
            return x.foo();
        }

        function badExplicitUndefined() {
            let y = undefined;
            return y.bar;
        }
        "#;
        let file = parser::parse(src, "test.ts").expect("parse failed");
        let mut verifier = lower::Verifier::new("test.ts");
        verifier.verify_source_file(&file);
        let null_findings: Vec<_> = verifier
            .findings
            .iter()
            .filter(|f| f.class == "ts-null-undefined-deref")
            .collect();
        assert!(!null_findings.is_empty(), "expected null deref findings");
        assert!(null_findings.iter().any(|f| f.verdict == "proved-defect"));
        assert!(null_findings.iter().any(|f| f.verdict == "possible-defect"));
    }

    #[test]
    fn test_null_undefined_deref_safe() {
        let src = r#"
        function safeGuard(node: any) {
            if (node !== null && node !== undefined) {
                return node.value;
            }
            return null;
        }

        function safeOptionalChaining(node: any) {
            return node?.value;
        }

        function safeNullishCoalesce(node: any) {
            const fallback = { value: 42 };
            return (node ?? fallback).value;
        }

        function safeTruthiness(node: any) {
            if (!node) return 0;
            return node.value;
        }
        "#;
        let file = parser::parse(src, "test.ts").expect("parse failed");
        let mut verifier = lower::Verifier::new("test.ts");
        verifier.verify_source_file(&file);
        let null_defects: Vec<_> = verifier
            .findings
            .iter()
            .filter(|f| f.class == "ts-null-undefined-deref" && f.verdict != "info")
            .collect();
        assert!(
            null_defects.is_empty(),
            "expected no null deref defects in safe code, got: {:?}",
            null_defects
        );
        assert!(verifier.obligations_proved_safe > 0);
    }

    #[test]
    fn test_type_error_call_vulnerable() {
        let src = r#"
        function badLiteralCall() {
            const num = 42;
            num();
        }

        function badUnverifiedParam(cb: any) {
            cb();
        }
        "#;
        let file = parser::parse(src, "test.ts").expect("parse failed");
        let mut verifier = lower::Verifier::new("test.ts");
        verifier.verify_source_file(&file);
        let call_findings: Vec<_> = verifier
            .findings
            .iter()
            .filter(|f| f.class == "ts-type-error-call")
            .collect();
        assert!(!call_findings.is_empty(), "expected type error call findings");
        assert!(call_findings.iter().any(|f| f.verdict == "proved-defect"));
        assert!(call_findings.iter().any(|f| f.verdict == "possible-defect"));
    }

    #[test]
    fn test_type_error_call_safe() {
        let src = r#"
        function helper() {
            return 1;
        }

        function safeCall(cb: any) {
            helper();
            if (typeof cb === 'function') {
                cb();
            }
            cb?.();
        }
        "#;
        let file = parser::parse(src, "test.ts").expect("parse failed");
        let mut verifier = lower::Verifier::new("test.ts");
        verifier.verify_source_file(&file);
        let call_defects: Vec<_> = verifier
            .findings
            .iter()
            .filter(|f| f.class == "ts-type-error-call" && f.verdict != "info")
            .collect();
        assert!(
            call_defects.is_empty(),
            "expected no type error call defects in safe code, got: {:?}",
            call_defects
        );
    }

    #[test]
    fn test_array_oob_vulnerable() {
        let src = r#"
        function badNegativeIndex(arr: number[]) {
            return arr[-1];
        }

        function badUnboundedIndex(arr: number[], i: number) {
            return arr[i];
        }
        "#;
        let file = parser::parse(src, "test.ts").expect("parse failed");
        let mut verifier = lower::Verifier::new("test.ts");
        verifier.verify_source_file(&file);
        let oob_findings: Vec<_> = verifier
            .findings
            .iter()
            .filter(|f| f.class == "ts-array-oob")
            .collect();
        assert!(!oob_findings.is_empty(), "expected array oob findings");
        assert!(oob_findings.iter().any(|f| f.verdict == "proved-defect"));
        assert!(oob_findings.iter().any(|f| f.verdict == "possible-defect"));
    }

    #[test]
    fn test_array_oob_safe() {
        let src = r#"
        function safeLoop(arr: number[]) {
            let sum = 0;
            for (let i = 0; i < arr.length; i++) {
                sum += arr[i];
            }
            return sum;
        }

        function safeGuard(arr: number[], i: number) {
            if (i < 0 || i >= arr.length) return 0;
            return arr[i];
        }
        "#;
        let file = parser::parse(src, "test.ts").expect("parse failed");
        let mut verifier = lower::Verifier::new("test.ts");
        verifier.verify_source_file(&file);
        let oob_defects: Vec<_> = verifier
            .findings
            .iter()
            .filter(|f| f.class == "ts-array-oob" && f.verdict != "info")
            .collect();
        assert!(
            oob_defects.is_empty(),
            "expected no array oob defects in safe code, got: {:?}",
            oob_defects
        );
    }

    #[test]
    fn test_nan_propagation_vulnerable() {
        let src = r#"
        function badNanLiteral() {
            return NaN + 10;
        }

        function badUndefinedArithmetic() {
            return undefined * 2;
        }

        function badUnverifiedParam(x: any) {
            return x * 5;
        }
        "#;
        let file = parser::parse(src, "test.ts").expect("parse failed");
        let mut verifier = lower::Verifier::new("test.ts");
        verifier.verify_source_file(&file);
        let nan_findings: Vec<_> = verifier
            .findings
            .iter()
            .filter(|f| f.class == "ts-nan-propagation")
            .collect();
        assert!(!nan_findings.is_empty(), "expected nan propagation findings");
        assert!(nan_findings.iter().any(|f| f.verdict == "proved-defect"));
        assert!(nan_findings.iter().any(|f| f.verdict == "possible-defect"));
    }

    #[test]
    fn test_nan_propagation_safe() {
        let src = r#"
        function safeArithmetic(a: number, b: number) {
            const x = 10;
            const y = 20;
            const z = x + y;
            if (!Number.isNaN(a)) {
                return a + z;
            }
            return 0;
        }
        "#;
        let file = parser::parse(src, "test.ts").expect("parse failed");
        let mut verifier = lower::Verifier::new("test.ts");
        verifier.verify_source_file(&file);
        let nan_defects: Vec<_> = verifier
            .findings
            .iter()
            .filter(|f| f.class == "ts-nan-propagation" && f.verdict != "info")
            .collect();
        assert!(
            nan_defects.is_empty(),
            "expected no nan propagation defects in safe code, got: {:?}",
            nan_defects
        );
    }

    #[test]
    fn test_prototype_pollution_vulnerable() {
        let src = r#"
        function badProtoDirect(obj: any, val: any) {
            obj.__proto__ = val;
        }

        function badProtoIndex(obj: any, val: any) {
            obj["__proto__"] = val;
        }

        function badConstructorProto(obj: any, val: any) {
            obj.constructor.prototype = val;
        }

        function badDynamicKey(obj: any, key: any, val: any) {
            obj[key] = val;
        }
        "#;
        let file = parser::parse(src, "test.ts").expect("parse failed");
        let mut verifier = lower::Verifier::new("test.ts");
        verifier.verify_source_file(&file);
        let proto_findings: Vec<_> = verifier
            .findings
            .iter()
            .filter(|f| f.class == "ts-prototype-pollution")
            .collect();
        assert!(!proto_findings.is_empty(), "expected prototype pollution findings");
        assert!(proto_findings.iter().any(|f| f.verdict == "proved-defect"));
        assert!(proto_findings.iter().any(|f| f.verdict == "possible-defect"));
    }

    #[test]
    fn test_prototype_pollution_safe() {
        let src = r#"
        function safeFixedProps(obj: any) {
            obj.name = "safe";
            obj["title"] = "ok";
        }

        function safeGuardedKey(obj: any, key: string, val: any) {
            if (key !== '__proto__' && key !== 'constructor' && key !== 'prototype') {
                obj[key] = val;
            }
        }
        "#;
        let file = parser::parse(src, "test.ts").expect("parse failed");
        let mut verifier = lower::Verifier::new("test.ts");
        verifier.verify_source_file(&file);
        let proto_defects: Vec<_> = verifier
            .findings
            .iter()
            .filter(|f| f.class == "ts-prototype-pollution" && f.verdict != "info")
            .collect();
        assert!(
            proto_defects.is_empty(),
            "expected no prototype pollution defects in safe code, got: {:?}",
            proto_defects
        );
    }

    #[test]
    fn test_div_by_zero_vulnerable() {
        let src = r#"
        function badDivZeroLiteral(x: number) {
            return x / 0;
        }

        function badModZeroLiteral(x: number) {
            return x % 0;
        }

        function badDivVariable(a: number, b: number) {
            return a / b;
        }
        "#;
        let file = parser::parse(src, "test.ts").expect("parse failed");
        let mut verifier = lower::Verifier::new("test.ts");
        verifier.verify_source_file(&file);
        let div_findings: Vec<_> = verifier
            .findings
            .iter()
            .filter(|f| f.class == "ts-div-by-zero")
            .collect();
        assert!(!div_findings.is_empty(), "expected div by zero findings");
        assert!(div_findings.iter().any(|f| f.verdict == "proved-defect"));
        assert!(div_findings.iter().any(|f| f.verdict == "possible-defect"));
    }

    #[test]
    fn test_div_by_zero_safe() {
        let src = r#"
        function safeDivConst(x: number) {
            return x / 2;
        }

        function safeDivGuarded(a: number, b: number) {
            if (b === 0) return 0;
            return a / b;
        }

        function safeDivPositive(a: number, b: number) {
            if (b > 0) {
                return a / b;
            }
            return 0;
        }
        "#;
        let file = parser::parse(src, "test.ts").expect("parse failed");
        let mut verifier = lower::Verifier::new("test.ts");
        verifier.verify_source_file(&file);
        let div_defects: Vec<_> = verifier
            .findings
            .iter()
            .filter(|f| f.class == "ts-div-by-zero" && f.verdict != "info")
            .collect();
        assert!(
            div_defects.is_empty(),
            "expected no div by zero defects in safe code, got: {:?}",
            div_defects
        );
    }
}
