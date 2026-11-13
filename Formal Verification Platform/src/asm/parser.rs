//! Parser: converts assembly text into AST nodes and functions.
//!
//! Supports x86_64 (AT&T syntax and Intel syntax) and AArch64 syntax,
//! robustly handling GNU as / glibc assembly constructs:
//! - Block and line comments (/* ... */, //, #)
//! - Preprocessor directives (#include, #define, #ifdef, #endif)
//! - Register aliases (#define src x1)
//! - Assembler directives (.globl, .section, .cfi_*, .type, .align, etc.)
//! - Function demarcation (ENTRY(name), .type name, @function, labels, END(name))
//! - Complex operands (registers, immediates, memory [reg + disp], disp(base, index, scale))

use std::collections::HashMap;
use crate::asm::ast::{Arch, Function, Instruction, Operand, Reg};

/// Errors that can occur during parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnknownMnemonic(String),
    BadOperand(String),
    BadRegister(String),
    BadImmediate(String),
    EmptyLine,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnknownMnemonic(m) => write!(f, "unknown mnemonic: {}", m),
            ParseError::BadOperand(o) => write!(f, "bad operand: {}", o),
            ParseError::BadRegister(r) => write!(f, "bad register: {}", r),
            ParseError::BadImmediate(i) => write!(f, "bad immediate: {}", i),
            ParseError::EmptyLine => write!(f, "empty line"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parser for assembly source text.
pub struct Parser {
    pub arch: Arch,
    aliases: HashMap<String, String>,
    next_offset: u64,
}

impl Parser {
    pub fn new(arch: Arch) -> Self {
        Self {
            arch,
            aliases: HashMap::new(),
            next_offset: 0,
        }
    }

    /// Detect architecture from file path and content.
    pub fn detect_arch(path_str: &str, content: &str) -> Arch {
        let path_lower = path_str.to_ascii_lowercase();
        if path_lower.contains("aarch64") || path_lower.contains("arm64") {
            return Arch::AArch64;
        }
        if path_lower.contains("x86_64") || path_lower.contains("amd64") || path_lower.contains("x86") || path_lower.contains("i386") {
            return Arch::X86_64;
        }

        // Heuristic from content
        let mut aarch64_hints = 0;
        let mut x86_hints = 0;
        for line in content.lines().take(200) {
            let trimmed = line.trim();
            if trimmed.contains("%rax") || trimmed.contains("%rsp") || trimmed.contains("%rdi")
                || trimmed.contains("movq") || trimmed.contains("pushq") || trimmed.contains("popq") {
                x86_hints += 3;
            }
            if trimmed.contains("x0") || trimmed.contains("x1") || trimmed.contains("x29")
                || trimmed.contains("x30") || trimmed.contains("stp") || trimmed.contains("ldp")
                || trimmed.contains("w0") || trimmed.contains("cbnz") || trimmed.contains("cbz") {
                aarch64_hints += 3;
            }
        }

        if aarch64_hints > x86_hints {
            Arch::AArch64
        } else {
            Arch::X86_64
        }
    }

    /// Parse a complete assembly source file into a list of Functions.
    pub fn parse_file(arch: Arch, content: &str) -> (Vec<Function>, usize) {
        let mut parser = Self::new(arch);
        parser.parse_source(content)
    }

    /// Parse an assembly string into functions and return (functions, unparsed_lines_count).
    pub fn parse_source(&mut self, content: &str) -> (Vec<Function>, usize) {
        let cleaned_lines = Self::strip_comments_and_preprocess(content, &mut self.aliases);

        let mut functions: Vec<Function> = Vec::new();
        let mut current_func_name: Option<String> = None;
        let mut current_instructions: Vec<Instruction> = Vec::new();
        let mut current_start_line = 1;
        let mut unparsed_lines = 0;

        for (line_no, line_text) in cleaned_lines {
            let line = line_text.trim();
            if line.is_empty() {
                continue;
            }

            // Check for ENTRY(name) or ENTRY_PKEY(name)
            if let Some(name) = Self::extract_macro_call(line, "ENTRY")
                .or_else(|| Self::extract_macro_call(line, "ENTRY_PKEY"))
            {
                // Finalize previous function if non-empty
                if !current_instructions.is_empty() {
                    let name = current_func_name.take().unwrap_or_else(|| "<toplevel>".into());
                    functions.push(Function {
                        name,
                        instructions: std::mem::take(&mut current_instructions),
                        start_line: current_start_line,
                    });
                }
                current_func_name = Some(name);
                current_start_line = line_no;
                continue;
            }

            // Check for END(name)
            if Self::extract_macro_call(line, "END").is_some() {
                if !current_instructions.is_empty() {
                    let name = current_func_name.take().unwrap_or_else(|| "<toplevel>".into());
                    functions.push(Function {
                        name,
                        instructions: std::mem::take(&mut current_instructions),
                        start_line: current_start_line,
                    });
                }
                continue;
            }

            // Check for assembler directive: .type name, @function
            if line.starts_with(".type") {
                if let Some((name, _)) = line.strip_prefix(".type").and_then(|s| s.split_once(',')) {
                    let name = name.trim().trim_start_matches('@').trim();
                    if !name.is_empty() && current_func_name.is_none() {
                        current_func_name = Some(name.to_string());
                        current_start_line = line_no;
                    }
                }
                continue;
            }

            // Check for other directives starting with '.'
            if line.starts_with('.') {
                // e.g. .globl, .section, .align, .cfi_*, .size, .byte, etc.
                continue;
            }

            // Check for label: `label_name:`
            let mut remaining_code = line;
            if let Some((label_candidate, rest)) = line.split_once(':') {
                let lbl = label_candidate.trim();
                // Check if label is a valid identifier
                if !lbl.is_empty() && !lbl.contains(' ') && !lbl.contains('"') && !lbl.contains('\'') {
                    // Check if it's a global / function label vs a local branch label
                    let is_local = lbl.starts_with(".L") || lbl.starts_with("L(") || lbl.chars().all(|c| c.is_ascii_digit());
                    if !is_local && !lbl.starts_with('.') {
                        // Start a new function if we didn't already have one, or if previous has instructions
                        if !current_instructions.is_empty() {
                            let name = current_func_name.take().unwrap_or_else(|| "<toplevel>".into());
                            functions.push(Function {
                                name,
                                instructions: std::mem::take(&mut current_instructions),
                                start_line: current_start_line,
                            });
                        }
                        current_func_name = Some(lbl.to_string());
                        current_start_line = line_no;
                    }
                    remaining_code = rest.trim();
                }
            }

            if remaining_code.is_empty() {
                continue;
            }

            // Parse one or more statements on this line (separated by ';')
            for stmt in Self::split_statements(remaining_code) {
                let s = stmt.trim();
                if s.is_empty() || s.starts_with('.') {
                    continue;
                }

                match self.parse_instruction(s, line_no) {
                    Ok(instr) => {
                        current_instructions.push(instr);
                    }
                    Err(_) => {
                        unparsed_lines += 1;
                    }
                }
            }
        }

        // Finalize last function
        if !current_instructions.is_empty() {
            let name = current_func_name.unwrap_or_else(|| "<toplevel>".into());
            functions.push(Function {
                name,
                instructions: current_instructions,
                start_line: current_start_line,
            });
        }

        (functions, unparsed_lines)
    }

    /// Split statements separated by ';', respecting quotes and brackets.
    fn split_statements(s: &str) -> Vec<&str> {
        let mut stmts = Vec::new();
        let mut start = 0;
        let mut in_quote = false;
        let mut bracket_depth = 0;
        let mut paren_depth = 0;

        for (i, c) in s.char_indices() {
            match c {
                '"' => in_quote = !in_quote,
                '[' if !in_quote => bracket_depth += 1,
                ']' if !in_quote && bracket_depth > 0 => bracket_depth -= 1,
                '(' if !in_quote => paren_depth += 1,
                ')' if !in_quote && paren_depth > 0 => paren_depth -= 1,
                ';' if !in_quote && bracket_depth == 0 && paren_depth == 0 => {
                    stmts.push(&s[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
        if start < s.len() {
            stmts.push(&s[start..]);
        }
        stmts
    }

    /// Extract macro call parameter, e.g. `ENTRY (foo)` -> `foo`.
    fn extract_macro_call(line: &str, macro_name: &str) -> Option<String> {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(macro_name) {
            let after = rest.trim();
            if let Some(inner) = after.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
                let name = inner.trim().trim_matches('"');
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
        None
    }

    /// Preprocess content: strips /* ... */ comments across lines, handles // and # comments,
    /// and extracts simple #define register aliases.
    fn strip_comments_and_preprocess(content: &str, aliases: &mut HashMap<String, String>) -> Vec<(usize, String)> {
        let mut result = Vec::new();
        let mut in_block_comment = false;
        let mut in_continuation = false;

        for (idx, raw_line) in content.lines().enumerate() {
            let line_no = idx + 1;
            let mut line = raw_line.to_string();

            // Handle block comments across lines
            if in_block_comment {
                if let Some(end_idx) = line.find("*/") {
                    line = line[end_idx + 2..].to_string();
                    in_block_comment = false;
                } else {
                    continue;
                }
            }

            while let Some(start_idx) = line.find("/*") {
                if let Some(end_idx) = line[start_idx + 2..].find("*/") {
                    let actual_end = start_idx + 2 + end_idx + 2;
                    line.replace_range(start_idx..actual_end, " ");
                } else {
                    line.truncate(start_idx);
                    in_block_comment = true;
                    break;
                }
            }

            // Strip // comments
            if let Some(idx) = line.find("//") {
                line.truncate(idx);
            }

            let trimmed = line.trim();

            // Handle multi-line macro continuation
            if in_continuation {
                if !trimmed.ends_with('\\') {
                    in_continuation = false;
                }
                continue;
            }

            // Check for preprocessor line starting with '#'
            if trimmed.starts_with('#') {
                if trimmed.ends_with('\\') {
                    in_continuation = true;
                }

                // Check for `#define NAME VALUE` register alias
                let after_hash = trimmed.strip_prefix('#').unwrap().trim();
                if let Some(rest) = after_hash.strip_prefix("define") {
                    let parts: Vec<&str> = rest.split_whitespace().collect();
                    if parts.len() == 2 {
                        let name = parts[0].trim();
                        let val = parts[1].trim();
                        // Only record if val looks like a register (e.g. x0, %rax, rdi)
                        if val.starts_with('%') || val.starts_with('x') || val.starts_with('w') || val.starts_with('r') {
                            aliases.insert(name.to_string(), val.to_string());
                        }
                    }
                }
                continue;
            }

            // In x86 assembly, '#' can also be an inline comment delimiter if preceded by space
            // In AArch64, '#' introduces immediates (e.g. #16), so only strip '#' if not followed by a number/sign
            if let Some(hash_pos) = line.find('#') {
                let rest_after_hash = &line[hash_pos + 1..].trim_start();
                let is_imm = !rest_after_hash.is_empty()
                    && (rest_after_hash.starts_with(|c: char| c.is_ascii_digit())
                        || rest_after_hash.starts_with('-')
                        || rest_after_hash.starts_with('+'));
                if !is_imm {
                    line.truncate(hash_pos);
                }
            }

            let trimmed_final = line.trim().to_string();
            result.push((line_no, trimmed_final));
        }

        result
    }

    /// Parse a single instruction string into an AST node.
    pub fn parse_instruction(&mut self, text: &str, line_no: usize) -> Result<Instruction, ParseError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(ParseError::EmptyLine);
        }

        // Split mnemonic from operands at first whitespace
        let (mnemonic, rest) = match trimmed.split_once(char::is_whitespace) {
            Some((m, r)) => (m.to_ascii_lowercase(), r.trim()),
            None => (trimmed.to_ascii_lowercase(), ""),
        };

        let operands = if rest.is_empty() {
            Vec::new()
        } else {
            self.parse_operands_str(rest)?
        };

        let instr = Instruction {
            arch: self.arch,
            mnemonic,
            operands,
            offset: self.next_offset,
            line: line_no,
            raw: trimmed.to_string(),
        };
        self.next_offset += 4;
        Ok(instr)
    }

    /// Parse operands string separated by commas, respecting brackets and parentheses.
    fn parse_operands_str(&self, rest: &str) -> Result<Vec<Operand>, ParseError> {
        let mut ops = Vec::new();
        let mut start = 0;
        let mut bracket_depth = 0;
        let mut paren_depth = 0;
        let mut brace_depth = 0;

        for (i, c) in rest.char_indices() {
            match c {
                '[' => bracket_depth += 1,
                ']' if bracket_depth > 0 => bracket_depth -= 1,
                '(' => paren_depth += 1,
                ')' if paren_depth > 0 => paren_depth -= 1,
                '{' => brace_depth += 1,
                '}' if brace_depth > 0 => brace_depth -= 1,
                ',' if bracket_depth == 0 && paren_depth == 0 && brace_depth == 0 => {
                    let part = rest[start..i].trim();
                    if !part.is_empty() {
                        ops.push(self.parse_single_operand(part)?);
                    }
                    start = i + 1;
                }
                _ => {}
            }
        }
        if start < rest.len() {
            let part = rest[start..].trim();
            if !part.is_empty() {
                ops.push(self.parse_single_operand(part)?);
            }
        }
        Ok(ops)
    }

    /// Parse a single operand token.
    pub fn parse_single_operand(&self, raw: &str) -> Result<Operand, ParseError> {
        let mut s = raw.trim();

        // Strip AArch64 curly braces e.g. {vdata.16b}
        if let Some(inner) = s.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
            s = inner.trim();
        }

        // Apply preprocessor aliases
        let resolved: String;
        let s = if let Some(alias) = self.aliases.get(s) {
            resolved = alias.clone();
            resolved.as_str()
        } else {
            s
        };

        // Strip size prefixes in Intel syntax: "qword ptr", "dword ptr", "byte ptr", etc.
        let mut s_mem = s;
        for prefix in &["qword ptr", "dword ptr", "word ptr", "byte ptr", "xmmword ptr", "ymmword ptr", "ptr"] {
            if let Some(rest) = s_mem.strip_prefix(prefix) {
                s_mem = rest.trim();
                break;
            }
        }

        // 1. Bracketed memory operand: `[reg + disp]` or `[reg, #disp]`
        if let Some(inner) = s_mem.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            return self.parse_bracketed_mem(inner);
        }

        // Also handle AArch64 pre-indexed: `[sp, #-16]!`
        if s_mem.ends_with('!') && s_mem.contains('[') && s_mem.contains(']') {
            let trimmed_bang = s_mem.trim_end_matches('!');
            if let Some(inner) = trimmed_bang.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                return self.parse_bracketed_mem(inner);
            }
        }

        // 2. Parenthesized memory operand: AT&T syntax `disp(base, index, scale)` or `(base)`
        if s.contains('(') && s.ends_with(')') {
            return self.parse_att_mem(s);
        }

        // 3. Immediates
        // AT&T immediate: starts with '$'
        if let Some(imm_str) = s.strip_prefix('$') {
            return Self::parse_imm(imm_str).map(Operand::Imm);
        }
        // AArch64 immediate: starts with '#'
        if let Some(imm_str) = s.strip_prefix('#') {
            return Self::parse_imm(imm_str).map(Operand::Imm);
        }
        // Bare integer / hex immediate
        if let Ok(val) = Self::try_parse_int(s) {
            return Ok(Operand::Imm(val));
        }

        // 4. Register
        if let Some(reg) = Reg::parse(self.arch, s) {
            return Ok(Operand::Reg(reg));
        }

        // 5. Label / Symbol fallback
        Ok(Operand::Label(s.to_string()))
    }

    /// Parse AT&T memory operand: `disp(base, index, scale)` or `(base)`.
    fn parse_att_mem(&self, s: &str) -> Result<Operand, ParseError> {
        let (disp_str, rest) = s.split_once('(').ok_or_else(|| ParseError::BadOperand(s.to_string()))?;
        let inner = rest.strip_suffix(')').ok_or_else(|| ParseError::BadOperand(s.to_string()))?;

        let disp = if disp_str.is_empty() {
            0
        } else {
            Self::try_parse_int(disp_str).unwrap_or(0)
        };

        let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
        let base_reg = match parts.first() {
            Some(&b) if !b.is_empty() => {
                Reg::parse(self.arch, b).ok_or_else(|| ParseError::BadRegister(b.to_string()))?
            }
            _ => Reg::RSP,
        };

        let index_reg = parts.get(1).and_then(|&idx| {
            if idx.is_empty() { None } else { Reg::parse(self.arch, idx) }
        });

        let scale = parts.get(2).and_then(|&sc| {
            if sc.is_empty() { None } else { Self::try_parse_int(sc).ok() }
        }).unwrap_or(1);

        Ok(Operand::Mem {
            base: base_reg,
            disp,
            index: index_reg,
            scale,
        })
    }

    /// Parse bracketed memory operand: `[reg + disp]` or `[reg, #disp]`.
    fn parse_bracketed_mem(&self, inner: &str) -> Result<Operand, ParseError> {
        let inner = inner.trim();

        // AArch64 comma-separated format: `sp, #16` or `x29, #8`
        if inner.contains(',') {
            let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
            let base_str = parts[0];
            let base = Reg::parse(self.arch, base_str)
                .ok_or_else(|| ParseError::BadRegister(base_str.to_string()))?;

            let disp = if parts.len() > 1 {
                let d_str = parts[1].trim_start_matches('#');
                Self::try_parse_int(d_str).unwrap_or(0)
            } else {
                0
            };

            return Ok(Operand::Mem {
                base,
                disp,
                index: None,
                scale: 1,
            });
        }

        // Intel format: `rsp + 16` or `rsp - 8` or `rsp` or `rax + rdx*4 + 16`
        if let Some((base_part, rest)) = inner.split_once('+') {
            let base_str = base_part.trim();
            let base = Reg::parse(self.arch, base_str)
                .ok_or_else(|| ParseError::BadRegister(base_str.to_string()))?;
            let disp = Self::try_parse_int(rest.trim()).unwrap_or(0);
            return Ok(Operand::Mem {
                base,
                disp,
                index: None,
                scale: 1,
            });
        }

        if let Some((base_part, rest)) = inner.split_once('-') {
            let base_str = base_part.trim();
            let base = Reg::parse(self.arch, base_str)
                .ok_or_else(|| ParseError::BadRegister(base_str.to_string()))?;
            let disp = -Self::try_parse_int(rest.trim()).unwrap_or(0);
            return Ok(Operand::Mem {
                base,
                disp,
                index: None,
                scale: 1,
            });
        }

        // Just [reg]
        let base = Reg::parse(self.arch, inner)
            .ok_or_else(|| ParseError::BadRegister(inner.to_string()))?;
        Ok(Operand::Mem {
            base,
            disp: 0,
            index: None,
            scale: 1,
        })
    }

    /// Parse immediate value string.
    pub fn parse_imm(s: &str) -> Result<i64, ParseError> {
        Self::try_parse_int(s).map_err(|_| ParseError::BadImmediate(s.to_string()))
    }

    /// Try parsing an integer literal (decimal, hex `0x...`, negative `-...`).
    pub fn try_parse_int(s: &str) -> Result<i64, ()> {
        let s = s.trim();
        if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            i64::from_str_radix(hex, 16).map_err(|_| ())
        } else if let Some(hex) = s.strip_prefix("-0x").or_else(|| s.strip_prefix("-0X")) {
            i64::from_str_radix(hex, 16).map(|v| -v).map_err(|_| ())
        } else if let Some(neg) = s.strip_prefix('-') {
            i64::from_str_radix(neg.trim(), 10).map(|v| -v).map_err(|_| ())
        } else if let Some(pos) = s.strip_prefix('+') {
            i64::from_str_radix(pos.trim(), 10).map_err(|_| ())
        } else {
            i64::from_str_radix(s, 10).map_err(|_| ())
        }
    }

    /// Parse a block of lines using a fresh Parser.
    pub fn parse_block(arch: Arch, text: &str) -> Result<Vec<Instruction>, ParseError> {
        let mut p = Self::new(arch);
        let mut instrs = Vec::new();
        for (i, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') || trimmed.starts_with("//") {
                continue;
            }
            instrs.push(p.parse_instruction(trimmed, i + 1)?);
        }
        Ok(instrs)
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new(Arch::X86_64)
    }
}
