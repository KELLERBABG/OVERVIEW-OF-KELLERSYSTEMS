//! Lightweight C++ parser extracting verification-relevant constructs.
//!
//! Extracts:
//! - Class declarations with virtual/pure-virtual methods, constructors, destructors
//! - Functions and methods (standalone, namespace, class methods)
//! - STL container declarations and operations
//! - Virtual call sites

#![allow(dead_code)]

use crate::cpp::ast::*;
use std::fs;

pub struct CppParser {
    pub source: String,
    pub lines: Vec<String>,
    pub clean_lines: Vec<(usize, String)>, // (1-based line number, stripped line)
}

impl CppParser {
    pub fn from_source(source: &str) -> Self {
        let lines: Vec<String> = source.lines().map(|l| l.to_string()).collect();
        let clean_lines = Self::strip_comments_preserving_lines(source);
        Self {
            source: source.to_string(),
            lines,
            clean_lines,
        }
    }

    pub fn from_file(path: &str) -> Result<Self, String> {
        let source = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path, e))?;
        Ok(Self::from_source(&source))
    }

    /// Strip single-line `//` and multi-line `/* */` comments while preserving
    /// exact 1-based line numbers.
    pub fn strip_comments_preserving_lines(source: &str) -> Vec<(usize, String)> {
        let mut result = Vec::new();
        let mut in_block_comment = false;
        let mut current_line = String::new();
        let mut line_num = 1;

        let chars: Vec<char> = source.chars().collect();
        let len = chars.len();
        let mut i = 0;
        let mut in_string = false;
        let mut in_char = false;

        while i < len {
            let ch = chars[i];

            if in_block_comment {
                if ch == '*' && i + 1 < len && chars[i + 1] == '/' {
                    in_block_comment = false;
                    current_line.push(' ');
                    current_line.push(' ');
                    i += 2;
                    continue;
                } else if ch == '\n' {
                    result.push((line_num, current_line.clone()));
                    current_line.clear();
                    line_num += 1;
                    i += 1;
                    continue;
                } else {
                    current_line.push(' ');
                    i += 1;
                    continue;
                }
            }

            if in_string {
                if ch == '\\' && i + 1 < len {
                    current_line.push(ch);
                    current_line.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if ch == '"' {
                    in_string = false;
                }
                current_line.push(ch);
                i += 1;
                continue;
            }

            if in_char {
                if ch == '\\' && i + 1 < len {
                    current_line.push(ch);
                    current_line.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if ch == '\'' {
                    in_char = false;
                }
                current_line.push(ch);
                i += 1;
                continue;
            }

            // Not inside comment or string
            if ch == '"' {
                in_string = true;
                current_line.push(ch);
                i += 1;
                continue;
            }

            if ch == '\'' {
                in_char = true;
                current_line.push(ch);
                i += 1;
                continue;
            }

            if ch == '/' && i + 1 < len && chars[i + 1] == '/' {
                // Line comment: skip rest of line
                while i < len && chars[i] != '\n' {
                    current_line.push(' ');
                    i += 1;
                }
                continue;
            }

            if ch == '/' && i + 1 < len && chars[i + 1] == '*' {
                in_block_comment = true;
                current_line.push(' ');
                current_line.push(' ');
                i += 2;
                continue;
            }

            if ch == '\n' {
                result.push((line_num, current_line.clone()));
                current_line.clear();
                line_num += 1;
                i += 1;
                continue;
            }

            current_line.push(ch);
            i += 1;
        }

        if !current_line.is_empty() || line_num <= source.lines().count() {
            result.push((line_num, current_line));
        }

        result
    }

    /// Parse the translation unit, extracting all verification-relevant nodes.
    pub fn parse(&self) -> TranslationUnit {
        let mut tu = TranslationUnit::new();
        self.parse_classes(&mut tu);
        self.parse_functions(&mut tu);
        self.parse_containers(&mut tu);
        self.parse_virtual_calls(&mut tu);
        tu
    }

    pub fn parse_classes(&self, tu: &mut TranslationUnit) {
        let mut idx = 0;
        while idx < self.clean_lines.len() {
            let (line_num, line) = &self.clean_lines[idx];
            if let Some(class_name) = Self::match_class_decl(line) {
                let (class, consumed) = self.parse_class_body(idx, &class_name, *line_num);
                let vtable: Vec<String> = class
                    .methods
                    .iter()
                    .filter(|m| m.is_virtual)
                    .map(|m| m.name.clone())
                    .collect();
                tu.vtables.insert(class_name.clone(), vtable);
                tu.classes.push(class);
                idx += consumed;
            } else {
                idx += 1;
            }
        }
    }

    fn parse_class_body(
        &self,
        start_idx: usize,
        name: &str,
        line_num: usize,
    ) -> (ClassDecl, usize) {
        let mut methods = Vec::new();
        let mut members = Vec::new();
        let mut base_classes = Vec::new();
        let mut i = start_idx;

        // Check for inheritance on initial line
        let first_line = &self.clean_lines[i].1;
        if let Some(bases) = first_line.split(':').nth(1) {
            for b in bases.split(',') {
                let b = b
                    .trim()
                    .trim_start_matches("public")
                    .trim_start_matches("protected")
                    .trim_start_matches("private")
                    .trim_start_matches("virtual")
                    .trim();
                let b_clean: String = b
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == ':')
                    .collect();
                if !b_clean.is_empty() {
                    base_classes.push(b_clean);
                }
            }
        }

        // Advance to body
        i += 1;
        let mut brace_depth = 0;
        let mut found_open = false;

        while i < self.clean_lines.len() {
            let (cur_line_no, line) = &self.clean_lines[i];
            let trimmed = line.trim();

            for ch in trimmed.chars() {
                if ch == '{' {
                    brace_depth += 1;
                    found_open = true;
                } else if ch == '}' {
                    brace_depth -= 1;
                }
            }

            if found_open && brace_depth == 0 {
                // Class body ended
                break;
            }

            // Inside class body
            if trimmed.is_empty()
                || trimmed.starts_with("public:")
                || trimmed.starts_with("private:")
                || trimmed.starts_with("protected:")
                || trimmed.starts_with("//")
            {
                i += 1;
                continue;
            }

            // Pure virtual: `virtual ... = 0;`
            if trimmed.contains("= 0") && trimmed.ends_with(';') {
                let is_virtual = trimmed.contains("virtual");
                let method_name = Self::extract_method_name(trimmed);
                methods.push(MethodDecl {
                    name: method_name,
                    params: Vec::new(),
                    return_type: String::new(),
                    is_virtual,
                    is_pure_virtual: true,
                    is_const: trimmed.contains("const"),
                    body_lines: Vec::new(),
                    line: *cur_line_no,
                });
                i += 1;
                continue;
            }

            // Virtual method with body or declaration
            if trimmed.starts_with("virtual") || trimmed.contains("virtual ") {
                let method_name = Self::extract_method_name(trimmed);
                let is_pure = trimmed.contains("= 0");
                methods.push(MethodDecl {
                    name: method_name,
                    params: Vec::new(),
                    return_type: String::new(),
                    is_virtual: true,
                    is_pure_virtual: is_pure,
                    is_const: trimmed.contains("const"),
                    body_lines: Vec::new(),
                    line: *cur_line_no,
                });
                i += 1;
                continue;
            }

            // Member variable or member container
            if trimmed.ends_with(';') && !trimmed.contains('(') {
                let without_semi = trimmed.trim_end_matches(';').trim();
                let parts: Vec<&str> = without_semi.split_whitespace().collect();
                if parts.len() >= 2 {
                    let member_name = parts[parts.len() - 1]
                        .trim_start_matches('*')
                        .trim_start_matches('&');
                    let member_type = parts[..parts.len() - 1].join(" ");
                    members.push(MemberDecl {
                        name: member_name.to_string(),
                        type_name: member_type,
                        initializer: None,
                        line: *cur_line_no,
                    });
                }
            }

            i += 1;
        }

        let consumed = if i >= start_idx { i - start_idx + 1 } else { 1 };
        (
            ClassDecl {
                name: name.to_string(),
                base_classes,
                methods,
                members,
                line: line_num,
            },
            consumed,
        )
    }

    pub fn parse_functions(&self, tu: &mut TranslationUnit) {
        let mut i = 0;
        let mut in_class = false;
        let mut class_brace_depth = 0;
        let mut current_class: Option<String> = None;

        while i < self.clean_lines.len() {
            let (line_num, line) = &self.clean_lines[i];
            let trimmed = line.trim();

            // Track class boundaries
            if let Some(cname) = Self::match_class_decl(trimmed) {
                in_class = true;
                current_class = Some(cname);
                class_brace_depth = 0;
            }

            if in_class {
                for ch in trimmed.chars() {
                    if ch == '{' {
                        class_brace_depth += 1;
                    } else if ch == '}' {
                        class_brace_depth -= 1;
                        if class_brace_depth <= 0 {
                            in_class = false;
                            current_class = None;
                        }
                    }
                }
            }

            // Look for function start: contains '(' and ')'
            if Self::is_function_header_candidate(trimmed) {
                // Check if this signature leads to a body '{'
                if let Some((name, class_name, is_ctor, is_dtor, body_start_idx, body_end_idx)) =
                    self.extract_function_definition(i, current_class.as_deref())
                {
                    let mut body_lines = Vec::new();
                    for k in body_start_idx..=body_end_idx {
                        if k < self.clean_lines.len() {
                            body_lines.push(self.clean_lines[k].clone());
                        }
                    }

                    tu.functions.push(FunctionDecl {
                        name,
                        class_name,
                        return_type: String::new(),
                        params: Vec::new(),
                        line: *line_num,
                        body_lines,
                        is_constructor: is_ctor,
                        is_destructor: is_dtor,
                    });

                    i = body_end_idx + 1;
                    continue;
                }
            }

            i += 1;
        }
    }

    fn is_function_header_candidate(line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with("//")
            || trimmed.starts_with("namespace")
            || trimmed.starts_with("typedef")
            || trimmed.starts_with("using")
        {
            return false;
        }

        // Must contain '(' and not start with control keywords
        if !trimmed.contains('(') {
            return false;
        }

        let first_word = trimmed
            .split(|c: char| c.is_whitespace() || c == '(')
            .next()
            .unwrap_or("");
        if matches!(
            first_word,
            "if" | "while" | "for" | "switch" | "catch" | "return" | "else"
        ) {
            return false;
        }

        true
    }

    fn extract_function_definition(
        &self,
        start_idx: usize,
        enclosing_class: Option<&str>,
    ) -> Option<(String, Option<String>, bool, bool, usize, usize)> {
        // Collect signature until '{' or ';'
        let mut sig = String::new();
        let mut idx = start_idx;
        let mut open_brace_idx = None;

        while idx < self.clean_lines.len() && idx < start_idx + 15 {
            let line = self.clean_lines[idx].1.trim();
            sig.push(' ');
            sig.push_str(line);

            if line.contains('{') {
                open_brace_idx = Some(idx);
                break;
            }
            if line.ends_with(';') {
                // Just a prototype/declaration
                return None;
            }
            idx += 1;
        }

        let body_start = open_brace_idx?;

        // Extract function name from signature
        let paren_pos = sig.find('(')?;
        let before_paren = sig[..paren_pos].trim();

        // The function name is the last identifier before '('
        let full_name: String = before_paren
            .split_whitespace()
            .next_back()?
            .trim_start_matches('*')
            .trim_start_matches('&')
            .to_string();

        if full_name.is_empty() {
            return None;
        }

        let mut class_name = enclosing_class.map(|s| s.to_string());
        let short_name;
        let mut is_ctor = false;
        let mut is_dtor = false;

        if let Some(colons) = full_name.rfind("::") {
            let cls = &full_name[..colons];
            let mth = &full_name[colons + 2..];
            class_name = Some(cls.to_string());
            short_name = mth.to_string();
            if short_name == cls {
                is_ctor = true;
            } else if short_name.starts_with('~') {
                is_dtor = true;
            }
        } else {
            short_name = full_name.clone();
            if let Some(cls) = enclosing_class {
                if short_name == cls {
                    is_ctor = true;
                } else if short_name == format!("~{}", cls) {
                    is_dtor = true;
                }
            }
        }

        // Find matching '}' for function body
        let mut depth = 0;
        let mut body_end = body_start;

        for k in body_start..self.clean_lines.len() {
            let line = &self.clean_lines[k].1;
            for ch in line.chars() {
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    depth -= 1;
                    if depth == 0 {
                        body_end = k;
                        return Some((
                            full_name,
                            class_name,
                            is_ctor,
                            is_dtor,
                            body_start,
                            body_end,
                        ));
                    }
                }
            }
        }

        if depth > 0 {
            // Unclosed body, take until end of clean lines
            Some((
                full_name,
                class_name,
                is_ctor,
                is_dtor,
                body_start,
                self.clean_lines.len() - 1,
            ))
        } else {
            Some((full_name, class_name, is_ctor, is_dtor, body_start, body_end))
        }
    }

    pub fn parse_containers(&self, tu: &mut TranslationUnit) {
        let mut idx = 0;
        while idx < self.clean_lines.len() {
            let (_line_num, line) = &self.clean_lines[idx];
            if let Some((var_name, kind, elem_type, init_sz)) = Self::match_container_decl(line) {
                let mut ops = Vec::new();
                let mut j = idx + 1;
                while j < self.clean_lines.len() {
                    let (cur_line_no, l) = &self.clean_lines[j];
                    let trimmed = l.trim();
                    if trimmed.is_empty() {
                        j += 1;
                        continue;
                    }
                    if let Some(op) = Self::match_container_op(trimmed, &var_name) {
                        let mut op_with_line = op;
                        op_with_line.line = *cur_line_no;
                        ops.push(op_with_line);
                    }
                    j += 1;
                }
                tu.containers.push(ContainerUse {
                    var_name,
                    container_kind: kind,
                    element_type: elem_type,
                    operations: ops,
                    initial_size: init_sz,
                });
            }
            idx += 1;
        }
    }

    pub fn parse_virtual_calls(&self, tu: &mut TranslationUnit) {
        let abstract_classes: Vec<&str> = tu
            .classes
            .iter()
            .filter(|c| !c.pure_virtual_methods().is_empty())
            .map(|c| c.name.as_str())
            .collect();

        for (line_num, line) in &self.clean_lines {
            let trimmed = line.trim();
            if let Some((receiver, method)) = Self::extract_call(trimmed) {
                let is_abstract = abstract_classes.contains(&receiver.as_str());
                tu.virtual_calls.push(VirtualCall {
                    receiver,
                    method_name: method,
                    line: *line_num,
                    receiver_is_abstract: is_abstract,
                });
            }
        }
    }

    // --- Pattern matching helpers ---

    pub fn match_class_decl(line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.starts_with("class ") || trimmed.starts_with("struct ") {
            let rest = trimmed
                .trim_start_matches("class")
                .trim_start_matches("struct")
                .trim();
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() && name != "final" && !trimmed.ends_with(';') {
                Some(name)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn match_container_decl(line: &str) -> Option<(String, ContainerKind, String, Option<usize>)> {
        let trimmed = line.trim();
        // std::vector<T> name(size); or vector<T> name; or std::string name;
        let without_std = trimmed.trim_start_matches("std::");
        if without_std.starts_with("vector<") {
            let rest = without_std.strip_prefix("vector<")?;
            let end = rest.find('>')?;
            let elem = rest[..end].trim();
            let after = rest[end + 1..].trim();

            let var_name: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();

            if !var_name.is_empty() {
                // Check if initialized with size: name(10) or name = {1, 2, 3}
                let remainder = after[var_name.len()..].trim();
                let mut initial_size = None;
                if remainder.starts_with('(') {
                    if let Some(close) = remainder.find(')') {
                        let inner = remainder[1..close].trim();
                        if let Ok(sz) = inner.parse::<usize>() {
                            initial_size = Some(sz);
                        }
                    }
                } else if remainder.starts_with('{') {
                    if let Some(close) = remainder.find('}') {
                        let inner = remainder[1..close].trim();
                        let count = inner.split(',').filter(|s| !s.trim().is_empty()).count();
                        initial_size = Some(count);
                    }
                } else if remainder.starts_with('=') && remainder.contains('{') {
                    if let Some(open) = remainder.find('{') {
                        if let Some(close) = remainder.find('}') {
                            let inner = remainder[open + 1..close].trim();
                            let count = inner.split(',').filter(|s| !s.trim().is_empty()).count();
                            initial_size = Some(count);
                        }
                    }
                }

                return Some((var_name, ContainerKind::Vector, elem.to_string(), initial_size));
            }
        }

        if without_std.starts_with("string ") || without_std.starts_with("string\t") {
            let after = without_std.strip_prefix("string")?.trim();
            let var_name: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !var_name.is_empty() {
                return Some((var_name, ContainerKind::String, "char".to_string(), None));
            }
        }

        None
    }

    pub fn match_container_op(line: &str, var: &str) -> Option<ContainerOp> {
        let prefix = format!("{}.", var);
        let arrow_prefix = format!("{}->", var);
        if let Some(pos) = line.find(&prefix).or_else(|| line.find(&arrow_prefix)) {
            let sep_len = if line[pos..].starts_with(&prefix) {
                prefix.len()
            } else {
                arrow_prefix.len()
            };
            let method_part = &line[pos + sep_len..];
            let method_name: String = method_part
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();

            let op_kind = match method_name.as_str() {
                "push_back" => OpKind::PushBack,
                "pop_back" => OpKind::PopBack,
                "insert" => OpKind::Insert,
                "erase" => OpKind::Erase,
                "resize" => OpKind::Resize,
                "clear" => OpKind::Clear,
                "emplace_back" => OpKind::EmplaceBack,
                "assign" => OpKind::Assign,
                "begin" | "end" | "rbegin" | "rend" | "cbegin" | "cend" => OpKind::Iterate,
                "at" | "operator[]" => OpKind::Index,
                _ => return None,
            };

            Some(ContainerOp {
                op_kind,
                line: 0,
                arg_exprs: Vec::new(),
            })
        } else {
            None
        }
    }

    pub fn extract_call(line: &str) -> Option<(String, String)> {
        for (i, ch) in line.char_indices() {
            if ch == '.' || (ch == '-' && i + 1 < line.len() && line.as_bytes().get(i + 1) == Some(&b'>')) {
                let sep_len = if ch == '.' { 1 } else { 2 };
                let before = &line[..i];
                let after = &line[i + sep_len..];
                if let Some(ms) = after.find('(') {
                    let method: String = after[..ms]
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    let receiver: String = before
                        .split_whitespace()
                        .next_back()
                        .unwrap_or("")
                        .chars()
                        .rev()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();
                    if !method.is_empty() && !receiver.is_empty() {
                        return Some((receiver, method));
                    }
                }
            }
        }
        None
    }

    pub fn extract_method_name(line: &str) -> String {
        if let Some(paren_pos) = line.find('(') {
            let before = &line[..paren_pos];
            let trimmed = before.trim();
            trimmed
                .split_whitespace()
                .next_back()
                .unwrap_or("")
                .trim_start_matches('*')
                .trim_start_matches('&')
                .to_string()
        } else {
            String::new()
        }
    }

    pub fn is_control_keyword(line: &str) -> bool {
        matches!(
            line.trim_start(),
            "if" | "for" | "while" | "return" | "switch" | "else" | "do"
        )
    }
}
