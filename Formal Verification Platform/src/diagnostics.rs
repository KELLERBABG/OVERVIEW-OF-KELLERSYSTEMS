use crate::types::VerificationEvent;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct DiagnosticCode {
    pub code: String,
    pub explanation: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DiagnosticSpanText {
    pub text: String,
    pub highlight_start: usize,
    pub highlight_end: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct DiagnosticSpan {
    pub file_name: String,
    pub byte_start: u32,
    pub byte_end: u32,
    pub line_start: usize,
    pub line_end: usize,
    pub column_start: usize,
    pub column_end: usize,
    pub is_primary: bool,
    pub text: Vec<DiagnosticSpanText>,
    pub label: Option<String>,
    pub suggested_replacement: Option<String>,
    pub suggestion_applicability: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DiagnosticChild {
    pub message: String,
    pub code: Option<DiagnosticCode>,
    pub level: String,
    pub spans: Vec<DiagnosticSpan>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub code: Option<DiagnosticCode>,
    pub level: String,
    pub spans: Vec<DiagnosticSpan>,
    pub children: Vec<DiagnosticChild>,
    pub rendered: Option<String>,
}

pub fn event_to_diagnostic(event: &VerificationEvent) -> Diagnostic {
    let level = match event.status.as_str() {
        "FAIL" => "error",
        "WARN" => "warning",
        _ => "note",
    };

    let code_str = if !event.defect_class.is_empty() {
        format!("keller::{}", event.defect_class)
    } else {
        format!("keller::{}", event.kind)
    };

    let line = event.line.unwrap_or(1) as usize;
    let span = DiagnosticSpan {
        file_name: event.file.clone(),
        byte_start: 0,
        byte_end: 0,
        line_start: line,
        line_end: line,
        column_start: 1,
        column_end: 1,
        is_primary: true,
        text: vec![DiagnosticSpanText {
            text: format!("fn {}", event.function),
            highlight_start: 1,
            highlight_end: 1,
        }],
        label: Some(format!(
            "in function `{}` ({})",
            event.function, event.block
        )),
        suggested_replacement: None,
        suggestion_applicability: None,
    };

    let mut children = Vec::new();
    for w in &event.witness {
        children.push(DiagnosticChild {
            message: format!(
                "counterexample witness: {} = {} (symbol: {})",
                w.input, w.value, w.symbol
            ),
            code: None,
            level: "note".to_string(),
            spans: Vec::new(),
        });
    }

    if !event.details.is_empty() {
        children.push(DiagnosticChild {
            message: format!("obligation details: {}", event.details),
            code: None,
            level: "note".to_string(),
            spans: Vec::new(),
        });
    }

    let rendered = format!(
        "{}[{}]: formal verification {}: in `{}` at {}:{}\n",
        level,
        code_str,
        event.status.to_lowercase(),
        event.function,
        event.file,
        line
    );

    Diagnostic {
        message: format!(
            "formal verification {}: in `{}`",
            event.status, event.function
        ),
        code: Some(DiagnosticCode {
            code: code_str,
            explanation: None,
        }),
        level: level.to_string(),
        spans: vec![span],
        children,
        rendered: Some(rendered),
    }
}

pub fn emit_json_diagnostics(events: &[VerificationEvent]) {
    for event in events {
        if event.status == "PASS" {
            continue;
        }
        let diag = event_to_diagnostic(event);
        if let Ok(json) = serde_json::to_string(&diag) {
            eprintln!("{json}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_to_diagnostic() {
        let event = VerificationEvent {
            crate_name: "test".to_string(),
            function: "bad_add".to_string(),
            block: "bb2".to_string(),
            status: "FAIL".to_string(),
            kind: "overflow".to_string(),
            defect_class: "overflow-add".to_string(),
            file: "src/calc.rs".to_string(),
            line: Some(15),
            details: "add overflow check".to_string(),
            witness: vec![crate::types::Witness {
                input: "a".to_string(),
                symbol: "overflow_a".to_string(),
                value: "255".to_string(),
            }],
            witness_kind: "pinned".to_string(),
        };

        let diag = event_to_diagnostic(&event);
        assert_eq!(diag.level, "error");
        assert_eq!(diag.code.as_ref().unwrap().code, "keller::overflow-add");
        assert_eq!(diag.spans[0].line_start, 15);
        assert_eq!(diag.children.len(), 2);
    }
}
