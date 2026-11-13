#![allow(dead_code, unused_variables)]

//! Fast, resilient recursive descent parser for TypeScript and modern JavaScript.

use crate::typescript::ast::*;
use std::collections::HashSet;

// ─── Tokens ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    // Literals
    Int(i64, String),
    Float(f64, String),
    Str(String),
    TemplateNoSubst(String),
    TemplateHead(String),
    TemplateMiddle(String),
    TemplateTail(String),
    Regex(String, String),
    True,
    False,
    Null,
    Undefined,
    Nan,

    // Identifiers & Contextual
    Ident(String),

    // Standard Keywords
    Function,
    Return,
    If,
    Else,
    While,
    For,
    Do,
    Switch,
    Case,
    Default,
    Break,
    Continue,
    Let,
    Const,
    Var,
    Class,
    Extends,
    New,
    This,
    Super,
    Typeof,
    Instanceof,
    In,
    Of,
    Try,
    Catch,
    Finally,
    Throw,
    Yield,
    Await,
    Async,
    Import,
    From,
    Export,
    Delete,
    Void,
    Debugger,

    // TS Contextual Keywords
    Type,
    Interface,
    Implements,
    Enum,
    As,
    Is,
    Keyof,
    Readonly,
    Private,
    Protected,
    Public,
    Static,
    Abstract,
    Declare,
    Constructor,
    Get,
    Set,
    Satisfies,
    Asserts,

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Comma,
    Colon,
    Dot,
    QuestionDot, // ?.
    Ellipsis,    // ...
    Arrow,       // =>
    Question,    // ?
    QuestionQuestion, // ??
    QuestionQuestionAssign, // ??=

    // Operators
    Assign,      // =
    EqEq,        // ==
    EqEqEq,      // ===
    Bang,        // !
    NotEq,       // !=
    NotEqEq,     // !==
    Plus,        // +
    PlusPlus,    // ++
    PlusAssign,  // +=
    Minus,       // -
    MinusMinus,  // --
    MinusAssign, // -=
    Star,        // *
    StarStar,    // **
    StarAssign,  // *=
    StarStarAssign, // **=
    Slash,       // /
    SlashAssign, // /=
    Percent,     // %
    PercentAssign, // %=
    Amp,         // &
    AmpAmp,      // &&
    AmpAssign,   // &=
    AmpAmpAssign,// &&=
    Pipe,        // |
    PipePipe,    // ||
    PipeAssign,  // |=
    PipePipeAssign, // ||=
    Caret,       // ^
    CaretAssign, // ^=
    Tilde,       // ~
    Lt,          // <
    LtEq,        // <=
    LtLt,        // <<
    LtLtAssign,  // <<=
    Gt,          // >
    GtEq,        // >=
    GtGt,        // >>
    GtGtGt,      // >>>
    GtGtAssign,  // >>=
    GtGtGtAssign,// >>>=

    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Tok,
    pub loc: SourceLoc,
    pub has_newline_before: bool,
}

// ─── Lexer ───────────────────────────────────────────────────────────────────

pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
    has_newline_before: bool,
    template_brace_stack: Vec<usize>,
    current_brace_depth: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            src: source.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
            has_newline_before: false,
            template_brace_stack: Vec::new(),
            current_brace_depth: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.src.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        if let Some(b) = self.peek() {
            self.pos += 1;
            if b == b'\n' {
                self.line += 1;
                self.col = 1;
                self.has_newline_before = true;
            } else {
                self.col += 1;
            }
            Some(b)
        } else {
            None
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r' | 0x0C) => {
                    self.advance();
                }
                Some(b'\n') => {
                    self.advance();
                }
                Some(b'/') => {
                    if self.peek_at(1) == Some(b'/') {
                        // Line comment
                        self.advance();
                        self.advance();
                        while let Some(b) = self.peek() {
                            self.advance();
                            if b == b'\n' {
                                break;
                            }
                        }
                    } else if self.peek_at(1) == Some(b'*') {
                        // Block comment
                        self.advance();
                        self.advance();
                        while let Some(b) = self.peek() {
                            if b == b'*' && self.peek_at(1) == Some(b'/') {
                                self.advance();
                                self.advance();
                                break;
                            }
                            self.advance();
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    fn read_identifier_or_keyword(&mut self) -> Tok {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_alphanumeric() || b == b'_' || b == b'$' {
                self.advance();
            } else {
                break;
            }
        }
        let s = String::from_utf8_lossy(&self.src[start..self.pos]).to_string();
        match s.as_str() {
            "function" => Tok::Function,
            "return" => Tok::Return,
            "if" => Tok::If,
            "else" => Tok::Else,
            "while" => Tok::While,
            "for" => Tok::For,
            "do" => Tok::Do,
            "switch" => Tok::Switch,
            "case" => Tok::Case,
            "default" => Tok::Default,
            "break" => Tok::Break,
            "continue" => Tok::Continue,
            "let" => Tok::Let,
            "const" => Tok::Const,
            "var" => Tok::Var,
            "class" => Tok::Class,
            "extends" => Tok::Extends,
            "new" => Tok::New,
            "this" => Tok::This,
            "super" => Tok::Super,
            "typeof" => Tok::Typeof,
            "instanceof" => Tok::Instanceof,
            "in" => Tok::In,
            "of" => Tok::Of,
            "try" => Tok::Try,
            "catch" => Tok::Catch,
            "finally" => Tok::Finally,
            "throw" => Tok::Throw,
            "yield" => Tok::Yield,
            "await" => Tok::Await,
            "async" => Tok::Async,
            "import" => Tok::Import,
            "from" => Tok::From,
            "export" => Tok::Export,
            "delete" => Tok::Delete,
            "void" => Tok::Void,
            "debugger" => Tok::Debugger,
            "true" => Tok::True,
            "false" => Tok::False,
            "null" => Tok::Null,
            "undefined" => Tok::Undefined,
            "NaN" => Tok::Nan,
            "type" => Tok::Type,
            "interface" => Tok::Interface,
            "implements" => Tok::Implements,
            "enum" => Tok::Enum,
            "as" => Tok::As,
            "is" => Tok::Is,
            "keyof" => Tok::Keyof,
            "readonly" => Tok::Readonly,
            "private" => Tok::Private,
            "protected" => Tok::Protected,
            "public" => Tok::Public,
            "static" => Tok::Static,
            "abstract" => Tok::Abstract,
            "declare" => Tok::Declare,
            "constructor" => Tok::Constructor,
            "get" => Tok::Get,
            "set" => Tok::Set,
            "satisfies" => Tok::Satisfies,
            "asserts" => Tok::Asserts,
            _ => Tok::Ident(s),
        }
    }

    fn read_number(&mut self) -> Tok {
        let start = self.pos;
        let mut is_float = false;

        if self.peek() == Some(b'0') {
            match self.peek_at(1) {
                Some(b'x' | b'X') => {
                    self.advance();
                    self.advance();
                    while let Some(b) = self.peek() {
                        if b.is_ascii_hexdigit() || b == b'_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if self.peek() == Some(b'n') {
                        self.advance();
                    }
                    let raw = String::from_utf8_lossy(&self.src[start..self.pos]).to_string();
                    let clean = raw.replace('_', "").trim_end_matches('n').to_string();
                    let val = i64::from_str_radix(&clean[2..], 16).unwrap_or(0);
                    return Tok::Int(val, raw);
                }
                Some(b'b' | b'B') => {
                    self.advance();
                    self.advance();
                    while let Some(b) = self.peek() {
                        if b == b'0' || b == b'1' || b == b'_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if self.peek() == Some(b'n') {
                        self.advance();
                    }
                    let raw = String::from_utf8_lossy(&self.src[start..self.pos]).to_string();
                    let clean = raw.replace('_', "").trim_end_matches('n').to_string();
                    let val = i64::from_str_radix(&clean[2..], 2).unwrap_or(0);
                    return Tok::Int(val, raw);
                }
                Some(b'o' | b'O') => {
                    self.advance();
                    self.advance();
                    while let Some(b) = self.peek() {
                        if (b'0'..=b'7').contains(&b) || b == b'_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if self.peek() == Some(b'n') {
                        self.advance();
                    }
                    let raw = String::from_utf8_lossy(&self.src[start..self.pos]).to_string();
                    let clean = raw.replace('_', "").trim_end_matches('n').to_string();
                    let val = i64::from_str_radix(&clean[2..], 8).unwrap_or(0);
                    return Tok::Int(val, raw);
                }
                _ => {}
            }
        }

        while let Some(b) = self.peek() {
            if b.is_ascii_digit() || b == b'_' {
                self.advance();
            } else {
                break;
            }
        }

        if self.peek() == Some(b'.') && self.peek_at(1).map_or(false, |b| b.is_ascii_digit()) {
            is_float = true;
            self.advance(); // .
            while let Some(b) = self.peek() {
                if b.is_ascii_digit() || b == b'_' {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        if let Some(b'e' | b'E') = self.peek() {
            is_float = true;
            self.advance();
            if let Some(b'+' | b'-') = self.peek() {
                self.advance();
            }
            while let Some(b) = self.peek() {
                if b.is_ascii_digit() || b == b'_' {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        if self.peek() == Some(b'n') {
            self.advance();
        }

        let raw = String::from_utf8_lossy(&self.src[start..self.pos]).to_string();
        let clean = raw.replace('_', "").trim_end_matches('n').to_string();

        if is_float {
            let val = clean.parse::<f64>().unwrap_or(0.0);
            Tok::Float(val, raw)
        } else {
            let val = clean.parse::<i64>().unwrap_or(0);
            Tok::Int(val, raw)
        }
    }

    fn read_string(&mut self, quote: u8) -> Tok {
        self.advance(); // quote
        let mut out = String::new();
        while let Some(b) = self.peek() {
            if b == quote {
                self.advance();
                break;
            }
            if b == b'\\' {
                self.advance();
                if let Some(esc) = self.advance() {
                    match esc {
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'\\' => out.push('\\'),
                        b'\'' => out.push('\''),
                        b'"' => out.push('"'),
                        b'0' => out.push('\0'),
                        _ => out.push(esc as char),
                    }
                }
            } else {
                out.push(b as char);
                self.advance();
            }
        }
        Tok::Str(out)
    }

    fn read_template_chunk(&mut self, is_start: bool) -> Tok {
        let mut out = String::new();
        while let Some(b) = self.peek() {
            if b == b'`' {
                self.advance();
                return if is_start {
                    Tok::TemplateNoSubst(out)
                } else {
                    Tok::TemplateTail(out)
                };
            }
            if b == b'$' && self.peek_at(1) == Some(b'{') {
                self.advance(); // $
                self.advance(); // {
                self.current_brace_depth += 1;
                self.template_brace_stack.push(self.current_brace_depth);
                return if is_start {
                    Tok::TemplateHead(out)
                } else {
                    Tok::TemplateMiddle(out)
                };
            }
            if b == b'\\' {
                self.advance();
                if let Some(esc) = self.advance() {
                    match esc {
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'\\' => out.push('\\'),
                        b'`' => out.push('`'),
                        b'$' => out.push('$'),
                        _ => out.push(esc as char),
                    }
                }
            } else {
                out.push(b as char);
                self.advance();
            }
        }
        if is_start {
            Tok::TemplateNoSubst(out)
        } else {
            Tok::TemplateTail(out)
        }
    }

    fn read_regex(&mut self) -> Tok {
        self.advance(); // opening '/'
        let mut pattern = String::new();
        let mut in_class = false;

        while let Some(b) = self.peek() {
            if b == b'/' && !in_class {
                self.advance(); // closing '/'
                break;
            }
            if b == b'[' {
                in_class = true;
                pattern.push('[');
                self.advance();
            } else if b == b']' {
                in_class = false;
                pattern.push(']');
                self.advance();
            } else if b == b'\\' {
                pattern.push('\\');
                self.advance();
                if let Some(next) = self.advance() {
                    pattern.push(next as char);
                }
            } else if b == b'\n' {
                break;
            } else {
                pattern.push(b as char);
                self.advance();
            }
        }

        let mut flags = String::new();
        while let Some(b) = self.peek() {
            if b.is_ascii_alphabetic() {
                flags.push(b as char);
                self.advance();
            } else {
                break;
            }
        }

        Tok::Regex(pattern, flags)
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            self.has_newline_before = false;
            self.skip_whitespace_and_comments();
            let newline_before = self.has_newline_before;
            let line = self.line;
            let col = self.col;

            let Some(b) = self.peek() else {
                tokens.push(Token {
                    kind: Tok::Eof,
                    loc: SourceLoc { line, col },
                    has_newline_before: newline_before,
                });
                break;
            };

            // Check if we are closing a template interpolation ${ ... }
            if b == b'}' {
                if let Some(&depth) = self.template_brace_stack.last() {
                    if depth == self.current_brace_depth {
                        self.advance(); // '}'
                        self.template_brace_stack.pop();
                        self.current_brace_depth -= 1;
                        let chunk = self.read_template_chunk(false);
                        tokens.push(Token {
                            kind: chunk,
                            loc: SourceLoc { line, col },
                            has_newline_before: newline_before,
                        });
                        continue;
                    }
                }
            }

            // Determine if '/' is regex or division
            if b == b'/' && self.peek_at(1) != Some(b'/') && self.peek_at(1) != Some(b'*') {
                let is_regex = match tokens.last().map(|t| &t.kind) {
                    None => true,
                    Some(
                        Tok::LParen
                        | Tok::LBracket
                        | Tok::LBrace
                        | Tok::Comma
                        | Tok::Semicolon
                        | Tok::Colon
                        | Tok::Assign
                        | Tok::Arrow
                        | Tok::Question
                        | Tok::Return
                        | Tok::Throw
                        | Tok::Case
                        | Tok::Yield
                        | Tok::Await
                        | Tok::Plus
                        | Tok::Minus
                        | Tok::Star
                        | Tok::Slash
                        | Tok::Percent
                        | Tok::Bang
                        | Tok::AmpAmp
                        | Tok::PipePipe
                        | Tok::QuestionQuestion,
                    ) => true,
                    _ => false,
                };

                if is_regex {
                    let tok = self.read_regex();
                    tokens.push(Token {
                        kind: tok,
                        loc: SourceLoc { line, col },
                        has_newline_before: newline_before,
                    });
                    continue;
                }
            }

            let kind = match b {
                b'(' => {
                    self.advance();
                    Tok::LParen
                }
                b')' => {
                    self.advance();
                    Tok::RParen
                }
                b'{' => {
                    self.advance();
                    self.current_brace_depth += 1;
                    Tok::LBrace
                }
                b'}' => {
                    self.advance();
                    if self.current_brace_depth > 0 {
                        self.current_brace_depth -= 1;
                    }
                    Tok::RBrace
                }
                b'[' => {
                    self.advance();
                    Tok::LBracket
                }
                b']' => {
                    self.advance();
                    Tok::RBracket
                }
                b';' => {
                    self.advance();
                    Tok::Semicolon
                }
                b',' => {
                    self.advance();
                    Tok::Comma
                }
                b':' => {
                    self.advance();
                    Tok::Colon
                }
                b'`' => {
                    self.advance(); // `
                    self.read_template_chunk(true)
                }
                b'\'' | b'"' => self.read_string(b),
                b'?' => {
                    self.advance();
                    if self.peek() == Some(b'.') && !self.peek_at(1).map_or(false, |c| c.is_ascii_digit()) {
                        self.advance();
                        Tok::QuestionDot
                    } else if self.peek() == Some(b'?') {
                        self.advance();
                        if self.peek() == Some(b'=') {
                            self.advance();
                            Tok::QuestionQuestionAssign
                        } else {
                            Tok::QuestionQuestion
                        }
                    } else {
                        Tok::Question
                    }
                }
                b'.' => {
                    if self.peek_at(1) == Some(b'.') && self.peek_at(2) == Some(b'.') {
                        self.advance();
                        self.advance();
                        self.advance();
                        Tok::Ellipsis
                    } else if self.peek_at(1).map_or(false, |c| c.is_ascii_digit()) {
                        self.read_number()
                    } else {
                        self.advance();
                        Tok::Dot
                    }
                }
                b'=' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        if self.peek() == Some(b'=') {
                            self.advance();
                            Tok::EqEqEq
                        } else {
                            Tok::EqEq
                        }
                    } else if self.peek() == Some(b'>') {
                        self.advance();
                        Tok::Arrow
                    } else {
                        Tok::Assign
                    }
                }
                b'!' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        if self.peek() == Some(b'=') {
                            self.advance();
                            Tok::NotEqEq
                        } else {
                            Tok::NotEq
                        }
                    } else {
                        Tok::Bang
                    }
                }
                b'+' => {
                    self.advance();
                    if self.peek() == Some(b'+') {
                        self.advance();
                        Tok::PlusPlus
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::PlusAssign
                    } else {
                        Tok::Plus
                    }
                }
                b'-' => {
                    self.advance();
                    if self.peek() == Some(b'-') {
                        self.advance();
                        Tok::MinusMinus
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::MinusAssign
                    } else {
                        Tok::Minus
                    }
                }
                b'*' => {
                    self.advance();
                    if self.peek() == Some(b'*') {
                        self.advance();
                        if self.peek() == Some(b'=') {
                            self.advance();
                            Tok::StarStarAssign
                        } else {
                            Tok::StarStar
                        }
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::StarAssign
                    } else {
                        Tok::Star
                    }
                }
                b'/' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::SlashAssign
                    } else {
                        Tok::Slash
                    }
                }
                b'%' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::PercentAssign
                    } else {
                        Tok::Percent
                    }
                }
                b'&' => {
                    self.advance();
                    if self.peek() == Some(b'&') {
                        self.advance();
                        if self.peek() == Some(b'=') {
                            self.advance();
                            Tok::AmpAmpAssign
                        } else {
                            Tok::AmpAmp
                        }
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::AmpAssign
                    } else {
                        Tok::Amp
                    }
                }
                b'|' => {
                    self.advance();
                    if self.peek() == Some(b'|') {
                        self.advance();
                        if self.peek() == Some(b'=') {
                            self.advance();
                            Tok::PipePipeAssign
                        } else {
                            Tok::PipePipe
                        }
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::PipeAssign
                    } else {
                        Tok::Pipe
                    }
                }
                b'^' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::CaretAssign
                    } else {
                        Tok::Caret
                    }
                }
                b'~' => {
                    self.advance();
                    Tok::Tilde
                }
                b'<' => {
                    self.advance();
                    if self.peek() == Some(b'<') {
                        self.advance();
                        if self.peek() == Some(b'=') {
                            self.advance();
                            Tok::LtLtAssign
                        } else {
                            Tok::LtLt
                        }
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::LtEq
                    } else {
                        Tok::Lt
                    }
                }
                b'>' => {
                    self.advance();
                    if self.peek() == Some(b'>') {
                        self.advance();
                        if self.peek() == Some(b'>') {
                            self.advance();
                            if self.peek() == Some(b'=') {
                                self.advance();
                                Tok::GtGtGtAssign
                            } else {
                                Tok::GtGtGt
                            }
                        } else if self.peek() == Some(b'=') {
                            self.advance();
                            Tok::GtGtAssign
                        } else {
                            Tok::GtGt
                        }
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::GtEq
                    } else {
                        Tok::Gt
                    }
                }
                _ if b.is_ascii_digit() => self.read_number(),
                _ if b.is_ascii_alphabetic() || b == b'_' || b == b'$' => {
                    self.read_identifier_or_keyword()
                }
                _ => {
                    // Unknown byte, advance past it
                    self.advance();
                    continue;
                }
            };

            tokens.push(Token {
                kind,
                loc: SourceLoc { line, col },
                has_newline_before: newline_before,
            });
        }

        tokens
    }
}

// ─── Parser ──────────────────────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Tok {
        &self.tokens.get(self.pos).map(|t| &t.kind).unwrap_or(&Tok::Eof)
    }

    fn peek_tok(&self) -> &Token {
        static EOF_TOK: Token = Token {
            kind: Tok::Eof,
            loc: SourceLoc { line: 0, col: 0 },
            has_newline_before: false,
        };
        self.tokens.get(self.pos).unwrap_or(&EOF_TOK)
    }

    fn peek_at(&self, offset: usize) -> &Tok {
        &self.tokens.get(self.pos + offset).map(|t| &t.kind).unwrap_or(&Tok::Eof)
    }

    fn loc(&self) -> SourceLoc {
        self.tokens.get(self.pos).map(|t| t.loc).unwrap_or_default()
    }

    fn advance(&mut self) -> Token {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            t
        } else {
            self.tokens.last().cloned().unwrap_or(Token {
                kind: Tok::Eof,
                loc: SourceLoc { line: 0, col: 0 },
                has_newline_before: false,
            })
        }
    }

    fn check(&self, expected: &Tok) -> bool {
        self.peek() == expected
    }

    fn eat(&mut self, expected: Tok) -> bool {
        if self.check(&expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: Tok) -> Result<Token, String> {
        if self.check(&expected) {
            Ok(self.advance())
        } else {
            Err(format!(
                "expected {:?}, found {:?} at {}",
                expected,
                self.peek(),
                self.loc()
            ))
        }
    }

    fn is_ident_or_keyword(&self) -> bool {
        match self.peek() {
            Tok::Ident(_)
            | Tok::Type
            | Tok::Interface
            | Tok::Implements
            | Tok::Enum
            | Tok::As
            | Tok::Is
            | Tok::Keyof
            | Tok::Readonly
            | Tok::Private
            | Tok::Protected
            | Tok::Public
            | Tok::Static
            | Tok::Abstract
            | Tok::Declare
            | Tok::Constructor
            | Tok::Get
            | Tok::Set
            | Tok::Satisfies
            | Tok::Asserts
            | Tok::From
            | Tok::Of => true,
            _ => false,
        }
    }

    fn expect_ident_or_keyword(&mut self) -> Result<String, String> {
        let tok = self.advance();
        match tok.kind {
            Tok::Ident(s) => Ok(s),
            Tok::Type => Ok("type".to_string()),
            Tok::Interface => Ok("interface".to_string()),
            Tok::Implements => Ok("implements".to_string()),
            Tok::Enum => Ok("enum".to_string()),
            Tok::As => Ok("as".to_string()),
            Tok::Is => Ok("is".to_string()),
            Tok::Keyof => Ok("keyof".to_string()),
            Tok::Readonly => Ok("readonly".to_string()),
            Tok::Private => Ok("private".to_string()),
            Tok::Protected => Ok("protected".to_string()),
            Tok::Public => Ok("public".to_string()),
            Tok::Static => Ok("static".to_string()),
            Tok::Abstract => Ok("abstract".to_string()),
            Tok::Declare => Ok("declare".to_string()),
            Tok::Constructor => Ok("constructor".to_string()),
            Tok::Get => Ok("get".to_string()),
            Tok::Set => Ok("set".to_string()),
            Tok::Satisfies => Ok("satisfies".to_string()),
            Tok::Asserts => Ok("asserts".to_string()),
            Tok::From => Ok("from".to_string()),
            Tok::Of => Ok("of".to_string()),
            other => Err(format!("expected identifier, found {:?} at {}", other, tok.loc)),
        }
    }

    fn eat_semicolon(&mut self) {
        if self.eat(Tok::Semicolon) {
            return;
        }
        // ASI: newline or closing brace or EOF
        if self.peek_tok().has_newline_before
            || self.check(&Tok::RBrace)
            || self.check(&Tok::Eof)
        {
            return;
        }
    }

    // ─── Top Level Parsing ───────────────────────────────────────────────────

    pub fn parse_source_file(&mut self, path: &str) -> SourceFile {
        let mut stmts = Vec::new();

        while !self.check(&Tok::Eof) {
            match self.parse_stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(_err) => {
                    // Resilient skip to next statement
                    self.synchronize();
                }
            }
        }

        SourceFile {
            path: path.to_string(),
            statements: stmts,
        }
    }

    fn synchronize(&mut self) {
        self.advance();
        while !self.check(&Tok::Eof) {
            if self.tokens[self.pos - 1].kind == Tok::Semicolon
                || self.peek_tok().has_newline_before
            {
                return;
            }
            match self.peek() {
                Tok::Function
                | Tok::Class
                | Tok::Interface
                | Tok::Type
                | Tok::Enum
                | Tok::Let
                | Tok::Const
                | Tok::Var
                | Tok::If
                | Tok::While
                | Tok::For
                | Tok::Return
                | Tok::Import
                | Tok::Export => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ─── Statements ──────────────────────────────────────────────────────────

    pub fn parse_stmt(&mut self) -> Result<Stmt, String> {
        let loc = self.loc();

        // Check for export
        if self.eat(Tok::Export) {
            return self.parse_export_stmt(loc);
        }

        // Check for import
        if self.eat(Tok::Import) {
            return self.parse_import_stmt(loc);
        }

        match self.peek() {
            Tok::Semicolon => {
                self.advance();
                Ok(Stmt::Empty(loc))
            }
            Tok::LBrace => {
                let block = self.parse_block()?;
                Ok(Stmt::Block(block))
            }
            Tok::Let => {
                self.advance();
                let decls = self.parse_var_decls()?;
                self.eat_semicolon();
                Ok(Stmt::VarDecl {
                    kind: VarKind::Let,
                    decls,
                    loc,
                })
            }
            Tok::Const => {
                self.advance();
                let decls = self.parse_var_decls()?;
                self.eat_semicolon();
                Ok(Stmt::VarDecl {
                    kind: VarKind::Const,
                    decls,
                    loc,
                })
            }
            Tok::Var => {
                self.advance();
                let decls = self.parse_var_decls()?;
                self.eat_semicolon();
                Ok(Stmt::VarDecl {
                    kind: VarKind::Var,
                    decls,
                    loc,
                })
            }
            Tok::Function => self.parse_function_decl(false),
            Tok::Async if self.peek_at(1) == &Tok::Function => {
                self.advance(); // async
                self.parse_function_decl(true)
            }
            Tok::Class => self.parse_class_decl(false),
            Tok::Abstract if self.peek_at(1) == &Tok::Class => {
                self.advance(); // abstract
                self.parse_class_decl(true)
            }
            Tok::Interface => self.parse_interface_decl(),
            Tok::Type if self.is_type_alias_ahead() => self.parse_type_alias_decl(),
            Tok::Enum => self.parse_enum_decl(),
            Tok::If => self.parse_if_stmt(),
            Tok::While => self.parse_while_stmt(),
            Tok::Do => self.parse_do_while_stmt(),
            Tok::For => self.parse_for_stmt(),
            Tok::Switch => self.parse_switch_stmt(),
            Tok::Return => {
                self.advance();
                let expr = if !self.check(&Tok::Semicolon)
                    && !self.check(&Tok::RBrace)
                    && !self.peek_tok().has_newline_before
                {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.eat_semicolon();
                Ok(Stmt::Return(expr, loc))
            }
            Tok::Throw => {
                self.advance();
                let expr = self.parse_expr()?;
                self.eat_semicolon();
                Ok(Stmt::Throw(expr, loc))
            }
            Tok::Try => self.parse_try_stmt(),
            Tok::Break => {
                self.advance();
                let label = if self.is_ident_or_keyword() && !self.peek_tok().has_newline_before {
                    Some(self.expect_ident_or_keyword()?)
                } else {
                    None
                };
                self.eat_semicolon();
                Ok(Stmt::Break(label, loc))
            }
            Tok::Continue => {
                self.advance();
                let label = if self.is_ident_or_keyword() && !self.peek_tok().has_newline_before {
                    Some(self.expect_ident_or_keyword()?)
                } else {
                    None
                };
                self.eat_semicolon();
                Ok(Stmt::Continue(label, loc))
            }
            Tok::Debugger => {
                self.advance();
                self.eat_semicolon();
                Ok(Stmt::Empty(loc))
            }
            _ => {
                let expr = self.parse_expr()?;
                self.eat_semicolon();
                Ok(Stmt::Expr(expr, loc))
            }
        }
    }

    fn is_type_alias_ahead(&self) -> bool {
        self.peek() == &Tok::Type && matches!(self.peek_at(1), Tok::Ident(_))
    }

    pub fn parse_block(&mut self) -> Result<Block, String> {
        let loc = self.loc();
        self.expect(Tok::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
            match self.parse_stmt() {
                Ok(s) => stmts.push(s),
                Err(_) => self.synchronize(),
            }
        }
        self.expect(Tok::RBrace)?;
        Ok(Block { stmts, loc })
    }

    // ─── Variable Declarations ───────────────────────────────────────────────

    fn parse_var_decls(&mut self) -> Result<Vec<VarDeclarator>, String> {
        let mut decls = Vec::new();
        loop {
            let loc = self.loc();
            let pattern = self.parse_binding_pattern()?;
            let ty = if self.eat(Tok::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            let init = if self.eat(Tok::Assign) {
                Some(self.parse_assign_expr()?)
            } else {
                None
            };
            decls.push(VarDeclarator {
                pattern,
                ty,
                init,
                loc,
            });
            if !self.eat(Tok::Comma) {
                break;
            }
        }
        Ok(decls)
    }

    fn parse_binding_pattern(&mut self) -> Result<BindingPattern, String> {
        match self.peek() {
            Tok::LBracket => {
                self.advance();
                let mut elems = Vec::new();
                while !self.check(&Tok::RBracket) && !self.check(&Tok::Eof) {
                    if self.eat(Tok::Comma) {
                        elems.push(None);
                        continue;
                    }
                    if self.eat(Tok::Ellipsis) {
                        let inner = self.parse_binding_pattern()?;
                        elems.push(Some(inner));
                        break;
                    }
                    let inner = self.parse_binding_pattern()?;
                    elems.push(Some(inner));
                    if !self.eat(Tok::Comma) {
                        break;
                    }
                }
                self.expect(Tok::RBracket)?;
                Ok(BindingPattern::Array(elems))
            }
            Tok::LBrace => {
                self.advance();
                let mut props = Vec::new();
                while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
                    if self.eat(Tok::Ellipsis) {
                        let name = self.expect_ident_or_keyword()?;
                        props.push(ObjectBindingProp {
                            key: name.clone(),
                            value: Some(BindingPattern::Ident(name)),
                            default_val: None,
                        });
                        break;
                    }
                    let key = self.expect_ident_or_keyword()?;
                    let value = if self.eat(Tok::Colon) {
                        Some(self.parse_binding_pattern()?)
                    } else {
                        None
                    };
                    let default_val = if self.eat(Tok::Assign) {
                        Some(self.parse_assign_expr()?)
                    } else {
                        None
                    };
                    props.push(ObjectBindingProp {
                        key,
                        value,
                        default_val,
                    });
                    if !self.eat(Tok::Comma) {
                        break;
                    }
                }
                self.expect(Tok::RBrace)?;
                Ok(BindingPattern::Object(props))
            }
            _ => {
                let name = self.expect_ident_or_keyword()?;
                Ok(BindingPattern::Ident(name))
            }
        }
    }

    // ─── Function Declarations ───────────────────────────────────────────────

    fn parse_function_decl(&mut self, is_async: bool) -> Result<Stmt, String> {
        let loc = self.loc();
        self.expect(Tok::Function)?;
        let is_generator = self.eat(Tok::Star);
        let name = if self.is_ident_or_keyword() {
            Some(self.expect_ident_or_keyword()?)
        } else {
            None
        };

        let type_params = self.parse_optional_type_params();
        let params = self.parse_params()?;
        let return_ty = if self.eat(Tok::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let body = if self.check(&Tok::LBrace) {
            Some(self.parse_block()?)
        } else {
            self.eat_semicolon();
            None
        };

        Ok(Stmt::FunctionDecl(Function {
            name,
            params,
            return_ty,
            body,
            is_async,
            is_generator,
            type_params,
            loc,
        }))
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, String> {
        self.expect(Tok::LParen)?;
        let mut params = Vec::new();
        while !self.check(&Tok::RParen) && !self.check(&Tok::Eof) {
            // Modifiers in constructor: public, private, protected, readonly
            while matches!(
                self.peek(),
                Tok::Public | Tok::Private | Tok::Protected | Tok::Readonly
            ) {
                self.advance();
            }

            let is_rest = self.eat(Tok::Ellipsis);
            let pattern = self.parse_binding_pattern()?;
            let is_optional = self.eat(Tok::Question);
            let ty = if self.eat(Tok::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            let default_value = if self.eat(Tok::Assign) {
                Some(self.parse_assign_expr()?)
            } else {
                None
            };

            params.push(Param {
                pattern,
                ty,
                default_value,
                is_optional,
                is_rest,
            });

            if !self.eat(Tok::Comma) {
                break;
            }
        }
        self.expect(Tok::RParen)?;
        Ok(params)
    }

    fn parse_optional_type_params(&mut self) -> Vec<String> {
        let mut params = Vec::new();
        if self.eat(Tok::Lt) {
            let mut depth = 1;
            while depth > 0 && !self.check(&Tok::Eof) {
                if self.check(&Tok::Lt) {
                    depth += 1;
                } else if self.check(&Tok::Gt) {
                    depth -= 1;
                    if depth == 0 {
                        self.advance();
                        break;
                    }
                }
                if depth == 1 && self.is_ident_or_keyword() {
                    if let Ok(name) = self.expect_ident_or_keyword() {
                        params.push(name);
                    }
                } else {
                    self.advance();
                }
            }
        }
        params
    }

    // ─── Class Declarations ──────────────────────────────────────────────────

    fn parse_class_decl(&mut self, is_abstract: bool) -> Result<Stmt, String> {
        let loc = self.loc();
        self.expect(Tok::Class)?;
        let name = if self.is_ident_or_keyword() {
            Some(self.expect_ident_or_keyword()?)
        } else {
            None
        };

        let type_params = self.parse_optional_type_params();

        let extends = if self.eat(Tok::Extends) {
            Some(self.parse_lhs_expr()?)
        } else {
            None
        };

        let implements = if self.eat(Tok::Implements) {
            self.parse_type_list()?
        } else {
            Vec::new()
        };

        self.expect(Tok::LBrace)?;
        let mut members = Vec::new();

        while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
            if self.eat(Tok::Semicolon) {
                continue;
            }

            let member_loc = self.loc();
            let mut is_static = false;
            let mut is_async = false;
            let mut _is_abstract_member = false;

            // Collect member modifiers
            loop {
                match self.peek() {
                    Tok::Public | Tok::Protected | Tok::Private | Tok::Readonly => {
                        self.advance();
                    }
                    Tok::Static => {
                        self.advance();
                        is_static = true;
                    }
                    Tok::Abstract => {
                        self.advance();
                        _is_abstract_member = true;
                    }
                    Tok::Async => {
                        self.advance();
                        is_async = true;
                    }
                    _ => break,
                }
            }

            let is_generator = self.eat(Tok::Star);

            let kind = if self.check(&Tok::Get) && self.peek_at(1) != &Tok::LParen {
                self.advance();
                MethodKind::Getter
            } else if self.check(&Tok::Set) && self.peek_at(1) != &Tok::LParen {
                self.advance();
                MethodKind::Setter
            } else {
                MethodKind::Normal
            };

            let member_name = if self.is_ident_or_keyword() {
                self.expect_ident_or_keyword()?
            } else if self.check(&Tok::LBracket) {
                // Computed property name: skip inside [...]
                self.advance();
                while !self.check(&Tok::RBracket) && !self.check(&Tok::Eof) {
                    self.advance();
                }
                self.eat(Tok::RBracket);
                "__computed__".to_string()
            } else {
                self.advance();
                continue;
            };

            if member_name == "constructor" {
                let params = self.parse_params()?;
                let body = if self.check(&Tok::LBrace) {
                    Some(self.parse_block()?)
                } else {
                    self.eat_semicolon();
                    None
                };
                members.push(ClassMember::Constructor {
                    params,
                    body,
                    loc: member_loc,
                });
                continue;
            }

            // Method vs Field
            let _ = self.parse_optional_type_params();
            if self.check(&Tok::LParen) {
                let params = self.parse_params()?;
                let return_ty = if self.eat(Tok::Colon) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };
                let body = if self.check(&Tok::LBrace) {
                    Some(self.parse_block()?)
                } else {
                    self.eat_semicolon();
                    None
                };
                members.push(ClassMember::Method {
                    name: member_name,
                    is_static,
                    is_async,
                    is_generator,
                    kind,
                    params,
                    return_ty,
                    body,
                    loc: member_loc,
                });
            } else {
                let ty = if self.eat(Tok::Colon) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };
                let init = if self.eat(Tok::Assign) {
                    Some(self.parse_assign_expr()?)
                } else {
                    None
                };
                self.eat_semicolon();
                members.push(ClassMember::Field {
                    name: member_name,
                    is_static,
                    ty,
                    init,
                    loc: member_loc,
                });
            }
        }

        self.expect(Tok::RBrace)?;

        Ok(Stmt::ClassDecl(Class {
            name,
            extends,
            implements,
            members,
            is_abstract,
            type_params,
            loc,
        }))
    }

    // ─── Interfaces & Types & Enums ──────────────────────────────────────────

    fn parse_interface_decl(&mut self) -> Result<Stmt, String> {
        let loc = self.loc();
        self.expect(Tok::Interface)?;
        let name = self.expect_ident_or_keyword()?;
        let type_params = self.parse_optional_type_params();
        let extends = if self.eat(Tok::Extends) {
            self.parse_type_list()?
        } else {
            Vec::new()
        };

        // Skip interface body: { ... }
        if self.eat(Tok::LBrace) {
            let mut depth = 1;
            while depth > 0 && !self.check(&Tok::Eof) {
                if self.check(&Tok::LBrace) {
                    depth += 1;
                } else if self.check(&Tok::RBrace) {
                    depth -= 1;
                }
                self.advance();
            }
        }

        Ok(Stmt::InterfaceDecl(Interface {
            name,
            extends,
            type_params,
            loc,
        }))
    }

    fn parse_type_alias_decl(&mut self) -> Result<Stmt, String> {
        let loc = self.loc();
        self.expect(Tok::Type)?;
        let name = self.expect_ident_or_keyword()?;
        let type_params = self.parse_optional_type_params();
        self.expect(Tok::Assign)?;
        let ty = self.parse_type_annotation()?;
        self.eat_semicolon();

        Ok(Stmt::TypeAliasDecl(TypeAlias {
            name,
            type_params,
            ty,
            loc,
        }))
    }

    fn parse_enum_decl(&mut self) -> Result<Stmt, String> {
        let loc = self.loc();
        self.expect(Tok::Enum)?;
        let name = self.expect_ident_or_keyword()?;
        self.expect(Tok::LBrace)?;
        let mut members = Vec::new();

        while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
            let mname = self.expect_ident_or_keyword()?;
            let init = if self.eat(Tok::Assign) {
                Some(self.parse_assign_expr()?)
            } else {
                None
            };
            members.push(EnumMember { name: mname, init });
            if !self.eat(Tok::Comma) {
                break;
            }
        }
        self.expect(Tok::RBrace)?;

        Ok(Stmt::EnumDecl(Enum { name, members, loc }))
    }

    // ─── Imports & Exports ───────────────────────────────────────────────────

    fn parse_import_stmt(&mut self, loc: SourceLoc) -> Result<Stmt, String> {
        let is_type_only = self.eat(Tok::Type);
        let mut specifiers = Vec::new();

        if self.check(&Tok::Str("".into())) {
            // import 'path'
            let s = match self.advance().kind {
                Tok::Str(val) => val,
                _ => String::new(),
            };
            self.eat_semicolon();
            return Ok(Stmt::Import(ImportDecl {
                specifiers,
                source: s,
                is_type_only,
                loc,
            }));
        }

        if self.eat(Tok::Star) {
            // import * as name from 'path'
            self.eat(Tok::As);
            if let Ok(name) = self.expect_ident_or_keyword() {
                specifiers.push(name);
            }
        } else if self.eat(Tok::LBrace) {
            // import { a, b as c } from 'path'
            while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
                if let Ok(orig) = self.expect_ident_or_keyword() {
                    if self.eat(Tok::As) {
                        if let Ok(alias) = self.expect_ident_or_keyword() {
                            specifiers.push(alias);
                        }
                    } else {
                        specifiers.push(orig);
                    }
                }
                if !self.eat(Tok::Comma) {
                    break;
                }
            }
            self.eat(Tok::RBrace);
        } else if self.is_ident_or_keyword() {
            // default import: import name from 'path'
            if let Ok(name) = self.expect_ident_or_keyword() {
                specifiers.push(name);
            }
            if self.eat(Tok::Comma) && self.eat(Tok::LBrace) {
                while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
                    if let Ok(orig) = self.expect_ident_or_keyword() {
                        specifiers.push(orig);
                    }
                    if !self.eat(Tok::Comma) {
                        break;
                    }
                }
                self.eat(Tok::RBrace);
            }
        }

        self.eat(Tok::From);
        let source = match self.advance().kind {
            Tok::Str(val) => val,
            _ => String::new(),
        };
        self.eat_semicolon();

        Ok(Stmt::Import(ImportDecl {
            specifiers,
            source,
            is_type_only,
            loc,
        }))
    }

    fn parse_export_stmt(&mut self, loc: SourceLoc) -> Result<Stmt, String> {
        let is_default = self.eat(Tok::Default);

        if is_default {
            let expr = self.parse_assign_expr()?;
            self.eat_semicolon();
            return Ok(Stmt::Export(ExportDecl {
                decl: Some(Box::new(Stmt::Expr(expr, loc))),
                specifiers: Vec::new(),
                source: None,
                is_default: true,
                loc,
            }));
        }

        if self.eat(Tok::Star) {
            // export * from 'path'
            self.eat(Tok::From);
            let src = match self.advance().kind {
                Tok::Str(val) => Some(val),
                _ => None,
            };
            self.eat_semicolon();
            return Ok(Stmt::Export(ExportDecl {
                decl: None,
                specifiers: vec!["*".to_string()],
                source: src,
                is_default: false,
                loc,
            }));
        }

        if self.eat(Tok::LBrace) {
            // export { a, b as c } [from 'path']
            let mut specs = Vec::new();
            while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
                if let Ok(orig) = self.expect_ident_or_keyword() {
                    if self.eat(Tok::As) {
                        if let Ok(alias) = self.expect_ident_or_keyword() {
                            specs.push(alias);
                        }
                    } else {
                        specs.push(orig);
                    }
                }
                if !self.eat(Tok::Comma) {
                    break;
                }
            }
            self.eat(Tok::RBrace);
            let src = if self.eat(Tok::From) {
                match self.advance().kind {
                    Tok::Str(val) => Some(val),
                    _ => None,
                }
            } else {
                None
            };
            self.eat_semicolon();
            return Ok(Stmt::Export(ExportDecl {
                decl: None,
                specifiers: specs,
                source: src,
                is_default: false,
                loc,
            }));
        }

        // export const/let/function/class/interface/type/enum
        let inner = self.parse_stmt()?;
        Ok(Stmt::Export(ExportDecl {
            decl: Some(Box::new(inner)),
            specifiers: Vec::new(),
            source: None,
            is_default: false,
            loc,
        }))
    }

    // ─── Control Flow Statements ─────────────────────────────────────────────

    fn parse_if_stmt(&mut self) -> Result<Stmt, String> {
        let loc = self.loc();
        self.expect(Tok::If)?;
        self.expect(Tok::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(Tok::RParen)?;
        let then_branch = Box::new(self.parse_stmt()?);
        let else_branch = if self.eat(Tok::Else) {
            Some(Box::new(self.parse_stmt()?))
        } else {
            None
        };
        Ok(Stmt::If {
            cond,
            then_branch,
            else_branch,
            loc,
        })
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, String> {
        let loc = self.loc();
        self.expect(Tok::While)?;
        self.expect(Tok::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(Tok::RParen)?;
        let body = Box::new(self.parse_stmt()?);
        Ok(Stmt::While { cond, body, loc })
    }

    fn parse_do_while_stmt(&mut self) -> Result<Stmt, String> {
        let loc = self.loc();
        self.expect(Tok::Do)?;
        let body = Box::new(self.parse_stmt()?);
        self.expect(Tok::While)?;
        self.expect(Tok::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(Tok::RParen)?;
        self.eat_semicolon();
        Ok(Stmt::DoWhile { body, cond, loc })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, String> {
        let loc = self.loc();
        self.expect(Tok::For)?;
        self.expect(Tok::LParen)?;

        // Check for (const x of arr) or (let k in obj)
        let var_kind = match self.peek() {
            Tok::Const => {
                self.advance();
                Some(VarKind::Const)
            }
            Tok::Let => {
                self.advance();
                Some(VarKind::Let)
            }
            Tok::Var => {
                self.advance();
                Some(VarKind::Var)
            }
            _ => None,
        };

        if let Some(kind) = var_kind {
            let pattern = self.parse_binding_pattern()?;
            let ty = if self.eat(Tok::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };

            if self.eat(Tok::Of) {
                let iterable = self.parse_expr()?;
                self.expect(Tok::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                return Ok(Stmt::ForOf {
                    var: VarBinding {
                        kind: Some(kind),
                        pattern,
                        ty,
                    },
                    iterable,
                    body,
                    loc,
                });
            } else if self.eat(Tok::In) {
                let object = self.parse_expr()?;
                self.expect(Tok::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                return Ok(Stmt::ForIn {
                    var: VarBinding {
                        kind: Some(kind),
                        pattern,
                        ty,
                    },
                    object,
                    body,
                    loc,
                });
            } else {
                // Regular for loop with initialized var
                let init_expr = if self.eat(Tok::Assign) {
                    Some(self.parse_assign_expr()?)
                } else {
                    None
                };
                let mut decls = vec![VarDeclarator {
                    pattern,
                    ty,
                    init: init_expr,
                    loc,
                }];
                while self.eat(Tok::Comma) {
                    let p = self.parse_binding_pattern()?;
                    let t = if self.eat(Tok::Colon) {
                        Some(self.parse_type_annotation()?)
                    } else {
                        None
                    };
                    let i = if self.eat(Tok::Assign) {
                        Some(self.parse_assign_expr()?)
                    } else {
                        None
                    };
                    decls.push(VarDeclarator {
                        pattern: p,
                        ty: t,
                        init: i,
                        loc: self.loc(),
                    });
                }
                self.expect(Tok::Semicolon)?;

                let cond = if !self.check(&Tok::Semicolon) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect(Tok::Semicolon)?;

                let update = if !self.check(&Tok::RParen) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect(Tok::RParen)?;

                let body = Box::new(self.parse_stmt()?);
                return Ok(Stmt::For {
                    init: Some(ForInit::VarDecl { kind, decls }),
                    cond,
                    update,
                    body,
                    loc,
                });
            }
        }

        // Standard for loop without var kind or empty init
        let init = if self.eat(Tok::Semicolon) {
            None
        } else {
            let expr = self.parse_expr()?;
            self.expect(Tok::Semicolon)?;
            Some(ForInit::Expr(expr))
        };

        let cond = if !self.check(&Tok::Semicolon) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect(Tok::Semicolon)?;

        let update = if !self.check(&Tok::RParen) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect(Tok::RParen)?;

        let body = Box::new(self.parse_stmt()?);
        Ok(Stmt::For {
            init,
            cond,
            update,
            body,
            loc,
        })
    }

    fn parse_switch_stmt(&mut self) -> Result<Stmt, String> {
        let loc = self.loc();
        self.expect(Tok::Switch)?;
        self.expect(Tok::LParen)?;
        let discriminant = self.parse_expr()?;
        self.expect(Tok::RParen)?;
        self.expect(Tok::LBrace)?;

        let mut cases = Vec::new();
        while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
            let test = if self.eat(Tok::Case) {
                let e = self.parse_expr()?;
                self.expect(Tok::Colon)?;
                Some(e)
            } else if self.eat(Tok::Default) {
                self.expect(Tok::Colon)?;
                None
            } else {
                self.advance();
                continue;
            };

            let mut stmts = Vec::new();
            while !self.check(&Tok::Case)
                && !self.check(&Tok::Default)
                && !self.check(&Tok::RBrace)
                && !self.check(&Tok::Eof)
            {
                stmts.push(self.parse_stmt()?);
            }
            cases.push(SwitchCase { test, stmts });
        }
        self.expect(Tok::RBrace)?;

        Ok(Stmt::Switch {
            discriminant,
            cases,
            loc,
        })
    }

    fn parse_try_stmt(&mut self) -> Result<Stmt, String> {
        let loc = self.loc();
        self.expect(Tok::Try)?;
        let body = self.parse_block()?;

        let catch_clause = if self.eat(Tok::Catch) {
            let param = if self.eat(Tok::LParen) {
                let pat = self.parse_binding_pattern()?;
                if self.eat(Tok::Colon) {
                    let _ = self.parse_type_annotation();
                }
                self.expect(Tok::RParen)?;
                Some(pat)
            } else {
                None
            };
            let cbody = self.parse_block()?;
            Some(CatchClause {
                param,
                body: cbody,
            })
        } else {
            None
        };

        let finally_block = if self.eat(Tok::Finally) {
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Stmt::Try {
            body,
            catch_clause,
            finally_block,
            loc,
        })
    }

    // ─── Type Annotations ────────────────────────────────────────────────────

    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, String> {
        let mut types = Vec::new();
        loop {
            let ty = self.parse_single_type()?;
            types.push(ty);
            if self.eat(Tok::Pipe) {
                continue;
            } else if self.eat(Tok::Amp) {
                // Intersection
                continue;
            } else {
                break;
            }
        }
        if types.len() == 1 {
            Ok(types.remove(0))
        } else {
            Ok(TypeAnnotation::Union(types))
        }
    }

    fn parse_single_type(&mut self) -> Result<TypeAnnotation, String> {
        let mut base = if self.eat(Tok::LParen) {
            // Could be function type (a: T) => R or parenthesized type (T | U)
            if self.check(&Tok::RParen) || self.peek_at(1) == &Tok::Colon {
                // Function type parameter list
                let mut params = Vec::new();
                while !self.check(&Tok::RParen) && !self.check(&Tok::Eof) {
                    let _ = self.expect_ident_or_keyword();
                    let _ = self.eat(Tok::Question);
                    if self.eat(Tok::Colon) {
                        params.push(self.parse_type_annotation()?);
                    }
                    if !self.eat(Tok::Comma) {
                        break;
                    }
                }
                self.expect(Tok::RParen)?;
                if self.eat(Tok::Arrow) {
                    let ret = self.parse_type_annotation()?;
                    TypeAnnotation::Function {
                        params,
                        return_ty: Box::new(ret),
                    }
                } else {
                    TypeAnnotation::Tuple(params)
                }
            } else {
                let inner = self.parse_type_annotation()?;
                self.expect(Tok::RParen)?;
                if self.eat(Tok::Arrow) {
                    let ret = self.parse_type_annotation()?;
                    TypeAnnotation::Function {
                        params: vec![inner],
                        return_ty: Box::new(ret),
                    }
                } else {
                    inner
                }
            }
        } else if self.eat(Tok::LBracket) {
            // Tuple type [A, B]
            let mut elems = Vec::new();
            while !self.check(&Tok::RBracket) && !self.check(&Tok::Eof) {
                elems.push(self.parse_type_annotation()?);
                if !self.eat(Tok::Comma) {
                    break;
                }
            }
            self.expect(Tok::RBracket)?;
            TypeAnnotation::Tuple(elems)
        } else if self.eat(Tok::LBrace) {
            // Object type { ... }
            let mut depth = 1;
            while depth > 0 && !self.check(&Tok::Eof) {
                if self.check(&Tok::LBrace) {
                    depth += 1;
                } else if self.check(&Tok::RBrace) {
                    depth -= 1;
                }
                self.advance();
            }
            TypeAnnotation::Object
        } else if let Ok(name) = self.expect_ident_or_keyword() {
            match name.as_str() {
                "any" => TypeAnnotation::Any,
                "unknown" => TypeAnnotation::Unknown,
                "void" => TypeAnnotation::Void,
                _ => {
                    // Check for generics: Name<T, U>
                    if self.check(&Tok::Lt) {
                        let args = self.parse_type_args()?;
                        TypeAnnotation::Generic(name, args)
                    } else {
                        // Check for dotted type name: Config.InitialOptions
                        let mut full_name = name;
                        while self.eat(Tok::Dot) {
                            if let Ok(part) = self.expect_ident_or_keyword() {
                                full_name.push('.');
                                full_name.push_str(&part);
                            }
                        }
                        TypeAnnotation::Basic(full_name)
                    }
                }
            }
        } else if let Tok::Str(s) = self.peek().clone() {
            self.advance();
            TypeAnnotation::Literal(s)
        } else {
            self.advance();
            TypeAnnotation::Basic("unknown".to_string())
        };

        // Check for array suffixes: T[], T[][], etc.
        while self.eat(Tok::LBracket) {
            self.expect(Tok::RBracket)?;
            base = TypeAnnotation::Array(Box::new(base));
        }

        Ok(base)
    }

    fn parse_type_args(&mut self) -> Result<Vec<TypeAnnotation>, String> {
        self.expect(Tok::Lt)?;
        let mut args = Vec::new();
        while !self.check(&Tok::Gt) && !self.check(&Tok::Eof) {
            args.push(self.parse_type_annotation()?);
            if !self.eat(Tok::Comma) {
                break;
            }
        }
        self.expect(Tok::Gt)?;
        Ok(args)
    }

    fn parse_type_list(&mut self) -> Result<Vec<TypeAnnotation>, String> {
        let mut list = Vec::new();
        loop {
            list.push(self.parse_type_annotation()?);
            if !self.eat(Tok::Comma) {
                break;
            }
        }
        Ok(list)
    }

    // ─── Expressions ─────────────────────────────────────────────────────────

    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_assign_expr()?;
        if self.check(&Tok::Comma) {
            let loc = expr.loc();
            let mut seq = vec![expr];
            while self.eat(Tok::Comma) {
                seq.push(self.parse_assign_expr()?);
            }
            expr = Expr::Sequence(seq, loc);
        }
        Ok(expr)
    }

    pub fn parse_assign_expr(&mut self) -> Result<Expr, String> {
        let left = self.parse_conditional_expr()?;

        let op = match self.peek() {
            Tok::Assign => Some(AssignOp::Assign),
            Tok::PlusAssign => Some(AssignOp::AddAssign),
            Tok::MinusAssign => Some(AssignOp::SubAssign),
            Tok::StarAssign => Some(AssignOp::MulAssign),
            Tok::SlashAssign => Some(AssignOp::DivAssign),
            Tok::PercentAssign => Some(AssignOp::ModAssign),
            Tok::StarStarAssign => Some(AssignOp::ExpAssign),
            Tok::AmpAssign => Some(AssignOp::BitAndAssign),
            Tok::PipeAssign => Some(AssignOp::BitOrAssign),
            Tok::CaretAssign => Some(AssignOp::BitXorAssign),
            Tok::LtLtAssign => Some(AssignOp::ShlAssign),
            Tok::GtGtAssign => Some(AssignOp::ShrAssign),
            Tok::GtGtGtAssign => Some(AssignOp::ShrUnsignedAssign),
            Tok::AmpAmpAssign => Some(AssignOp::AndAssign),
            Tok::PipePipeAssign => Some(AssignOp::OrAssign),
            Tok::QuestionQuestionAssign => Some(AssignOp::NullishAssign),
            _ => None,
        };

        if let Some(assign_op) = op {
            let loc = self.loc();
            self.advance();
            let right = self.parse_assign_expr()?;
            return Ok(Expr::Assign {
                op: assign_op,
                left: Box::new(left),
                right: Box::new(right),
                loc,
            });
        }

        Ok(left)
    }

    fn parse_conditional_expr(&mut self) -> Result<Expr, String> {
        let cond = self.parse_logical_or_expr()?;

        if self.eat(Tok::Question) {
            let loc = self.loc();
            let then_expr = self.parse_assign_expr()?;
            self.expect(Tok::Colon)?;
            let else_expr = self.parse_assign_expr()?;
            return Ok(Expr::Conditional {
                cond: Box::new(cond),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
                loc,
            });
        }

        Ok(cond)
    }

    fn parse_logical_or_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_logical_and_expr()?;

        while self.check(&Tok::PipePipe) || self.check(&Tok::QuestionQuestion) {
            let loc = self.loc();
            let tok = self.advance();
            let op = match tok.kind {
                Tok::PipePipe => LogicalOp::Or,
                Tok::QuestionQuestion => LogicalOp::NullishCoalesce,
                _ => unreachable!(),
            };
            let right = self.parse_logical_and_expr()?;
            left = Expr::Logical {
                op,
                left: Box::new(left),
                right: Box::new(right),
                loc,
            };
        }

        Ok(left)
    }

    fn parse_logical_and_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_or_expr()?;

        while self.eat(Tok::AmpAmp) {
            let loc = self.loc();
            let right = self.parse_bitwise_or_expr()?;
            left = Expr::Logical {
                op: LogicalOp::And,
                left: Box::new(left),
                right: Box::new(right),
                loc,
            };
        }

        Ok(left)
    }

    fn parse_bitwise_or_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_xor_expr()?;
        while self.eat(Tok::Pipe) {
            let loc = self.loc();
            let right = self.parse_bitwise_xor_expr()?;
            left = Expr::Binary {
                op: BinaryOp::BitOr,
                left: Box::new(left),
                right: Box::new(right),
                loc,
            };
        }
        Ok(left)
    }

    fn parse_bitwise_xor_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_and_expr()?;
        while self.eat(Tok::Caret) {
            let loc = self.loc();
            let right = self.parse_bitwise_and_expr()?;
            left = Expr::Binary {
                op: BinaryOp::BitXor,
                left: Box::new(left),
                right: Box::new(right),
                loc,
            };
        }
        Ok(left)
    }

    fn parse_bitwise_and_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality_expr()?;
        while self.eat(Tok::Amp) {
            let loc = self.loc();
            let right = self.parse_equality_expr()?;
            left = Expr::Binary {
                op: BinaryOp::BitAnd,
                left: Box::new(left),
                right: Box::new(right),
                loc,
            };
        }
        Ok(left)
    }

    fn parse_equality_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_relational_expr()?;

        while matches!(
            self.peek(),
            Tok::EqEq | Tok::EqEqEq | Tok::NotEq | Tok::NotEqEq
        ) {
            let loc = self.loc();
            let tok = self.advance();
            let op = match tok.kind {
                Tok::EqEq => BinaryOp::EqEq,
                Tok::EqEqEq => BinaryOp::EqEqEq,
                Tok::NotEq => BinaryOp::NotEq,
                Tok::NotEqEq => BinaryOp::NotEqEq,
                _ => unreachable!(),
            };
            let right = self.parse_relational_expr()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                loc,
            };
        }

        Ok(left)
    }

    fn parse_relational_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_shift_expr()?;

        loop {
            let loc = self.loc();
            let op = match self.peek() {
                Tok::Lt => Some(BinaryOp::Lt),
                Tok::LtEq => Some(BinaryOp::LtEq),
                Tok::Gt => Some(BinaryOp::Gt),
                Tok::GtEq => Some(BinaryOp::GtEq),
                Tok::Instanceof => Some(BinaryOp::Instanceof),
                Tok::In => Some(BinaryOp::In),
                _ => None,
            };

            if let Some(bin_op) = op {
                self.advance();
                let right = self.parse_shift_expr()?;
                left = Expr::Binary {
                    op: bin_op,
                    left: Box::new(left),
                    right: Box::new(right),
                    loc,
                };
            } else if self.eat(Tok::As) {
                // Type assertion: expr as Type
                let ty = self.parse_type_annotation()?;
                left = Expr::TypeAssertion {
                    expr: Box::new(left),
                    is_non_null: false,
                    ty: Some(ty),
                    loc,
                };
            } else if self.eat(Tok::Satisfies) {
                let ty = self.parse_type_annotation()?;
                left = Expr::TypeAssertion {
                    expr: Box::new(left),
                    is_non_null: false,
                    ty: Some(ty),
                    loc,
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_shift_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive_expr()?;

        while matches!(self.peek(), Tok::LtLt | Tok::GtGt | Tok::GtGtGt) {
            let loc = self.loc();
            let tok = self.advance();
            let op = match tok.kind {
                Tok::LtLt => BinaryOp::Shl,
                Tok::GtGt => BinaryOp::Shr,
                Tok::GtGtGt => BinaryOp::ShrUnsigned,
                _ => unreachable!(),
            };
            let right = self.parse_additive_expr()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                loc,
            };
        }

        Ok(left)
    }

    fn parse_additive_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative_expr()?;

        while matches!(self.peek(), Tok::Plus | Tok::Minus) {
            let loc = self.loc();
            let tok = self.advance();
            let op = match tok.kind {
                Tok::Plus => BinaryOp::Add,
                Tok::Minus => BinaryOp::Sub,
                _ => unreachable!(),
            };
            let right = self.parse_multiplicative_expr()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                loc,
            };
        }

        Ok(left)
    }

    fn parse_multiplicative_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_exponentiation_expr()?;

        while matches!(self.peek(), Tok::Star | Tok::Slash | Tok::Percent) {
            let loc = self.loc();
            let tok = self.advance();
            let op = match tok.kind {
                Tok::Star => BinaryOp::Mul,
                Tok::Slash => BinaryOp::Div,
                Tok::Percent => BinaryOp::Mod,
                _ => unreachable!(),
            };
            let right = self.parse_exponentiation_expr()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                loc,
            };
        }

        Ok(left)
    }

    fn parse_exponentiation_expr(&mut self) -> Result<Expr, String> {
        let left = self.parse_unary_expr()?;

        if self.eat(Tok::StarStar) {
            let loc = self.loc();
            let right = self.parse_exponentiation_expr()?;
            return Ok(Expr::Binary {
                op: BinaryOp::Exp,
                left: Box::new(left),
                right: Box::new(right),
                loc,
            });
        }

        Ok(left)
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, String> {
        let loc = self.loc();

        match self.peek() {
            Tok::Bang => {
                self.advance();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                    loc,
                })
            }
            Tok::Tilde => {
                self.advance();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::Unary {
                    op: UnaryOp::BitNot,
                    expr: Box::new(expr),
                    loc,
                })
            }
            Tok::Plus => {
                self.advance();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Plus,
                    expr: Box::new(expr),
                    loc,
                })
            }
            Tok::Minus => {
                self.advance();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Minus,
                    expr: Box::new(expr),
                    loc,
                })
            }
            Tok::PlusPlus => {
                self.advance();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::Update {
                    op: UpdateOp::Inc,
                    expr: Box::new(expr),
                    prefix: true,
                    loc,
                })
            }
            Tok::MinusMinus => {
                self.advance();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::Update {
                    op: UpdateOp::Dec,
                    expr: Box::new(expr),
                    prefix: true,
                    loc,
                })
            }
            Tok::Typeof => {
                self.advance();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Typeof,
                    expr: Box::new(expr),
                    loc,
                })
            }
            Tok::Void => {
                self.advance();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Void,
                    expr: Box::new(expr),
                    loc,
                })
            }
            Tok::Delete => {
                self.advance();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Delete,
                    expr: Box::new(expr),
                    loc,
                })
            }
            Tok::Await => {
                self.advance();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Await,
                    expr: Box::new(expr),
                    loc,
                })
            }
            _ => self.parse_postfix_expr(),
        }
    }

    fn parse_postfix_expr(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_lhs_expr()?;

        loop {
            let loc = self.loc();
            if self.peek_tok().has_newline_before {
                break;
            }
            match self.peek() {
                Tok::PlusPlus => {
                    self.advance();
                    expr = Expr::Update {
                        op: UpdateOp::Inc,
                        expr: Box::new(expr),
                        prefix: false,
                        loc,
                    };
                }
                Tok::MinusMinus => {
                    self.advance();
                    expr = Expr::Update {
                        op: UpdateOp::Dec,
                        expr: Box::new(expr),
                        prefix: false,
                        loc,
                    };
                }
                Tok::Bang => {
                    // Non-null assertion: expr!
                    self.advance();
                    expr = Expr::TypeAssertion {
                        expr: Box::new(expr),
                        is_non_null: true,
                        ty: None,
                        loc,
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_lhs_expr(&mut self) -> Result<Expr, String> {
        let mut expr = if self.eat(Tok::New) {
            let loc = self.loc();
            let callee = self.parse_primary_expr()?;
            let args = if self.check(&Tok::LParen) {
                self.parse_args()?
            } else {
                Vec::new()
            };
            Expr::New {
                callee: Box::new(callee),
                args,
                loc,
            }
        } else {
            self.parse_primary_expr()?
        };

        loop {
            let loc = self.loc();
            match self.peek() {
                Tok::Dot => {
                    self.advance();
                    let prop = self.expect_ident_or_keyword()?;
                    expr = Expr::Member {
                        object: Box::new(expr),
                        property: prop,
                        optional: false,
                        loc,
                    };
                }
                Tok::QuestionDot => {
                    self.advance();
                    if self.check(&Tok::LBracket) {
                        self.advance();
                        let idx = self.parse_expr()?;
                        self.expect(Tok::RBracket)?;
                        expr = Expr::Index {
                            object: Box::new(expr),
                            index: Box::new(idx),
                            optional: true,
                            loc,
                        };
                    } else if self.check(&Tok::LParen) {
                        let args = self.parse_args()?;
                        expr = Expr::Call {
                            callee: Box::new(expr),
                            args,
                            optional: true,
                            loc,
                        };
                    } else {
                        let prop = self.expect_ident_or_keyword()?;
                        expr = Expr::Member {
                            object: Box::new(expr),
                            property: prop,
                            optional: true,
                            loc,
                        };
                    }
                }
                Tok::LBracket => {
                    self.advance();
                    let idx = self.parse_expr()?;
                    self.expect(Tok::RBracket)?;
                    expr = Expr::Index {
                        object: Box::new(expr),
                        index: Box::new(idx),
                        optional: false,
                        loc,
                    };
                }
                Tok::LParen => {
                    let args = self.parse_args()?;
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                        optional: false,
                        loc,
                    };
                }
                Tok::Lt if self.is_generic_call_ahead() => {
                    // Type args before call: foo<T>(x)
                    let _ = self.parse_type_args();
                    let args = self.parse_args()?;
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                        optional: false,
                        loc,
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn is_generic_call_ahead(&self) -> bool {
        if self.peek() != &Tok::Lt {
            return false;
        }
        let mut depth = 0;
        let mut offset = 0;
        while let Some(tok) = self.tokens.get(self.pos + offset) {
            match tok.kind {
                Tok::Lt => depth += 1,
                Tok::Gt => {
                    depth -= 1;
                    if depth == 0 {
                        // Check if followed by LParen
                        return self.tokens.get(self.pos + offset + 1).map(|t| &t.kind)
                            == Some(&Tok::LParen);
                    }
                }
                Tok::Semicolon | Tok::RBrace | Tok::LBrace | Tok::Eof => return false,
                _ => {}
            }
            offset += 1;
            if offset > 30 {
                break;
            }
        }
        false
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, String> {
        self.expect(Tok::LParen)?;
        let mut args = Vec::new();
        while !self.check(&Tok::RParen) && !self.check(&Tok::Eof) {
            if self.eat(Tok::Ellipsis) {
                let e = self.parse_assign_expr()?;
                args.push(e);
            } else {
                args.push(self.parse_assign_expr()?);
            }
            if !self.eat(Tok::Comma) {
                break;
            }
        }
        self.expect(Tok::RParen)?;
        Ok(args)
    }

    // ─── Primary Expressions ─────────────────────────────────────────────────

    fn parse_primary_expr(&mut self) -> Result<Expr, String> {
        let loc = self.loc();

        // Arrow function with single parameter: x => expr
        if self.is_ident_or_keyword() && self.peek_at(1) == &Tok::Arrow {
            let name = self.expect_ident_or_keyword()?;
            self.expect(Tok::Arrow)?;
            let body = self.parse_arrow_body()?;
            return Ok(Expr::ArrowFunction {
                params: vec![Param {
                    pattern: BindingPattern::Ident(name),
                    ty: None,
                    default_value: None,
                    is_optional: false,
                    is_rest: false,
                }],
                return_ty: None,
                body,
                is_async: false,
                loc,
            });
        }

        // Async arrow function: async x => expr
        if self.check(&Tok::Async) && self.is_async_arrow_ahead() {
            self.advance(); // async
            if self.is_ident_or_keyword() {
                let name = self.expect_ident_or_keyword()?;
                self.expect(Tok::Arrow)?;
                let body = self.parse_arrow_body()?;
                return Ok(Expr::ArrowFunction {
                    params: vec![Param {
                        pattern: BindingPattern::Ident(name),
                        ty: None,
                        default_value: None,
                        is_optional: false,
                        is_rest: false,
                    }],
                    return_ty: None,
                    body,
                    is_async: true,
                    loc,
                });
            }
        }

        match self.peek().clone() {
            Tok::Null => {
                self.advance();
                Ok(Expr::Null(loc))
            }
            Tok::Undefined => {
                self.advance();
                Ok(Expr::Undefined(loc))
            }
            Tok::Nan => {
                self.advance();
                Ok(Expr::Number(f64::NAN, "NaN".to_string(), loc))
            }
            Tok::True => {
                self.advance();
                Ok(Expr::Bool(true, loc))
            }
            Tok::False => {
                self.advance();
                Ok(Expr::Bool(false, loc))
            }
            Tok::This => {
                self.advance();
                Ok(Expr::This(loc))
            }
            Tok::Super => {
                self.advance();
                Ok(Expr::Super(loc))
            }
            Tok::Int(val, raw) => {
                self.advance();
                Ok(Expr::Number(val as f64, raw, loc))
            }
            Tok::Float(val, raw) => {
                self.advance();
                Ok(Expr::Number(val, raw, loc))
            }
            Tok::Str(s) => {
                self.advance();
                Ok(Expr::StringLit(s, loc))
            }
            Tok::Regex(p, f) => {
                self.advance();
                Ok(Expr::RegexLit(p, f, loc))
            }
            Tok::TemplateNoSubst(s) => {
                self.advance();
                Ok(Expr::TemplateLit {
                    quasis: vec![s],
                    expressions: Vec::new(),
                    loc,
                })
            }
            Tok::TemplateHead(head) => {
                self.advance();
                let mut quasis = vec![head];
                let mut exprs = Vec::new();

                loop {
                    exprs.push(self.parse_expr()?);
                    match self.peek().clone() {
                        Tok::TemplateMiddle(mid) => {
                            self.advance();
                            quasis.push(mid);
                        }
                        Tok::TemplateTail(tail) => {
                            self.advance();
                            quasis.push(tail);
                            break;
                        }
                        _ => break,
                    }
                }
                Ok(Expr::TemplateLit {
                    quasis,
                    expressions: exprs,
                    loc,
                })
            }
            Tok::LBracket => self.parse_array_lit(),
            Tok::LBrace => self.parse_object_lit(),
            Tok::Function => {
                self.advance();
                let is_generator = self.eat(Tok::Star);
                let name = if self.is_ident_or_keyword() {
                    Some(self.expect_ident_or_keyword()?)
                } else {
                    None
                };
                let type_params = self.parse_optional_type_params();
                let params = self.parse_params()?;
                let return_ty = if self.eat(Tok::Colon) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };
                let body = Some(self.parse_block()?);
                Ok(Expr::FunctionExpr(Function {
                    name,
                    params,
                    return_ty,
                    body,
                    is_async: false,
                    is_generator,
                    type_params,
                    loc,
                }))
            }
            Tok::LParen => self.parse_paren_or_arrow_expr(),
            _ if self.is_ident_or_keyword() => {
                let name = self.expect_ident_or_keyword()?;
                Ok(Expr::Ident(name, loc))
            }
            other => Err(format!(
                "unexpected token {:?} in expression at {}",
                other, loc
            )),
        }
    }

    fn is_async_arrow_ahead(&self) -> bool {
        if self.peek() != &Tok::Async {
            return false;
        }
        let next = self.peek_at(1);
        if matches!(next, Tok::Ident(_)) && self.peek_at(2) == &Tok::Arrow {
            return true;
        }
        if next == &Tok::LParen {
            return self.is_arrow_after_paren_group(1);
        }
        false
    }

    fn parse_paren_or_arrow_expr(&mut self) -> Result<Expr, String> {
        let loc = self.loc();

        if self.is_arrow_after_paren_group(0) {
            // It's an arrow function: (a, b): Ret => ...
            let params = self.parse_params()?;
            let return_ty = if self.eat(Tok::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            self.expect(Tok::Arrow)?;
            let body = self.parse_arrow_body()?;
            return Ok(Expr::ArrowFunction {
                params,
                return_ty,
                body,
                is_async: false,
                loc,
            });
        }

        // Parenthesized expression
        self.expect(Tok::LParen)?;
        let expr = self.parse_expr()?;
        self.expect(Tok::RParen)?;
        Ok(expr)
    }

    fn is_arrow_after_paren_group(&self, start_offset: usize) -> bool {
        let mut depth = 0;
        let mut offset = start_offset;

        while let Some(tok) = self.tokens.get(self.pos + offset) {
            match tok.kind {
                Tok::LParen => depth += 1,
                Tok::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        // Check what follows ')'
                        let mut next_offset = self.pos + offset + 1;
                        if self.tokens.get(next_offset).map(|t| &t.kind) == Some(&Tok::Colon) {
                            // Skip return type
                            next_offset += 1;
                            while let Some(t) = self.tokens.get(next_offset) {
                                if t.kind == Tok::Arrow {
                                    return true;
                                }
                                if t.kind == Tok::LBrace
                                    || t.kind == Tok::Semicolon
                                    || t.kind == Tok::Eof
                                {
                                    break;
                                }
                                next_offset += 1;
                            }
                            return false;
                        }
                        return self.tokens.get(next_offset).map(|t| &t.kind) == Some(&Tok::Arrow);
                    }
                }
                Tok::Semicolon | Tok::RBrace | Tok::LBrace | Tok::Eof => return false,
                _ => {}
            }
            offset += 1;
            if offset > 50 {
                break;
            }
        }
        false
    }

    fn parse_arrow_body(&mut self) -> Result<ArrowBody, String> {
        if self.check(&Tok::LBrace) {
            let block = self.parse_block()?;
            Ok(ArrowBody::Block(block))
        } else {
            let expr = self.parse_assign_expr()?;
            Ok(ArrowBody::Expr(Box::new(expr)))
        }
    }

    fn parse_array_lit(&mut self) -> Result<Expr, String> {
        let loc = self.loc();
        self.expect(Tok::LBracket)?;
        let mut elems = Vec::new();

        while !self.check(&Tok::RBracket) && !self.check(&Tok::Eof) {
            if self.eat(Tok::Comma) {
                elems.push(ArrayElem::Hole);
                continue;
            }
            if self.eat(Tok::Ellipsis) {
                let e = self.parse_assign_expr()?;
                elems.push(ArrayElem::Spread(e));
            } else {
                let e = self.parse_assign_expr()?;
                elems.push(ArrayElem::Expr(e));
            }
            if !self.eat(Tok::Comma) {
                break;
            }
        }

        self.expect(Tok::RBracket)?;
        Ok(Expr::ArrayLit(elems, loc))
    }

    fn parse_object_lit(&mut self) -> Result<Expr, String> {
        let loc = self.loc();
        self.expect(Tok::LBrace)?;
        let mut props = Vec::new();

        while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
            if self.eat(Tok::Ellipsis) {
                let expr = self.parse_assign_expr()?;
                props.push(ObjectProp::Spread(expr));
                if !self.eat(Tok::Comma) {
                    break;
                }
                continue;
            }

            let prop_loc = self.loc();
            let (key, computed) = if self.eat(Tok::LBracket) {
                let e = self.parse_expr()?;
                self.expect(Tok::RBracket)?;
                (e, true)
            } else if let Tok::Str(s) = self.peek().clone() {
                self.advance();
                (Expr::StringLit(s, prop_loc), false)
            } else if let Tok::Int(val, raw) = self.peek().clone() {
                self.advance();
                (Expr::Number(val as f64, raw, prop_loc), false)
            } else {
                let name = self.expect_ident_or_keyword()?;
                (Expr::Ident(name, prop_loc), false)
            };

            // Check if method: key(params) { body }
            if self.check(&Tok::LParen) {
                let params = self.parse_params()?;
                let return_ty = if self.eat(Tok::Colon) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };
                let body = Some(self.parse_block()?);
                props.push(ObjectProp::Method(Function {
                    name: key.as_ident().map(|s| s.to_string()),
                    params,
                    return_ty,
                    body,
                    is_async: false,
                    is_generator: false,
                    type_params: Vec::new(),
                    loc: prop_loc,
                }));
            } else if self.eat(Tok::Colon) {
                let val = self.parse_assign_expr()?;
                props.push(ObjectProp::Property {
                    key,
                    value: val,
                    shorthand: false,
                    computed,
                });
            } else {
                // Shorthand: { x }
                props.push(ObjectProp::Property {
                    key: key.clone(),
                    value: key,
                    shorthand: true,
                    computed,
                });
            }

            if !self.eat(Tok::Comma) {
                break;
            }
        }

        self.expect(Tok::RBrace)?;
        Ok(Expr::ObjectLit(props, loc))
    }
}

/// Convenience entry point to parse a TypeScript / JavaScript source string.
pub fn parse(source: &str, path: &str) -> Result<SourceFile, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    Ok(parser.parse_source_file(path))
}
