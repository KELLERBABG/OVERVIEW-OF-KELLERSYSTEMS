#![allow(dead_code, unused_variables, unused_imports, private_interfaces)]

//! Fast, resilient recursive descent parser for Java source code (.java).

use crate::java::ast::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    // Literals
    Int(i64),
    Float(f64),
    Str(String),
    Char(char),
    True,
    False,
    Null,

    // Identifiers
    Ident(String),

    // Keywords
    Abstract,
    Assert,
    Boolean,
    Break,
    Byte,
    Case,
    Catch,
    CharKw,
    Class,
    Const,
    Continue,
    Default,
    Do,
    Double,
    Else,
    Enum,
    Extends,
    Final,
    Finally,
    FloatKw,
    For,
    Goto,
    If,
    Implements,
    Import,
    Instanceof,
    IntKw,
    Interface,
    Long,
    Native,
    New,
    Package,
    Private,
    Protected,
    Public,
    Return,
    Short,
    Static,
    Strictfp,
    Super,
    Switch,
    Synchronized,
    This,
    Throw,
    Throws,
    Transient,
    Try,
    Void,
    Volatile,
    While,

    // Contextual Keywords
    Record,
    Var,
    Yield,
    Sealed,
    NonSealed,
    Permits,

    // Delimiters & Operators
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Comma,
    Dot,
    Ellipsis,
    At,
    ColonColon,
    Arrow,
    Colon,
    Question,

    // Operators
    Assign,
    PlusAssign,
    MinusAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
    UshrAssign,

    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,

    And,
    Or,
    Not,
    BitNot,
    PlusPlus,
    MinusMinus,

    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    Pipe,
    Caret,
    Shl,
    Shr,
    Ushr,

    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Tok,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src: src.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
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
                Some(b' ' | b'\t' | b'\r' | b'\n' | 0x0C) => {
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
                        // Block / doc comment
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

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            let line = self.line;
            let col = self.col;
            let Some(b) = self.peek() else {
                tokens.push(Token {
                    kind: Tok::Eof,
                    line,
                    col,
                });
                break;
            };

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
                    Tok::LBrace
                }
                b'}' => {
                    self.advance();
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
                b'?' => {
                    self.advance();
                    Tok::Question
                }
                b'~' => {
                    self.advance();
                    Tok::BitNot
                }
                b'@' => {
                    self.advance();
                    Tok::At
                }

                b':' => {
                    self.advance();
                    if self.peek() == Some(b':') {
                        self.advance();
                        Tok::ColonColon
                    } else {
                        Tok::Colon
                    }
                }

                b'.' => {
                    if self.peek_at(1) == Some(b'.') && self.peek_at(2) == Some(b'.') {
                        self.advance();
                        self.advance();
                        self.advance();
                        Tok::Ellipsis
                    } else if self.peek_at(1).map_or(false, |c| c.is_ascii_digit()) {
                        self.read_number(line, col)
                    } else {
                        self.advance();
                        Tok::Dot
                    }
                }

                b'=' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::Eq
                    } else {
                        Tok::Assign
                    }
                }

                b'!' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::Ne
                    } else {
                        Tok::Not
                    }
                }

                b'<' => {
                    self.advance();
                    if self.peek() == Some(b'<') {
                        self.advance();
                        if self.peek() == Some(b'=') {
                            self.advance();
                            Tok::ShlAssign
                        } else {
                            Tok::Shl
                        }
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::Le
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
                                Tok::UshrAssign
                            } else {
                                Tok::Ushr
                            }
                        } else if self.peek() == Some(b'=') {
                            self.advance();
                            Tok::ShrAssign
                        } else {
                            Tok::Shr
                        }
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::Ge
                    } else {
                        Tok::Gt
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
                    } else if self.peek() == Some(b'>') {
                        self.advance();
                        Tok::Arrow
                    } else {
                        Tok::Minus
                    }
                }

                b'*' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::MulAssign
                    } else {
                        Tok::Star
                    }
                }

                b'/' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::DivAssign
                    } else {
                        Tok::Slash
                    }
                }

                b'%' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::RemAssign
                    } else {
                        Tok::Percent
                    }
                }

                b'&' => {
                    self.advance();
                    if self.peek() == Some(b'&') {
                        self.advance();
                        Tok::And
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::BitAndAssign
                    } else {
                        Tok::Amp
                    }
                }

                b'|' => {
                    self.advance();
                    if self.peek() == Some(b'|') {
                        self.advance();
                        Tok::Or
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::BitOrAssign
                    } else {
                        Tok::Pipe
                    }
                }

                b'^' => {
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::BitXorAssign
                    } else {
                        Tok::Caret
                    }
                }

                b'"' => self.read_string(),

                b'\'' => self.read_char(),

                b'0'..=b'9' => self.read_number(line, col),

                b if is_ident_start(b) => self.read_ident_or_keyword(),

                _ => {
                    // Unknown byte, advance to avoid stalling
                    self.advance();
                    continue;
                }
            };

            tokens.push(Token { kind, line, col });
        }
        tokens
    }

    fn read_string(&mut self) -> Tok {
        self.advance(); // consume opening "
        if self.peek() == Some(b'"') && self.peek_at(1) == Some(b'"') {
            // Text block """ ... """
            self.advance();
            self.advance();
            let mut s = String::new();
            while let Some(c) = self.peek() {
                if c == b'"' && self.peek_at(1) == Some(b'"') && self.peek_at(2) == Some(b'"') {
                    self.advance();
                    self.advance();
                    self.advance();
                    break;
                }
                if c == b'\\' {
                    self.advance();
                    if let Some(esc) = self.advance() {
                        match esc {
                            b'n' => s.push('\n'),
                            b't' => s.push('\t'),
                            b'r' => s.push('\r'),
                            b'\\' => s.push('\\'),
                            b'"' => s.push('"'),
                            _ => s.push(esc as char),
                        }
                    }
                } else {
                    s.push(c as char);
                    self.advance();
                }
            }
            Tok::Str(s)
        } else {
            let mut s = String::new();
            while let Some(c) = self.peek() {
                if c == b'"' {
                    self.advance();
                    break;
                }
                if c == b'\n' {
                    break; // unclosed string
                }
                if c == b'\\' {
                    self.advance();
                    if let Some(esc) = self.advance() {
                        match esc {
                            b'n' => s.push('\n'),
                            b't' => s.push('\t'),
                            b'r' => s.push('\r'),
                            b'b' => s.push('\x08'),
                            b'f' => s.push('\x0c'),
                            b'\\' => s.push('\\'),
                            b'"' => s.push('"'),
                            b'\'' => s.push('\''),
                            _ => s.push(esc as char),
                        }
                    }
                } else {
                    s.push(c as char);
                    self.advance();
                }
            }
            Tok::Str(s)
        }
    }

    fn read_char(&mut self) -> Tok {
        self.advance(); // consume opening '
        let mut ch = '\0';
        if let Some(c) = self.peek() {
            if c == b'\\' {
                self.advance();
                if let Some(esc) = self.advance() {
                    ch = match esc {
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        b'\\' => '\\',
                        b'\'' => '\'',
                        b'"' => '"',
                        _ => esc as char,
                    };
                }
            } else if c != b'\'' {
                ch = c as char;
                self.advance();
            }
        }
        if self.peek() == Some(b'\'') {
            self.advance();
        }
        Tok::Char(ch)
    }

    fn read_number(&mut self, _line: usize, _col: usize) -> Tok {
        let mut num_str = String::new();
        let mut is_float = false;

        // Check for hex (0x) or binary (0b)
        if self.peek() == Some(b'0') {
            if let Some(second) = self.peek_at(1) {
                if second == b'x' || second == b'X' {
                    num_str.push(self.advance().unwrap() as char);
                    num_str.push(self.advance().unwrap() as char);
                    while let Some(c) = self.peek() {
                        if c.is_ascii_hexdigit() || c == b'_' {
                            if c != b'_' {
                                num_str.push(c as char);
                            }
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if let Some(b'l' | b'L') = self.peek() {
                        self.advance();
                    }
                    let val = i64::from_str_radix(&num_str[2..], 16).unwrap_or(0);
                    return Tok::Int(val);
                } else if second == b'b' || second == b'B' {
                    num_str.push(self.advance().unwrap() as char);
                    num_str.push(self.advance().unwrap() as char);
                    while let Some(c) = self.peek() {
                        if c == b'0' || c == b'1' || c == b'_' {
                            if c != b'_' {
                                num_str.push(c as char);
                            }
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if let Some(b'l' | b'L') = self.peek() {
                        self.advance();
                    }
                    let val = i64::from_str_radix(&num_str[2..], 2).unwrap_or(0);
                    return Tok::Int(val);
                }
            }
        }

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num_str.push(c as char);
                self.advance();
            } else if c == b'_' {
                self.advance();
            } else if c == b'.' && self.peek_at(1).map_or(false, |next| next.is_ascii_digit()) {
                is_float = true;
                num_str.push('.');
                self.advance();
            } else if c == b'e' || c == b'E' {
                is_float = true;
                num_str.push(c as char);
                self.advance();
                if let Some(sign @ (b'+' | b'-')) = self.peek() {
                    num_str.push(sign as char);
                    self.advance();
                }
            } else {
                break;
            }
        }

        if let Some(b'f' | b'F' | b'd' | b'D') = self.peek() {
            is_float = true;
            self.advance();
        } else if let Some(b'l' | b'L') = self.peek() {
            self.advance();
        }

        if is_float {
            let val = num_str.parse::<f64>().unwrap_or(0.0);
            Tok::Float(val)
        } else {
            let val = num_str.parse::<i64>().unwrap_or(0);
            Tok::Int(val)
        }
    }

    fn read_ident_or_keyword(&mut self) -> Tok {
        let mut s = String::new();
        while let Some(b) = self.peek() {
            if is_ident_continue(b) {
                s.push(b as char);
                self.advance();
            } else {
                break;
            }
        }

        match s.as_str() {
            "abstract" => Tok::Abstract,
            "assert" => Tok::Assert,
            "boolean" => Tok::Boolean,
            "break" => Tok::Break,
            "byte" => Tok::Byte,
            "case" => Tok::Case,
            "catch" => Tok::Catch,
            "char" => Tok::CharKw,
            "class" => Tok::Class,
            "const" => Tok::Const,
            "continue" => Tok::Continue,
            "default" => Tok::Default,
            "do" => Tok::Do,
            "double" => Tok::Double,
            "else" => Tok::Else,
            "enum" => Tok::Enum,
            "extends" => Tok::Extends,
            "final" => Tok::Final,
            "finally" => Tok::Finally,
            "float" => Tok::FloatKw,
            "for" => Tok::For,
            "goto" => Tok::Goto,
            "if" => Tok::If,
            "implements" => Tok::Implements,
            "import" => Tok::Import,
            "instanceof" => Tok::Instanceof,
            "int" => Tok::IntKw,
            "interface" => Tok::Interface,
            "long" => Tok::Long,
            "native" => Tok::Native,
            "new" => Tok::New,
            "package" => Tok::Package,
            "private" => Tok::Private,
            "protected" => Tok::Protected,
            "public" => Tok::Public,
            "return" => Tok::Return,
            "short" => Tok::Short,
            "static" => Tok::Static,
            "strictfp" => Tok::Strictfp,
            "super" => Tok::Super,
            "switch" => Tok::Switch,
            "synchronized" => Tok::Synchronized,
            "this" => Tok::This,
            "throw" => Tok::Throw,
            "throws" => Tok::Throws,
            "transient" => Tok::Transient,
            "try" => Tok::Try,
            "void" => Tok::Void,
            "volatile" => Tok::Volatile,
            "while" => Tok::While,

            "record" => Tok::Record,
            "var" => Tok::Var,
            "yield" => Tok::Yield,
            "sealed" => Tok::Sealed,
            "non-sealed" => Tok::NonSealed,
            "permits" => Tok::Permits,

            "true" => Tok::True,
            "false" => Tok::False,
            "null" => Tok::Null,

            _ => Tok::Ident(s),
        }
    }
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

// ─────────────────────────────────────────────────────────────────────────────
// PARSER
// ─────────────────────────────────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Tok {
        self.tokens
            .get(self.pos)
            .map(|t| &t.kind)
            .unwrap_or(&Tok::Eof)
    }

    fn peek_token(&self) -> &Token {
        static EOF_TOKEN: Token = Token {
            kind: Tok::Eof,
            line: 0,
            col: 0,
        };
        self.tokens.get(self.pos).unwrap_or(&EOF_TOKEN)
    }

    fn peek_next(&self) -> &Tok {
        self.tokens
            .get(self.pos + 1)
            .map(|t| &t.kind)
            .unwrap_or(&Tok::Eof)
    }

    fn line(&self) -> usize {
        self.peek_token().line
    }

    fn advance(&mut self) -> Token {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            tok
        } else {
            Token {
                kind: Tok::Eof,
                line: self.line(),
                col: 0,
            }
        }
    }

    fn check(&self, kind: &Tok) -> bool {
        self.peek() == kind
    }

    fn match_tok(&mut self, kind: &Tok) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &Tok) -> Result<Token, String> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(format!(
                "expected {:?}, got {:?} at line {}",
                kind,
                self.peek(),
                self.line()
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.peek() {
            Tok::Ident(_)
            | Tok::Record
            | Tok::Var
            | Tok::Yield
            | Tok::Sealed
            | Tok::NonSealed
            | Tok::Permits => {
                let tok = self.advance();
                match tok.kind {
                    Tok::Ident(s) => Ok(s),
                    Tok::Record => Ok("record".into()),
                    Tok::Var => Ok("var".into()),
                    Tok::Yield => Ok("yield".into()),
                    Tok::Sealed => Ok("sealed".into()),
                    Tok::NonSealed => Ok("non-sealed".into()),
                    Tok::Permits => Ok("permits".into()),
                    _ => unreachable!(),
                }
            }
            other => Err(format!(
                "expected identifier, got {:?} at line {}",
                other,
                self.line()
            )),
        }
    }

    fn parse_qualified_name(&mut self) -> String {
        let mut name = self.expect_ident().unwrap_or_default();
        while self.match_tok(&Tok::Dot) {
            if self.match_tok(&Tok::Star) {
                name.push_str(".*");
                break;
            }
            if let Ok(segment) = self.expect_ident() {
                name.push('.');
                name.push_str(&segment);
            } else {
                break;
            }
        }
        name
    }

    // Skip annotations and collect modifier strings
    fn parse_modifiers(&mut self) -> Vec<String> {
        let mut mods = Vec::new();
        loop {
            // Annotations: @Foo or @Foo(...)
            if self.match_tok(&Tok::At) {
                let _name = self.parse_qualified_name();
                if self.match_tok(&Tok::LParen) {
                    let mut depth = 1;
                    while depth > 0 && !self.check(&Tok::Eof) {
                        match self.advance().kind {
                            Tok::LParen => depth += 1,
                            Tok::RParen => depth -= 1,
                            _ => {}
                        }
                    }
                }
                continue;
            }

            match self.peek() {
                Tok::Public => {
                    self.advance();
                    mods.push("public".into());
                }
                Tok::Private => {
                    self.advance();
                    mods.push("private".into());
                }
                Tok::Protected => {
                    self.advance();
                    mods.push("protected".into());
                }
                Tok::Static => {
                    self.advance();
                    mods.push("static".into());
                }
                Tok::Final => {
                    self.advance();
                    mods.push("final".into());
                }
                Tok::Abstract => {
                    self.advance();
                    mods.push("abstract".into());
                }
                Tok::Synchronized => {
                    self.advance();
                    mods.push("synchronized".into());
                }
                Tok::Native => {
                    self.advance();
                    mods.push("native".into());
                }
                Tok::Transient => {
                    self.advance();
                    mods.push("transient".into());
                }
                Tok::Volatile => {
                    self.advance();
                    mods.push("volatile".into());
                }
                Tok::Strictfp => {
                    self.advance();
                    mods.push("strictfp".into());
                }
                Tok::Default => {
                    self.advance();
                    mods.push("default".into());
                }
                Tok::Sealed => {
                    self.advance();
                    mods.push("sealed".into());
                }
                Tok::NonSealed => {
                    self.advance();
                    mods.push("non-sealed".into());
                }
                _ => break,
            }
        }
        mods
    }

    fn skip_type_parameters(&mut self) -> Vec<String> {
        let mut params = Vec::new();
        if self.match_tok(&Tok::Lt) {
            let mut depth = 1;
            while depth > 0 && !self.check(&Tok::Eof) {
                if let Tok::Ident(ref s) = self.peek() {
                    if depth == 1 {
                        params.push(s.clone());
                    }
                }
                match self.advance().kind {
                    Tok::Lt => depth += 1,
                    Tok::Gt => depth -= 1,
                    Tok::Shr => {
                        if depth >= 2 {
                            depth -= 2;
                        } else {
                            depth = 0;
                        }
                    }
                    Tok::Ushr => {
                        if depth >= 3 {
                            depth -= 3;
                        } else {
                            depth = 0;
                        }
                    }
                    _ => {}
                }
            }
        }
        params
    }

    fn parse_type(&mut self) -> Result<JavaType, String> {
        let base_name = match self.peek() {
            Tok::Byte => {
                self.advance();
                "byte".to_string()
            }
            Tok::Short => {
                self.advance();
                "short".to_string()
            }
            Tok::IntKw => {
                self.advance();
                "int".to_string()
            }
            Tok::Long => {
                self.advance();
                "long".to_string()
            }
            Tok::FloatKw => {
                self.advance();
                "float".to_string()
            }
            Tok::Double => {
                self.advance();
                "double".to_string()
            }
            Tok::CharKw => {
                self.advance();
                "char".to_string()
            }
            Tok::Boolean => {
                self.advance();
                "boolean".to_string()
            }
            Tok::Void => {
                self.advance();
                "void".to_string()
            }
            Tok::Var => {
                self.advance();
                "var".to_string()
            }
            Tok::Ident(_) => self.parse_qualified_name(),
            _ => {
                return Err(format!(
                    "expected type, got {:?} at line {}",
                    self.peek(),
                    self.line()
                ))
            }
        };

        // Type arguments if any, e.g. List<String>
        if self.check(&Tok::Lt) {
            self.skip_type_parameters();
        }

        let mut ty = JavaType::new(&base_name);
        while self.check(&Tok::LBracket) && self.peek_next() == &Tok::RBracket {
            self.advance();
            self.advance();
            ty.array_dims += 1;
        }

        // Ellipsis in varargs: int... args
        if self.match_tok(&Tok::Ellipsis) {
            ty.array_dims += 1;
        }

        Ok(ty)
    }

    pub fn parse_compilation_unit(&mut self) -> Result<CompilationUnit, String> {
        let mut package = None;
        let mut imports = Vec::new();
        let mut types = Vec::new();

        while !self.check(&Tok::Eof) {
            if self.match_tok(&Tok::Semicolon) {
                continue;
            }

            if self.match_tok(&Tok::Package) {
                let pkg = self.parse_qualified_name();
                let _ = self.expect(&Tok::Semicolon);
                package = Some(pkg);
                continue;
            }

            if self.match_tok(&Tok::Import) {
                if self.match_tok(&Tok::Static) {
                    let imp = self.parse_qualified_name();
                    let _ = self.expect(&Tok::Semicolon);
                    imports.push(format!("static {}", imp));
                } else {
                    let imp = self.parse_qualified_name();
                    let _ = self.expect(&Tok::Semicolon);
                    imports.push(imp);
                }
                continue;
            }

            // Top-level type declaration
            if let Some(td) = self.parse_type_decl() {
                types.push(td);
            } else {
                // If not type decl, advance to avoid stalling
                self.advance();
            }
        }

        Ok(CompilationUnit {
            package,
            imports,
            types,
        })
    }

    pub fn parse_type_decl(&mut self) -> Option<TypeDecl> {
        let line = self.line();
        let modifiers = self.parse_modifiers();

        let kind = if self.match_tok(&Tok::Class) {
            TypeKind::Class
        } else if self.match_tok(&Tok::Interface) {
            TypeKind::Interface
        } else if self.match_tok(&Tok::Enum) {
            TypeKind::Enum
        } else if self.match_tok(&Tok::Record) {
            TypeKind::Record
        } else if self.match_tok(&Tok::At) && self.match_tok(&Tok::Interface) {
            TypeKind::Annotation
        } else {
            return None;
        };

        let name = self.expect_ident().unwrap_or_else(|_| "Anonymous".into());
        let type_params = self.skip_type_parameters();

        // Record header components if record
        if kind == TypeKind::Record && self.match_tok(&Tok::LParen) {
            let mut depth = 1;
            while depth > 0 && !self.check(&Tok::Eof) {
                match self.advance().kind {
                    Tok::LParen => depth += 1,
                    Tok::RParen => depth -= 1,
                    _ => {}
                }
            }
        }

        // Extends
        let extends = if self.match_tok(&Tok::Extends) {
            self.parse_type().ok()
        } else {
            None
        };

        // Implements / Permits
        let mut implements = Vec::new();
        if self.match_tok(&Tok::Implements) || self.match_tok(&Tok::Permits) {
            loop {
                if let Ok(t) = self.parse_type() {
                    implements.push(t);
                }
                if !self.match_tok(&Tok::Comma) {
                    break;
                }
            }
        }

        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut constructors = Vec::new();
        let mut nested_types = Vec::new();

        if self.match_tok(&Tok::LBrace) {
            // Enum constants first if enum
            if kind == TypeKind::Enum {
                self.skip_enum_constants();
            }

            while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
                if self.match_tok(&Tok::Semicolon) {
                    continue;
                }

                // Initializer blocks: static { ... } or { ... }
                if self.match_tok(&Tok::Static) && self.check(&Tok::LBrace) {
                    let _ = self.parse_block();
                    continue;
                }
                if self.check(&Tok::LBrace) {
                    let _ = self.parse_block();
                    continue;
                }

                // Nested type declaration
                let member_mods = self.parse_modifiers();
                if self.check(&Tok::Class)
                    || self.check(&Tok::Interface)
                    || self.check(&Tok::Enum)
                    || self.check(&Tok::Record)
                {
                    if let Some(nested) = self.parse_type_decl() {
                        nested_types.push(nested);
                        continue;
                    }
                }

                // Generic method/constructor: <T> ReturnType method(...)
                let _member_tparams = self.skip_type_parameters();

                // Now either Constructor or Method or Field
                let member_line = self.line();
                // Check if constructor: Name matches class name followed by '('
                if let Tok::Ident(ref id) = self.peek() {
                    if id == &name && self.peek_next() == &Tok::LParen {
                        let cname = self.advance();
                        let Tok::Ident(cname) = cname.kind else {
                            unreachable!()
                        };
                        let params = self.parse_param_list();
                        self.skip_throws();
                        let body = if self.check(&Tok::LBrace) {
                            Some(self.parse_block())
                        } else {
                            let _ = self.match_tok(&Tok::Semicolon);
                            None
                        };
                        constructors.push(ConstructorDecl {
                            name: cname,
                            modifiers: member_mods,
                            params,
                            body,
                            line: member_line,
                        });
                        continue;
                    }
                }

                // Parse type for field or method
                let Ok(ty) = self.parse_type() else {
                    // Could not parse type, synchronize to next member
                    self.sync_to_next_member();
                    continue;
                };

                let Ok(member_name) = self.expect_ident() else {
                    self.sync_to_next_member();
                    continue;
                };

                if self.check(&Tok::LParen) {
                    // Method
                    let params = self.parse_param_list();
                    self.skip_throws();
                    let body = if self.check(&Tok::LBrace) {
                        Some(self.parse_block())
                    } else {
                        let _ = self.match_tok(&Tok::Semicolon);
                        None
                    };
                    methods.push(MethodDecl {
                        name: member_name,
                        modifiers: member_mods,
                        return_ty: Some(ty),
                        params,
                        body,
                        line: member_line,
                    });
                } else {
                    // Field
                    let init = if self.match_tok(&Tok::Assign) {
                        self.parse_expr().ok()
                    } else {
                        None
                    };
                    fields.push(FieldDecl {
                        name: member_name,
                        ty: ty.clone(),
                        modifiers: member_mods.clone(),
                        init,
                        line: member_line,
                    });

                    // More fields on same line: int a = 1, b = 2;
                    while self.match_tok(&Tok::Comma) {
                        if let Ok(extra_name) = self.expect_ident() {
                            let extra_init = if self.match_tok(&Tok::Assign) {
                                self.parse_expr().ok()
                            } else {
                                None
                            };
                            fields.push(FieldDecl {
                                name: extra_name,
                                ty: ty.clone(),
                                modifiers: member_mods.clone(),
                                init: extra_init,
                                line: member_line,
                            });
                        }
                    }
                    let _ = self.match_tok(&Tok::Semicolon);
                }
            }
            let _ = self.match_tok(&Tok::RBrace);
        }

        Some(TypeDecl {
            name,
            kind,
            modifiers,
            type_params,
            extends,
            implements,
            fields,
            methods,
            constructors,
            nested_types,
            line,
        })
    }

    fn skip_enum_constants(&mut self) {
        while !self.check(&Tok::Semicolon)
            && !self.check(&Tok::RBrace)
            && !self.check(&Tok::Eof)
        {
            let _ = self.parse_modifiers();
            if let Ok(_name) = self.expect_ident() {
                // Arguments: CONST(1, 2)
                if self.match_tok(&Tok::LParen) {
                    let mut depth = 1;
                    while depth > 0 && !self.check(&Tok::Eof) {
                        match self.advance().kind {
                            Tok::LParen => depth += 1,
                            Tok::RParen => depth -= 1,
                            _ => {}
                        }
                    }
                }
                // Anonymous class body: CONST { void foo() {} }
                if self.check(&Tok::LBrace) {
                    let mut depth = 1;
                    self.advance();
                    while depth > 0 && !self.check(&Tok::Eof) {
                        match self.advance().kind {
                            Tok::LBrace => depth += 1,
                            Tok::RBrace => depth -= 1,
                            _ => {}
                        }
                    }
                }
                if !self.match_tok(&Tok::Comma) {
                    break;
                }
            } else {
                break;
            }
        }
        let _ = self.match_tok(&Tok::Semicolon);
    }

    fn skip_throws(&mut self) {
        if self.match_tok(&Tok::Throws) {
            loop {
                let _ = self.parse_qualified_name();
                if !self.match_tok(&Tok::Comma) {
                    break;
                }
            }
        }
    }

    fn parse_param_list(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        if !self.match_tok(&Tok::LParen) {
            return params;
        }

        while !self.check(&Tok::RParen) && !self.check(&Tok::Eof) {
            let line = self.line();
            let _mods = self.parse_modifiers();
            if let Ok(ty) = self.parse_type() {
                let name = self.expect_ident().unwrap_or_else(|_| "param".into());
                params.push(Param { name, ty, line });
            } else {
                break;
            }
            if !self.match_tok(&Tok::Comma) {
                break;
            }
        }
        let _ = self.expect(&Tok::RParen);
        params
    }

    pub fn parse_block(&mut self) -> Block {
        let line = self.line();
        let _ = self.expect(&Tok::LBrace);
        let mut stmts = Vec::new();

        while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
            if self.match_tok(&Tok::Semicolon) {
                continue;
            }
            match self.parse_stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(_) => {
                    self.sync_to_next_stmt();
                }
            }
        }

        let _ = self.match_tok(&Tok::RBrace);
        Block { stmts, line }
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        let line = self.line();

        if self.check(&Tok::LBrace) {
            return Ok(Stmt::Block(self.parse_block()));
        }

        if self.match_tok(&Tok::If) {
            self.expect(&Tok::LParen)?;
            let cond = self.parse_expr()?;
            self.expect(&Tok::RParen)?;
            let then_branch = Box::new(self.parse_stmt()?);
            let else_branch = if self.match_tok(&Tok::Else) {
                Some(Box::new(self.parse_stmt()?))
            } else {
                None
            };
            return Ok(Stmt::If {
                cond,
                then_branch,
                else_branch,
                line,
            });
        }

        if self.match_tok(&Tok::While) {
            self.expect(&Tok::LParen)?;
            let cond = self.parse_expr()?;
            self.expect(&Tok::RParen)?;
            let body = Box::new(self.parse_stmt()?);
            return Ok(Stmt::While { cond, body, line });
        }

        if self.match_tok(&Tok::Do) {
            let body = Box::new(self.parse_stmt()?);
            self.expect(&Tok::While)?;
            self.expect(&Tok::LParen)?;
            let cond = self.parse_expr()?;
            self.expect(&Tok::RParen)?;
            let _ = self.match_tok(&Tok::Semicolon);
            return Ok(Stmt::DoWhile { body, cond, line });
        }

        if self.match_tok(&Tok::For) {
            return self.parse_for(line);
        }

        if self.match_tok(&Tok::Switch) {
            return self.parse_switch(line);
        }

        if self.match_tok(&Tok::Return) {
            let val = if self.check(&Tok::Semicolon) {
                None
            } else {
                Some(self.parse_expr()?)
            };
            let _ = self.match_tok(&Tok::Semicolon);
            return Ok(Stmt::Return(val, line));
        }

        if self.match_tok(&Tok::Throw) {
            let expr = self.parse_expr()?;
            let _ = self.match_tok(&Tok::Semicolon);
            return Ok(Stmt::Throw(expr, line));
        }

        if self.match_tok(&Tok::Try) {
            return self.parse_try(line);
        }

        if self.match_tok(&Tok::Assert) {
            let cond = self.parse_expr()?;
            let msg = if self.match_tok(&Tok::Colon) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            let _ = self.match_tok(&Tok::Semicolon);
            return Ok(Stmt::Assert { cond, msg, line });
        }

        if self.match_tok(&Tok::Break) {
            let label = self.expect_ident().ok();
            let _ = self.match_tok(&Tok::Semicolon);
            return Ok(Stmt::Break(label, line));
        }

        if self.match_tok(&Tok::Continue) {
            let label = self.expect_ident().ok();
            let _ = self.match_tok(&Tok::Semicolon);
            return Ok(Stmt::Continue(label, line));
        }

        if self.match_tok(&Tok::Synchronized) {
            self.expect(&Tok::LParen)?;
            let lock = self.parse_expr()?;
            self.expect(&Tok::RParen)?;
            let body = self.parse_block();
            return Ok(Stmt::Synchronized { lock, body, line });
        }

        // Variable declaration or expression statement
        if self.is_var_decl_start() {
            let _mods = self.parse_modifiers();
            let ty = self.parse_type()?;
            let mut vars = Vec::new();
            loop {
                let name = self.expect_ident()?;
                let init = if self.match_tok(&Tok::Assign) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                vars.push((name, init));
                if !self.match_tok(&Tok::Comma) {
                    break;
                }
            }
            let _ = self.match_tok(&Tok::Semicolon);
            return Ok(Stmt::VarDecl { ty, vars, line });
        }

        // Expression statement
        let expr = self.parse_expr()?;
        let _ = self.match_tok(&Tok::Semicolon);
        Ok(Stmt::Expr(expr, line))
    }

    fn is_var_decl_start(&self) -> bool {
        match self.peek() {
            Tok::Byte
            | Tok::Short
            | Tok::IntKw
            | Tok::Long
            | Tok::FloatKw
            | Tok::Double
            | Tok::CharKw
            | Tok::Boolean
            | Tok::Var => true,
            Tok::Final => true,
            Tok::Ident(_) => {
                // If followed by type brackets/args or another identifier, it's a type!
                matches!(
                    self.peek_next(),
                    Tok::Lt | Tok::LBracket | Tok::Ident(_) | Tok::Dot
                )
            }
            _ => false,
        }
    }

    fn parse_for(&mut self, line: usize) -> Result<Stmt, String> {
        self.expect(&Tok::LParen)?;

        // Check for enhanced for-each: for (Type x : iterable)
        if self.is_var_decl_start() {
            let save_pos = self.pos;
            let _mods = self.parse_modifiers();
            if let Ok(ty) = self.parse_type() {
                if let Ok(var_name) = self.expect_ident() {
                    if self.match_tok(&Tok::Colon) {
                        let iterable = self.parse_expr()?;
                        self.expect(&Tok::RParen)?;
                        let body = Box::new(self.parse_stmt()?);
                        return Ok(Stmt::ForEach {
                            var_name,
                            var_ty: ty,
                            iterable,
                            body,
                            line,
                        });
                    }
                }
            }
            // Rewind if not enhanced for
            self.pos = save_pos;
        }

        // Standard for loop: for (init; cond; update)
        let mut inits = Vec::new();
        if !self.check(&Tok::Semicolon) {
            if self.is_var_decl_start() {
                let _mods = self.parse_modifiers();
                let ty = self.parse_type()?;
                let mut vars = Vec::new();
                loop {
                    let name = self.expect_ident()?;
                    let init = if self.match_tok(&Tok::Assign) {
                        Some(self.parse_expr()?)
                    } else {
                        None
                    };
                    vars.push((name, init));
                    if !self.match_tok(&Tok::Comma) {
                        break;
                    }
                }
                inits.push(Stmt::VarDecl { ty, vars, line });
            } else {
                loop {
                    let expr = self.parse_expr()?;
                    inits.push(Stmt::Expr(expr, line));
                    if !self.match_tok(&Tok::Comma) {
                        break;
                    }
                }
            }
        }
        self.expect(&Tok::Semicolon)?;

        let cond = if self.check(&Tok::Semicolon) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(&Tok::Semicolon)?;

        let mut updates = Vec::new();
        if !self.check(&Tok::RParen) {
            loop {
                let expr = self.parse_expr()?;
                updates.push(expr);
                if !self.match_tok(&Tok::Comma) {
                    break;
                }
            }
        }
        self.expect(&Tok::RParen)?;

        let body = Box::new(self.parse_stmt()?);
        Ok(Stmt::For {
            init: Some(inits),
            cond,
            update: Some(updates),
            body,
            line,
        })
    }

    fn parse_switch(&mut self, line: usize) -> Result<Stmt, String> {
        self.expect(&Tok::LParen)?;
        let selector = self.parse_expr()?;
        self.expect(&Tok::RParen)?;

        self.expect(&Tok::LBrace)?;
        let mut cases = Vec::new();

        while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
            let case_line = self.line();
            let mut labels = Vec::new();

            if self.match_tok(&Tok::Case) {
                loop {
                    let label = self.parse_expr()?;
                    labels.push(label);
                    if !self.match_tok(&Tok::Comma) {
                        break;
                    }
                }
                if !self.match_tok(&Tok::Colon) {
                    let _ = self.match_tok(&Tok::Arrow);
                }
            } else if self.match_tok(&Tok::Default) {
                if !self.match_tok(&Tok::Colon) {
                    let _ = self.match_tok(&Tok::Arrow);
                }
            } else {
                // Unexpected in switch, skip
                self.advance();
                continue;
            }

            let mut stmts = Vec::new();
            while !self.check(&Tok::Case)
                && !self.check(&Tok::Default)
                && !self.check(&Tok::RBrace)
                && !self.check(&Tok::Eof)
            {
                if let Ok(stmt) = self.parse_stmt() {
                    stmts.push(stmt);
                } else {
                    self.advance();
                }
            }

            cases.push(SwitchCase {
                labels,
                stmts,
                line: case_line,
            });
        }

        let _ = self.match_tok(&Tok::RBrace);
        Ok(Stmt::Switch {
            selector,
            cases,
            line,
        })
    }

    fn parse_try(&mut self, line: usize) -> Result<Stmt, String> {
        let mut resources = Vec::new();
        // Try-with-resources: try (Resource r = ...)
        if self.match_tok(&Tok::LParen) {
            while !self.check(&Tok::RParen) && !self.check(&Tok::Eof) {
                let res_line = self.line();
                let ty = self.parse_type().unwrap_or_else(|_| JavaType::new("AutoCloseable"));
                let name = self.expect_ident().unwrap_or_else(|_| "res".into());
                let init = if self.match_tok(&Tok::Assign) {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                resources.push(Stmt::VarDecl {
                    ty,
                    vars: vec![(name, init)],
                    line: res_line,
                });
                if !self.match_tok(&Tok::Semicolon) {
                    break;
                }
            }
            let _ = self.expect(&Tok::RParen);
        }

        let body = self.parse_block();
        let mut catches = Vec::new();

        while self.match_tok(&Tok::Catch) {
            let catch_line = self.line();
            self.expect(&Tok::LParen)?;
            let mut param_types = Vec::new();
            loop {
                if let Ok(ty) = self.parse_type() {
                    param_types.push(ty.name);
                }
                if !self.match_tok(&Tok::Pipe) {
                    break;
                }
            }
            let param_name = self.expect_ident().unwrap_or_else(|_| "e".into());
            self.expect(&Tok::RParen)?;
            let catch_body = self.parse_block();
            catches.push(CatchClause {
                param_name,
                param_types,
                body: catch_body,
                line: catch_line,
            });
        }

        let finally_body = if self.match_tok(&Tok::Finally) {
            Some(self.parse_block())
        } else {
            None
        };

        Ok(Stmt::Try {
            resources,
            body,
            catches,
            finally_body,
            line,
        })
    }

    // ─── EXPRESSIONS (PRATT PARSER) ──────────────────────────────────────────

    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, String> {
        let left = self.parse_conditional()?;

        let op = match self.peek() {
            Tok::Assign => Some(AssignOp::Assign),
            Tok::PlusAssign => Some(AssignOp::AddAssign),
            Tok::MinusAssign => Some(AssignOp::SubAssign),
            Tok::MulAssign => Some(AssignOp::MulAssign),
            Tok::DivAssign => Some(AssignOp::DivAssign),
            Tok::RemAssign => Some(AssignOp::RemAssign),
            Tok::BitAndAssign => Some(AssignOp::BitAndAssign),
            Tok::BitOrAssign => Some(AssignOp::BitOrAssign),
            Tok::BitXorAssign => Some(AssignOp::BitXorAssign),
            Tok::ShlAssign => Some(AssignOp::ShlAssign),
            Tok::ShrAssign => Some(AssignOp::ShrAssign),
            Tok::UshrAssign => Some(AssignOp::UshrAssign),
            _ => None,
        };

        if let Some(assign_op) = op {
            let line = self.line();
            self.advance();
            let right = self.parse_assignment()?;
            Ok(Expr::Assign {
                op: assign_op,
                target: Box::new(left),
                value: Box::new(right),
                line,
            })
        } else {
            Ok(left)
        }
    }

    fn parse_conditional(&mut self) -> Result<Expr, String> {
        let cond = self.parse_logical_or()?;
        if self.match_tok(&Tok::Question) {
            let line = self.line();
            let then_expr = self.parse_expr()?;
            self.expect(&Tok::Colon)?;
            let else_expr = self.parse_conditional()?;
            Ok(Expr::Ternary {
                cond: Box::new(cond),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
                line,
            })
        } else {
            Ok(cond)
        }
    }

    fn parse_logical_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_logical_and()?;
        while self.match_tok(&Tok::Or) {
            let line = self.line();
            let right = self.parse_logical_and()?;
            left = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_or()?;
        while self.match_tok(&Tok::And) {
            let line = self.line();
            let right = self.parse_bitwise_or()?;
            left = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_xor()?;
        while self.match_tok(&Tok::Pipe) {
            let line = self.line();
            let right = self.parse_bitwise_xor()?;
            left = Expr::Binary {
                op: BinaryOp::BitOr,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitwise_and()?;
        while self.match_tok(&Tok::Caret) {
            let line = self.line();
            let right = self.parse_bitwise_and()?;
            left = Expr::Binary {
                op: BinaryOp::BitXor,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;
        while self.match_tok(&Tok::Amp) {
            let line = self.line();
            let right = self.parse_equality()?;
            left = Expr::Binary {
                op: BinaryOp::BitAnd,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_relational()?;
        loop {
            let op = if self.match_tok(&Tok::Eq) {
                BinaryOp::Eq
            } else if self.match_tok(&Tok::Ne) {
                BinaryOp::Ne
            } else {
                break;
            };
            let line = self.line();
            let right = self.parse_relational()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_shift()?;
        loop {
            if self.match_tok(&Tok::Instanceof) {
                let line = self.line();
                let check_ty = self.parse_type()?;
                // Pattern variable: instanceof String s
                let pattern_var = self.expect_ident().ok();
                left = Expr::InstanceOf {
                    expr: Box::new(left),
                    check_ty,
                    pattern_var,
                    line,
                };
                continue;
            }

            let op = if self.match_tok(&Tok::Lt) {
                BinaryOp::Lt
            } else if self.match_tok(&Tok::Le) {
                BinaryOp::Le
            } else if self.match_tok(&Tok::Gt) {
                BinaryOp::Gt
            } else if self.match_tok(&Tok::Ge) {
                BinaryOp::Ge
            } else {
                break;
            };
            let line = self.line();
            let right = self.parse_shift()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;
        loop {
            let op = if self.match_tok(&Tok::Shl) {
                BinaryOp::Shl
            } else if self.match_tok(&Tok::Shr) {
                BinaryOp::Shr
            } else if self.match_tok(&Tok::Ushr) {
                BinaryOp::Ushr
            } else {
                break;
            };
            let line = self.line();
            let right = self.parse_additive()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = if self.match_tok(&Tok::Plus) {
                BinaryOp::Add
            } else if self.match_tok(&Tok::Minus) {
                BinaryOp::Sub
            } else {
                break;
            };
            let line = self.line();
            let right = self.parse_multiplicative()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = if self.match_tok(&Tok::Star) {
                BinaryOp::Mul
            } else if self.match_tok(&Tok::Slash) {
                BinaryOp::Div
            } else if self.match_tok(&Tok::Percent) {
                BinaryOp::Rem
            } else {
                break;
            };
            let line = self.line();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        let line = self.line();

        if self.match_tok(&Tok::PlusPlus) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::PreInc,
                expr: Box::new(expr),
                line,
            });
        }
        if self.match_tok(&Tok::MinusMinus) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::PreDec,
                expr: Box::new(expr),
                line,
            });
        }
        if self.match_tok(&Tok::Plus) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Plus,
                expr: Box::new(expr),
                line,
            });
        }
        if self.match_tok(&Tok::Minus) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Minus,
                expr: Box::new(expr),
                line,
            });
        }
        if self.match_tok(&Tok::Not) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
                line,
            });
        }
        if self.match_tok(&Tok::BitNot) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::BitNot,
                expr: Box::new(expr),
                line,
            });
        }

        // Check for cast: (TargetType) expr
        if self.check(&Tok::LParen) && self.is_cast_lookahead() {
            self.advance(); // consume '('
            let target_ty = self.parse_type()?;
            self.expect(&Tok::RParen)?;
            let expr = self.parse_unary()?;
            return Ok(Expr::Cast {
                target_ty,
                expr: Box::new(expr),
                line,
            });
        }

        self.parse_postfix()
    }

    fn is_cast_lookahead(&self) -> bool {
        // Look ahead past '(' to see if it's a type name followed by ')' and then expression start
        let next = self.peek_next();
        match next {
            Tok::Byte
            | Tok::Short
            | Tok::IntKw
            | Tok::Long
            | Tok::FloatKw
            | Tok::Double
            | Tok::CharKw
            | Tok::Boolean => true,
            Tok::Ident(_) => {
                // Peek ahead for closing RParen
                let mut p = self.pos + 1;
                while p < self.tokens.len() {
                    match &self.tokens[p].kind {
                        Tok::RParen => {
                            if p + 1 < self.tokens.len() {
                                return matches!(
                                    &self.tokens[p + 1].kind,
                                    Tok::Ident(_)
                                        | Tok::Int(_)
                                        | Tok::Float(_)
                                        | Tok::Str(_)
                                        | Tok::Char(_)
                                        | Tok::True
                                        | Tok::False
                                        | Tok::Null
                                        | Tok::This
                                        | Tok::Super
                                        | Tok::New
                                        | Tok::LParen
                                        | Tok::Not
                                        | Tok::BitNot
                                        | Tok::PlusPlus
                                        | Tok::MinusMinus
                                );
                            }
                            return false;
                        }
                        Tok::Ident(_) | Tok::Dot | Tok::Lt | Tok::Gt | Tok::LBracket | Tok::RBracket | Tok::Question | Tok::Comma => {
                            p += 1;
                        }
                        _ => return false,
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            let line = self.line();
            if self.match_tok(&Tok::PlusPlus) {
                expr = Expr::Unary {
                    op: UnaryOp::PostInc,
                    expr: Box::new(expr),
                    line,
                };
            } else if self.match_tok(&Tok::MinusMinus) {
                expr = Expr::Unary {
                    op: UnaryOp::PostDec,
                    expr: Box::new(expr),
                    line,
                };
            } else if self.match_tok(&Tok::LBracket) {
                let index = self.parse_expr()?;
                self.expect(&Tok::RBracket)?;
                expr = Expr::ArrayAccess {
                    array: Box::new(expr),
                    index: Box::new(index),
                    line,
                };
            } else if self.match_tok(&Tok::Dot) {
                // Could be .class or .length or .method(args) or .field
                if self.match_tok(&Tok::Class) {
                    expr = Expr::ClassLiteral(format!("{:?}", expr), line);
                } else if self.match_tok(&Tok::Super) {
                    expr = Expr::Super(line);
                } else {
                    // Optional generic type args: obj.<T>method(...)
                    let _ = self.skip_type_parameters();
                    let member = self.expect_ident()?;
                    if self.check(&Tok::LParen) {
                        let args = self.parse_argument_list()?;
                        expr = Expr::MethodCall {
                            target: Some(Box::new(expr)),
                            method: member,
                            args,
                            line,
                        };
                    } else {
                        expr = Expr::FieldAccess {
                            target: Box::new(expr),
                            field: member,
                            line,
                        };
                    }
                }
            } else if self.match_tok(&Tok::ColonColon) {
                // Method reference: Type::method
                let member = self.expect_ident()?;
                expr = Expr::MethodRef {
                    target: Box::new(expr),
                    method: member,
                    line,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_argument_list(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        self.expect(&Tok::LParen)?;
        while !self.check(&Tok::RParen) && !self.check(&Tok::Eof) {
            args.push(self.parse_expr()?);
            if !self.match_tok(&Tok::Comma) {
                break;
            }
        }
        self.expect(&Tok::RParen)?;
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let line = self.line();

        match self.peek() {
            Tok::Null => {
                self.advance();
                Ok(Expr::Null(line))
            }
            Tok::True => {
                self.advance();
                Ok(Expr::Bool(true, line))
            }
            Tok::False => {
                self.advance();
                Ok(Expr::Bool(false, line))
            }
            Tok::Int(val) => {
                let v = *val;
                self.advance();
                Ok(Expr::Int(v, line))
            }
            Tok::Float(val) => {
                let v = *val;
                self.advance();
                Ok(Expr::Float(v, line))
            }
            Tok::Str(val) => {
                let s = val.clone();
                self.advance();
                Ok(Expr::StringLit(s, line))
            }
            Tok::Char(val) => {
                let c = *val;
                self.advance();
                Ok(Expr::CharLit(c, line))
            }
            Tok::This => {
                self.advance();
                Ok(Expr::This(line))
            }
            Tok::Super => {
                self.advance();
                Ok(Expr::Super(line))
            }
            Tok::New => self.parse_new(line),

            Tok::LParen => {
                // Parenthesized expression OR lambda: (a, b) -> ...
                let save_pos = self.pos;
                self.advance(); // consume '('

                // Try lambda parameter list
                if self.is_lambda_params() {
                    let mut params = Vec::new();
                    while !self.check(&Tok::RParen) && !self.check(&Tok::Eof) {
                        let _ = self.parse_modifiers();
                        let _ = self.parse_type();
                        if let Ok(pname) = self.expect_ident() {
                            params.push(pname);
                        }
                        if !self.match_tok(&Tok::Comma) {
                            break;
                        }
                    }
                    if self.match_tok(&Tok::RParen) && self.match_tok(&Tok::Arrow) {
                        if self.check(&Tok::LBrace) {
                            let block = self.parse_block();
                            return Ok(Expr::Lambda {
                                params,
                                body_expr: None,
                                body_block: Some(block),
                                line,
                            });
                        } else {
                            let body_expr = self.parse_expr()?;
                            return Ok(Expr::Lambda {
                                params,
                                body_expr: Some(Box::new(body_expr)),
                                body_block: None,
                                line,
                            });
                        }
                    }
                }

                // Regular parenthesized expression
                self.pos = save_pos;
                self.advance(); // consume '('
                let inner = self.parse_expr()?;
                self.expect(&Tok::RParen)?;
                Ok(inner)
            }

            Tok::Ident(_) => {
                let name = self.expect_ident()?;
                // Single identifier lambda: x -> ...
                if self.match_tok(&Tok::Arrow) {
                    if self.check(&Tok::LBrace) {
                        let block = self.parse_block();
                        return Ok(Expr::Lambda {
                            params: vec![name],
                            body_expr: None,
                            body_block: Some(block),
                            line,
                        });
                    } else {
                        let body_expr = self.parse_expr()?;
                        return Ok(Expr::Lambda {
                            params: vec![name],
                            body_expr: Some(Box::new(body_expr)),
                            body_block: None,
                            line,
                        });
                    }
                }

                // Direct method call: foo(...)
                if self.check(&Tok::LParen) {
                    let args = self.parse_argument_list()?;
                    Ok(Expr::MethodCall {
                        target: None,
                        method: name,
                        args,
                        line,
                    })
                } else {
                    Ok(Expr::Ident(name, line))
                }
            }

            other => Err(format!(
                "unexpected token {:?} in expression at line {}",
                other, line
            )),
        }
    }

    fn is_lambda_params(&self) -> bool {
        // Quick scan to see if closing ')' is followed by '->'
        let mut depth = 1;
        let mut p = self.pos;
        while p < self.tokens.len() {
            match &self.tokens[p].kind {
                Tok::LParen => depth += 1,
                Tok::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        return p + 1 < self.tokens.len()
                            && self.tokens[p + 1].kind == Tok::Arrow;
                    }
                }
                _ => {}
            }
            p += 1;
        }
        false
    }

    fn parse_new(&mut self, line: usize) -> Result<Expr, String> {
        self.expect(&Tok::New)?;
        let ty = self.parse_type()?;

        // Array creation: new int[10] or new int[] { 1, 2, 3 }
        if ty.array_dims > 0 || self.check(&Tok::LBracket) {
            let mut dims = Vec::new();
            while self.match_tok(&Tok::LBracket) {
                if !self.check(&Tok::RBracket) {
                    let dim = self.parse_expr()?;
                    dims.push(dim);
                }
                let _ = self.match_tok(&Tok::RBracket);
            }

            let initializer = if self.match_tok(&Tok::LBrace) {
                let mut inits = Vec::new();
                while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
                    inits.push(self.parse_expr()?);
                    if !self.match_tok(&Tok::Comma) {
                        break;
                    }
                }
                let _ = self.match_tok(&Tok::RBrace);
                Some(inits)
            } else {
                None
            };

            return Ok(Expr::NewArray {
                element_ty: ty,
                dims,
                initializer,
                line,
            });
        }

        // Object creation: new Foo(args) [ { anonymous class body } ]
        let args = self.parse_argument_list()?;
        if self.check(&Tok::LBrace) {
            // Skip anonymous class body
            let _ = self.parse_block();
        }

        Ok(Expr::NewObject { ty, args, line })
    }

    // ─── ERROR RECOVERY HELPERS ──────────────────────────────────────────────

    fn sync_to_next_member(&mut self) {
        let mut depth = 0;
        while !self.check(&Tok::Eof) {
            match self.peek() {
                Tok::LBrace => {
                    depth += 1;
                    self.advance();
                }
                Tok::RBrace => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    self.advance();
                    if depth == 0 {
                        break;
                    }
                }
                Tok::Semicolon => {
                    if depth == 0 {
                        self.advance();
                        break;
                    }
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn sync_to_next_stmt(&mut self) {
        let mut depth = 0;
        while !self.check(&Tok::Eof) {
            match self.peek() {
                Tok::LBrace => {
                    depth += 1;
                    self.advance();
                }
                Tok::RBrace => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    self.advance();
                    if depth == 0 {
                        break;
                    }
                }
                Tok::Semicolon => {
                    if depth == 0 {
                        self.advance();
                        break;
                    }
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }
    }
}

pub fn parse(src: &str) -> Result<CompilationUnit, String> {
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse_compilation_unit()
}
