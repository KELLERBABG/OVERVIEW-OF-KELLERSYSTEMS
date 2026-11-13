#![allow(dead_code, unused_variables, unused_imports, private_interfaces)]

use crate::go::ast::*;

/// Token kinds for the Go lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    Rune(char),
    // Punctuation
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Semicolon, Comma, Colon, Dot, Ellipsis,
    Plus, Minus, Star, Slash, Percent,
    Amp, Pipe, Caret, Shl, Shr, AmpCaret,
    PlusPlus, MinusMinus,
    Assign, PlusAssign, MinusAssign, StarAssign, SlashAssign, PercentAssign,
    AmpAssign, PipeAssign, CaretAssign, ShlAssign, ShrAssign, AmpCaretAssign,
    Define, // :=
    Eq, Neq, Lt, Gt, Le, Ge,
    AndAnd, OrOr,
    Arrow, // <-
    // Keywords
    Package, Import, Func, Var, Const, Type, Struct, Interface, Map, Chan,
    If, Else, For, Range, Switch, Case, Default, Select, Return,
    Go, Defer, Break, Continue, Goto, Fallthrough,
    Make, New, Close, Len, Cap, Panic,
    Nil, True, False,
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
        Self { src: src.as_bytes(), pos: 0, line: 1, col: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.src.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let b = self.src.get(self.pos).copied();
        if let Some(byte) = b {
            self.pos += 1;
            self.col += 1;
            if byte == b'\n' {
                self.line += 1;
                self.col = 0;
            }
        }
        b
    }

    fn skip_ws_and_comments(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
                self.advance();
            } else if b == b'/' && self.peek_at(1) == Some(b'/') {
                while let Some(c) = self.peek() {
                    if c == b'\n' { break; }
                    self.advance();
                }
            } else if b == b'/' && self.peek_at(1) == Some(b'*') {
                self.advance(); self.advance();
                while let Some(c) = self.peek() {
                    if c == b'*' && self.peek_at(1) == Some(b'/') {
                        self.advance(); self.advance();
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_ws_and_comments();
        let line = self.line;
        let col = self.col;
        let b = match self.peek() {
            Some(b) => b,
            None => return Token { kind: Tok::Eof, line, col },
        };

        // Identifiers & Keywords
        if b.is_ascii_alphabetic() || b == b'_' || b >= 0x80 {
            let start = self.pos;
            while let Some(c) = self.peek() {
                if c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80 {
                    self.advance();
                } else {
                    break;
                }
            }
            let word = std::str::from_utf8(&self.src[start..self.pos]).unwrap_or("");
            let kind = match word {
                "package" => Tok::Package,
                "import" => Tok::Import,
                "func" => Tok::Func,
                "var" => Tok::Var,
                "const" => Tok::Const,
                "type" => Tok::Type,
                "struct" => Tok::Struct,
                "interface" => Tok::Interface,
                "map" => Tok::Map,
                "chan" => Tok::Chan,
                "if" => Tok::If,
                "else" => Tok::Else,
                "for" => Tok::For,
                "range" => Tok::Range,
                "switch" => Tok::Switch,
                "case" => Tok::Case,
                "default" => Tok::Default,
                "select" => Tok::Select,
                "return" => Tok::Return,
                "go" => Tok::Go,
                "defer" => Tok::Defer,
                "break" => Tok::Break,
                "continue" => Tok::Continue,
                "goto" => Tok::Goto,
                "fallthrough" => Tok::Fallthrough,
                "make" => Tok::Make,
                "new" => Tok::New,
                "close" => Tok::Close,
                "len" => Tok::Len,
                "cap" => Tok::Cap,
                "panic" => Tok::Panic,
                "nil" => Tok::Nil,
                "true" => Tok::True,
                "false" => Tok::False,
                _ => Tok::Ident(word.to_string()),
            };
            return Token { kind, line, col };
        }

        // Numbers (decimal, hex, binary, octal, float)
        if b.is_ascii_digit() || (b == b'.' && self.peek_at(1).map_or(false, |c| c.is_ascii_digit())) {
            let start = self.pos;
            if b == b'0' && (self.peek_at(1) == Some(b'x') || self.peek_at(1) == Some(b'X')) {
                self.advance(); self.advance();
                while let Some(c) = self.peek() {
                    if c.is_ascii_hexdigit() || c == b'_' { self.advance(); } else { break; }
                }
                let raw = std::str::from_utf8(&self.src[start + 2..self.pos]).unwrap_or("0").replace('_', "");
                let val = i64::from_str_radix(&raw, 16).unwrap_or(0);
                return Token { kind: Tok::Int(val), line, col };
            } else if b == b'0' && (self.peek_at(1) == Some(b'b') || self.peek_at(1) == Some(b'B')) {
                self.advance(); self.advance();
                while let Some(c) = self.peek() {
                    if c == b'0' || c == b'1' || c == b'_' { self.advance(); } else { break; }
                }
                let raw = std::str::from_utf8(&self.src[start + 2..self.pos]).unwrap_or("0").replace('_', "");
                let val = i64::from_str_radix(&raw, 2).unwrap_or(0);
                return Token { kind: Tok::Int(val), line, col };
            } else if b == b'0' && (self.peek_at(1) == Some(b'o') || self.peek_at(1) == Some(b'O')) {
                self.advance(); self.advance();
                while let Some(c) = self.peek() {
                    if (b'0'..=b'7').contains(&c) || c == b'_' { self.advance(); } else { break; }
                }
                let raw = std::str::from_utf8(&self.src[start + 2..self.pos]).unwrap_or("0").replace('_', "");
                let val = i64::from_str_radix(&raw, 8).unwrap_or(0);
                return Token { kind: Tok::Int(val), line, col };
            }

            let mut is_float = false;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() || c == b'_' {
                    self.advance();
                } else if c == b'.' && !is_float && self.peek_at(1) != Some(b'.') {
                    is_float = true;
                    self.advance();
                } else if (c == b'e' || c == b'E') && !is_float {
                    is_float = true;
                    self.advance();
                    if self.peek() == Some(b'+') || self.peek() == Some(b'-') {
                        self.advance();
                    }
                } else if c == b'i' {
                    is_float = true;
                    self.advance();
                    break;
                } else {
                    break;
                }
            }
            let raw = std::str::from_utf8(&self.src[start..self.pos]).unwrap_or("0").replace(['_', 'i'], "");
            let kind = if is_float {
                Tok::Float(raw.parse().unwrap_or(0.0))
            } else {
                Tok::Int(raw.parse().unwrap_or(0))
            };
            return Token { kind, line, col };
        }

        // Strings
        if b == b'"' {
            self.advance();
            let mut s = String::new();
            while let Some(c) = self.peek() {
                if c == b'"' { self.advance(); break; }
                if c == b'\\' {
                    self.advance();
                    match self.advance() {
                        Some(b'n') => s.push('\n'),
                        Some(b't') => s.push('\t'),
                        Some(b'r') => s.push('\r'),
                        Some(b'\\') => s.push('\\'),
                        Some(b'"') => s.push('"'),
                        Some(other) => { s.push('\\'); s.push(other as char); }
                        None => break,
                    }
                } else {
                    s.push(c as char);
                    self.advance();
                }
            }
            return Token { kind: Tok::Str(s), line, col };
        }

        // Raw strings `...`
        if b == b'`' {
            self.advance();
            let mut s = String::new();
            while let Some(c) = self.peek() {
                if c == b'`' { self.advance(); break; }
                s.push(c as char);
                self.advance();
            }
            return Token { kind: Tok::Str(s), line, col };
        }

        // Rune literals '...'
        if b == b'\'' {
            self.advance();
            let mut ch = ' ';
            if let Some(c) = self.peek() {
                if c == b'\\' {
                    self.advance();
                    ch = match self.advance() {
                        Some(b'n') => '\n',
                        Some(b't') => '\t',
                        Some(b'r') => '\r',
                        Some(b'\\') => '\\',
                        Some(b'\'') => '\'',
                        Some(b'0') => '\0',
                        Some(o) => o as char,
                        None => ' ',
                    };
                } else {
                    ch = c as char;
                    self.advance();
                }
            }
            if self.peek() == Some(b'\'') { self.advance(); }
            return Token { kind: Tok::Rune(ch), line, col };
        }

        // 3-character operators
        if let (Some(a), Some(b2), Some(c3)) = (self.peek(), self.peek_at(1), self.peek_at(2)) {
            match (a, b2, c3) {
                (b'.', b'.', b'.') => { self.advance(); self.advance(); self.advance(); return Token { kind: Tok::Ellipsis, line, col }; }
                (b'&', b'^', b'=') => { self.advance(); self.advance(); self.advance(); return Token { kind: Tok::AmpCaretAssign, line, col }; }
                (b'<', b'<', b'=') => { self.advance(); self.advance(); self.advance(); return Token { kind: Tok::ShlAssign, line, col }; }
                (b'>', b'>', b'=') => { self.advance(); self.advance(); self.advance(); return Token { kind: Tok::ShrAssign, line, col }; }
                _ => {}
            }
        }

        // 2-character operators
        if let (Some(a), Some(b2)) = (self.peek(), self.peek_at(1)) {
            let matched = match (a, b2) {
                (b':', b'=') => Some(Tok::Define),
                (b'+', b'+') => Some(Tok::PlusPlus),
                (b'-', b'-') => Some(Tok::MinusMinus),
                (b'+', b'=') => Some(Tok::PlusAssign),
                (b'-', b'=') => Some(Tok::MinusAssign),
                (b'*', b'=') => Some(Tok::StarAssign),
                (b'/', b'=') => Some(Tok::SlashAssign),
                (b'%', b'=') => Some(Tok::PercentAssign),
                (b'&', b'=') => Some(Tok::AmpAssign),
                (b'|', b'=') => Some(Tok::PipeAssign),
                (b'^', b'=') => Some(Tok::CaretAssign),
                (b'&', b'^') => Some(Tok::AmpCaret),
                (b'<', b'<') => Some(Tok::Shl),
                (b'>', b'>') => Some(Tok::Shr),
                (b'<', b'-') => Some(Tok::Arrow),
                (b'=', b'=') => Some(Tok::Eq),
                (b'!', b'=') => Some(Tok::Neq),
                (b'<', b'=') => Some(Tok::Le),
                (b'>', b'=') => Some(Tok::Ge),
                (b'&', b'&') => Some(Tok::AndAnd),
                (b'|', b'|') => Some(Tok::OrOr),
                _ => None,
            };
            if let Some(kind) = matched {
                self.advance(); self.advance();
                return Token { kind, line, col };
            }
        }

        // 1-character tokens
        self.advance();
        let kind = match b {
            b'(' => Tok::LParen,
            b')' => Tok::RParen,
            b'{' => Tok::LBrace,
            b'}' => Tok::RBrace,
            b'[' => Tok::LBracket,
            b']' => Tok::RBracket,
            b';' => Tok::Semicolon,
            b',' => Tok::Comma,
            b':' => Tok::Colon,
            b'.' => Tok::Dot,
            b'+' => Tok::Plus,
            b'-' => Tok::Minus,
            b'*' => Tok::Star,
            b'/' => Tok::Slash,
            b'%' => Tok::Percent,
            b'&' => Tok::Amp,
            b'|' => Tok::Pipe,
            b'^' => Tok::Caret,
            b'<' => Tok::Lt,
            b'>' => Tok::Gt,
            b'=' => Tok::Assign,
            b'!' => Tok::Neq, // unary not or part of neq
            _ => Tok::Ident(String::from(b as char)),
        };
        Token { kind, line, col }
    }
}

/// Parse Go source text into a Package AST.
pub fn parse(source: &str) -> Result<Package, String> {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    loop {
        let tok = lexer.next_token();
        if tok.kind == Tok::Eof { break; }
        tokens.push(tok);
    }

    let mut parser = Parser { tokens, pos: 0, prev_line: 1 };
    parser.parse_package()
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    prev_line: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        self.tokens.get(self.pos).map(|t| &t.kind).unwrap_or(&Tok::Eof)
    }

    fn peek_line(&self) -> usize {
        self.tokens.get(self.pos).map(|t| t.line).unwrap_or(0)
    }

    fn advance(&mut self) -> Token {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.prev_line = tok.line;
            self.pos += 1;
            tok
        } else {
            Token { kind: Tok::Eof, line: 0, col: 0 }
        }
    }

    fn expect(&mut self, kind: &Tok) -> Result<(), String> {
        if self.peek() == kind {
            self.advance();
            Ok(())
        } else {
            Err(format!("line {}: expected {:?}, got {:?}", self.peek_line(), kind, self.peek()))
        }
    }

    fn eat(&mut self, kind: &Tok) -> bool {
        if self.peek() == kind {
            self.advance();
            true
        } else {
            false
        }
    }

    fn parse_package(&mut self) -> Result<Package, String> {
        let mut imports = Vec::new();
        let mut functions = Vec::new();
        let mut pkg_name = String::from("main");

        while self.pos < self.tokens.len() && !matches!(self.peek(), Tok::Eof) {
            let start_pos = self.pos;
            match self.peek() {
                Tok::Package => {
                    self.advance();
                    if let Tok::Ident(n) = self.peek().clone() {
                        pkg_name = n;
                        self.advance();
                    }
                    self.eat(&Tok::Semicolon);
                }
                Tok::Import => {
                    self.advance();
                    if self.eat(&Tok::LParen) {
                        while !self.eat(&Tok::RParen) && !matches!(self.peek(), Tok::Eof) {
                            let inner_start = self.pos;
                            if let Tok::Str(ref path) = self.peek().clone() {
                                imports.push(Import { path: path.clone(), alias: None });
                                self.advance();
                            } else if let Tok::Ident(ref alias) = self.peek().clone() {
                                let a = alias.clone();
                                self.advance();
                                if let Tok::Str(ref path) = self.peek().clone() {
                                    imports.push(Import { path: path.clone(), alias: Some(a) });
                                    self.advance();
                                }
                            } else if self.eat(&Tok::Dot) {
                                if let Tok::Str(ref path) = self.peek().clone() {
                                    imports.push(Import { path: path.clone(), alias: Some(".".into()) });
                                    self.advance();
                                }
                            } else {
                                self.advance();
                            }
                            self.eat(&Tok::Semicolon);
                            if self.pos <= inner_start && !matches!(self.peek(), Tok::RParen | Tok::Eof) {
                                self.advance();
                            }
                        }
                    } else if let Tok::Str(ref path) = self.peek().clone() {
                        imports.push(Import { path: path.clone(), alias: None });
                        self.advance();
                    } else if let Tok::Ident(ref alias) = self.peek().clone() {
                        let a = alias.clone();
                        self.advance();
                        if let Tok::Str(ref path) = self.peek().clone() {
                            imports.push(Import { path: path.clone(), alias: Some(a) });
                            self.advance();
                        }
                    }
                    self.eat(&Tok::Semicolon);
                }
                Tok::Func => {
                    let fn_start = self.pos;
                    match self.parse_function() {
                        Ok(f) => functions.push(f),
                        Err(_) => {
                            self.sync_top_level();
                            if self.pos <= fn_start && !matches!(self.peek(), Tok::Eof) {
                                self.advance();
                            }
                        }
                    }
                }
                Tok::Const | Tok::Var => {
                    self.advance();
                    if self.eat(&Tok::LParen) {
                        self.skip_balanced(Tok::LParen, Tok::RParen);
                    } else {
                        while !matches!(self.peek(), Tok::Semicolon | Tok::Func | Tok::Const | Tok::Var | Tok::Type | Tok::Eof) {
                            if matches!(self.peek(), Tok::LBrace) {
                                self.skip_balanced(Tok::LBrace, Tok::RBrace);
                                break;
                            } else {
                                self.advance();
                            }
                        }
                        self.eat(&Tok::Semicolon);
                    }
                }
                Tok::Type => {
                    self.advance();
                    if self.eat(&Tok::LParen) {
                        self.skip_balanced(Tok::LParen, Tok::RParen);
                    } else {
                        while !matches!(self.peek(), Tok::Semicolon | Tok::Func | Tok::Const | Tok::Var | Tok::Type | Tok::Eof) {
                            if matches!(self.peek(), Tok::LBrace) {
                                self.skip_balanced(Tok::LBrace, Tok::RBrace);
                                break;
                            } else {
                                self.advance();
                            }
                        }
                        self.eat(&Tok::Semicolon);
                    }
                }
                _ => {
                    self.advance();
                }
            }
            if self.pos <= start_pos && !matches!(self.peek(), Tok::Eof) {
                self.advance();
            }
        }

        Ok(Package { name: pkg_name, imports, functions })
    }

    fn sync_top_level(&mut self) {
        if matches!(self.peek(), Tok::Func | Tok::Const | Tok::Var | Tok::Type | Tok::Package) {
            self.advance();
        }
        while !matches!(self.peek(), Tok::Func | Tok::Const | Tok::Var | Tok::Type | Tok::Package | Tok::Eof) {
            if matches!(self.peek(), Tok::LBrace) {
                self.skip_balanced(Tok::LBrace, Tok::RBrace);
            } else {
                self.advance();
            }
        }
    }

    fn skip_balanced(&mut self, open: Tok, close: Tok) {
        let mut depth = 1;
        if self.peek() == &open {
            self.advance();
        }
        while depth > 0 && !matches!(self.peek(), Tok::Eof) {
            let tok = self.advance();
            if tok.kind == open {
                depth += 1;
            } else if tok.kind == close {
                depth -= 1;
            }
        }
    }

    fn parse_function(&mut self) -> Result<Function, String> {
        let line = self.peek_line();
        self.expect(&Tok::Func)?;

        // Optional receiver: func (r *Receiver) MethodName(...)
        let mut receiver = None;
        if self.eat(&Tok::LParen) {
            let mut rname = None;
            if let Tok::Ident(ref n) = self.peek().clone() {
                rname = Some(n.clone());
                self.advance();
            }
            let rtype = self.parse_type()?;
            receiver = Some(Param { name: rname, ty: rtype });
            self.expect(&Tok::RParen)?;
        }

        let name = match self.peek().clone() {
            Tok::Ident(n) => { self.advance(); n }
            _ => return Err(format!("line {}: expected function name", line)),
        };

        // Generics on function: [T any, U comparable]
        if self.eat(&Tok::LBracket) {
            self.skip_balanced(Tok::LBracket, Tok::RBracket);
        }

        // Parameters
        let mut params = Vec::new();
        self.expect(&Tok::LParen)?;
        while !matches!(self.peek(), Tok::RParen | Tok::Eof) {
            let mut names = Vec::new();
            if let Tok::Ident(n) = self.peek().clone() {
                names.push(n);
                self.advance();
                while self.eat(&Tok::Comma) {
                    if let Tok::Ident(n2) = self.peek().clone() {
                        names.push(n2);
                        self.advance();
                    } else {
                        break;
                    }
                }
            }

            // Variadic ...T
            self.eat(&Tok::Ellipsis);

            let ty = if !matches!(self.peek(), Tok::RParen | Tok::Comma) {
                self.parse_type()?
            } else {
                Type::Basic(BasicType::Any)
            };

            if names.is_empty() {
                params.push(Param { name: None, ty });
            } else {
                for n in names {
                    params.push(Param { name: Some(n), ty: ty.clone() });
                }
            }

            if !self.eat(&Tok::Comma) { break; }
        }
        self.expect(&Tok::RParen)?;

        // Results (return types)
        let mut results = Vec::new();
        if self.eat(&Tok::LParen) {
            // (int, error) or (val int, err error)
            while !matches!(self.peek(), Tok::RParen | Tok::Eof) {
                let mut rname = None;
                if let Tok::Ident(n) = self.peek().clone() {
                    rname = Some(n);
                    self.advance();
                }
                let ty = if !matches!(self.peek(), Tok::RParen | Tok::Comma) {
                    self.parse_type()?
                } else if let Some(ref n) = rname {
                    // It was a type, not a name
                    Type::Named(n.clone())
                } else {
                    Type::Basic(BasicType::Any)
                };
                results.push(Param { name: rname, ty });
                if !self.eat(&Tok::Comma) { break; }
            }
            self.expect(&Tok::RParen)?;
        } else if !matches!(self.peek(), Tok::LBrace | Tok::Semicolon | Tok::Eof) {
            // Single return type
            let ty = self.parse_type()?;
            results.push(Param { name: None, ty });
        }

        // Body
        let body = if matches!(self.peek(), Tok::LBrace) {
            self.parse_block()?
        } else {
            self.eat(&Tok::Semicolon);
            Block { stmts: Vec::new() }
        };

        Ok(Function { name, receiver, params, results, body, line })
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        match self.peek().clone() {
            Tok::Star => {
                self.advance();
                let inner = self.parse_type()?;
                Ok(Type::Pointer(Box::new(inner)))
            }
            Tok::LBracket => {
                self.advance();
                let len = if let Tok::Int(n) = self.peek() {
                    let l = *n as usize;
                    self.advance();
                    Some(l)
                } else {
                    self.eat(&Tok::Ellipsis);
                    None
                };
                self.expect(&Tok::RBracket)?;
                let elem = self.parse_type()?;
                if len.is_some() {
                    Ok(Type::Array(len, Box::new(elem)))
                } else {
                    Ok(Type::Slice(Box::new(elem)))
                }
            }
            Tok::Map => {
                self.advance();
                self.expect(&Tok::LBracket)?;
                let key = self.parse_type()?;
                self.expect(&Tok::RBracket)?;
                let val = self.parse_type()?;
                Ok(Type::Map(Box::new(key), Box::new(val)))
            }
            Tok::Chan => {
                self.advance();
                let dir = if self.eat(&Tok::Arrow) { ChanDir::Send } else { ChanDir::Both };
                let elem = self.parse_type()?;
                Ok(Type::Chan(dir, Box::new(elem)))
            }
            Tok::Arrow => {
                // <-chan T
                self.advance();
                self.expect(&Tok::Chan)?;
                let elem = self.parse_type()?;
                Ok(Type::Chan(ChanDir::Recv, Box::new(elem)))
            }
            Tok::Struct => {
                self.advance();
                self.expect(&Tok::LBrace)?;
                let mut fields = Vec::new();
                while !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
                    let s_start = self.pos;
                    let mut names = Vec::new();
                    if let Tok::Ident(n) = self.peek().clone() {
                        names.push(n);
                        self.advance();
                        while self.eat(&Tok::Comma) {
                            if let Tok::Ident(n2) = self.peek().clone() {
                                names.push(n2);
                                self.advance();
                            }
                        }
                    }
                    if !matches!(self.peek(), Tok::RBrace | Tok::Semicolon) {
                        let ty = self.parse_type().unwrap_or(Type::Basic(BasicType::Any));
                        if names.is_empty() {
                            fields.push(StructField { name: String::new(), ty });
                        } else {
                            for n in names {
                                fields.push(StructField { name: n, ty: ty.clone() });
                            }
                        }
                    }
                    // Tag string `json:"..."`
                    if matches!(self.peek(), Tok::Str(_)) { self.advance(); }
                    self.eat(&Tok::Semicolon);
                    if self.pos <= s_start && !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
                        self.advance();
                    }
                }
                self.expect(&Tok::RBrace)?;
                Ok(Type::Struct(fields))
            }
            Tok::Interface => {
                self.advance();
                self.expect(&Tok::LBrace)?;
                let mut methods = Vec::new();
                while !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
                    if let Tok::Ident(n) = self.peek().clone() {
                        methods.push(n);
                        self.advance();
                        if self.eat(&Tok::LParen) {
                            self.skip_balanced(Tok::LParen, Tok::RParen);
                        }
                        if !matches!(self.peek(), Tok::RBrace | Tok::Semicolon) {
                            let _ = self.parse_type();
                        }
                    } else {
                        self.advance();
                    }
                    self.eat(&Tok::Semicolon);
                }
                self.expect(&Tok::RBrace)?;
                Ok(Type::Interface(methods))
            }
            Tok::Func => {
                self.advance();
                self.expect(&Tok::LParen)?;
                let mut in_types = Vec::new();
                while !matches!(self.peek(), Tok::RParen | Tok::Eof) {
                    if let Ok(t) = self.parse_type() { in_types.push(t); }
                    if !self.eat(&Tok::Comma) { break; }
                }
                self.expect(&Tok::RParen)?;
                let mut out_types = Vec::new();
                if self.eat(&Tok::LParen) {
                    while !matches!(self.peek(), Tok::RParen | Tok::Eof) {
                        if let Ok(t) = self.parse_type() { out_types.push(t); }
                        if !self.eat(&Tok::Comma) { break; }
                    }
                    self.expect(&Tok::RParen)?;
                } else if !matches!(self.peek(), Tok::LBrace | Tok::RParen | Tok::Comma | Tok::Semicolon | Tok::RBracket | Tok::Eof) {
                    if let Ok(t) = self.parse_type() { out_types.push(t); }
                }
                Ok(Type::Func(in_types, out_types))
            }
            Tok::Ident(ref n) => {
                let mut name = n.clone();
                self.advance();
                // Qualified name: pkg.Type
                if self.eat(&Tok::Dot) {
                    if let Tok::Ident(ref n2) = self.peek().clone() {
                        name.push('.');
                        name.push_str(n2);
                        self.advance();
                    }
                }
                // Generic type params: T[A, B]
                if self.eat(&Tok::LBracket) {
                    self.skip_balanced(Tok::LBracket, Tok::RBracket);
                }

                let basic = match name.as_str() {
                    "int" => Some(BasicType::Int),
                    "int8" => Some(BasicType::Int8),
                    "int16" => Some(BasicType::Int16),
                    "int32" => Some(BasicType::Int32),
                    "int64" => Some(BasicType::Int64),
                    "uint" => Some(BasicType::Uint),
                    "uint8" => Some(BasicType::Uint8),
                    "uint16" => Some(BasicType::Uint16),
                    "uint32" => Some(BasicType::Uint32),
                    "uint64" => Some(BasicType::Uint64),
                    "uintptr" => Some(BasicType::Uintptr),
                    "float32" => Some(BasicType::Float32),
                    "float64" => Some(BasicType::Float64),
                    "bool" => Some(BasicType::Bool),
                    "string" => Some(BasicType::String),
                    "byte" => Some(BasicType::Byte),
                    "rune" => Some(BasicType::Rune),
                    "error" => Some(BasicType::Error),
                    "any" => Some(BasicType::Any),
                    _ => None,
                };

                if let Some(b) = basic {
                    Ok(Type::Basic(b))
                } else {
                    Ok(Type::Named(name))
                }
            }
            _ => Err(format!("line {}: unexpected token in type: {:?}", self.peek_line(), self.peek())),
        }
    }

    fn parse_block(&mut self) -> Result<Block, String> {
        self.expect(&Tok::LBrace)?;
        let mut stmts = Vec::new();
        while !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
            let start_pos = self.pos;
            match self.parse_stmt() {
                Ok(s) => stmts.push(s),
                Err(_) => {
                    // Synchronize to next statement
                    while !matches!(self.peek(), Tok::Semicolon | Tok::RBrace | Tok::Eof
                        | Tok::Var | Tok::Const | Tok::If | Tok::For | Tok::Return | Tok::Go | Tok::Defer) {
                        if matches!(self.peek(), Tok::LBrace) {
                            self.skip_balanced(Tok::LBrace, Tok::RBrace);
                        } else {
                            self.advance();
                        }
                    }
                    if self.pos <= start_pos && !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
                        self.advance();
                    }
                }
            }
            self.eat(&Tok::Semicolon);
            if self.pos <= start_pos && !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
                self.advance();
            }
        }
        self.expect(&Tok::RBrace)?;
        Ok(Block { stmts })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        let line = self.peek_line();
        match self.peek().clone() {
            Tok::Var | Tok::Const => {
                let is_const = matches!(self.peek(), Tok::Const);
                self.advance();
                let mut names = Vec::new();
                if self.eat(&Tok::LParen) {
                    while !matches!(self.peek(), Tok::RParen | Tok::Eof) {
                        if let Tok::Ident(n) = self.peek().clone() {
                            names.push(n);
                            self.advance();
                        }
                        while !matches!(self.peek(), Tok::Semicolon | Tok::RParen | Tok::Eof) {
                            self.advance();
                        }
                        self.eat(&Tok::Semicolon);
                    }
                    self.expect(&Tok::RParen)?;
                    return Ok(Stmt::Decl(DclStmt { names, ty: None, init: None, line }));
                }

                if let Tok::Ident(n) = self.peek().clone() {
                    names.push(n);
                    self.advance();
                    while self.eat(&Tok::Comma) {
                        if let Tok::Ident(n2) = self.peek().clone() {
                            names.push(n2);
                            self.advance();
                        }
                    }
                }
                let ty = if !matches!(self.peek(), Tok::Assign | Tok::Semicolon | Tok::RBrace | Tok::Eof) {
                    self.parse_type().ok()
                } else {
                    None
                };
                let init = if self.eat(&Tok::Assign) {
                    self.parse_expr().ok()
                } else {
                    None
                };
                Ok(Stmt::Decl(DclStmt { names, ty, init, line }))
            }
            Tok::If => {
                self.advance();
                // Optional simple statement: if init; cond { ... }
                let first = self.parse_simple_stmt()?;
                let (init, cond) = if self.eat(&Tok::Semicolon) {
                    let cond = self.parse_expr()?;
                    (first, cond)
                } else if let Some(Stmt::Expr(ExprStmt { expr, .. })) = first {
                    (None, expr)
                } else {
                    (first, Expr::Literal(Literal::Bool(true, line)))
                };

                let then = self.parse_block()?;
                let els = if self.eat(&Tok::Else) {
                    if matches!(self.peek(), Tok::If) {
                        let if_stmt = self.parse_stmt()?;
                        Some(Block { stmts: vec![if_stmt] })
                    } else {
                        Some(self.parse_block()?)
                    }
                } else {
                    None
                };
                Ok(Stmt::If(IfStmt { init: init.map(Box::new), cond, then, els, line }))
            }
            Tok::For => {
                self.advance();
                // for range x { ... }
                if self.eat(&Tok::Range) {
                    let iter = self.parse_expr()?;
                    let body = self.parse_block()?;
                    return Ok(Stmt::Range(RangeStmt { key: None, val: None, iter, body, line }));
                }
                // for { ... }
                if matches!(self.peek(), Tok::LBrace) {
                    let body = self.parse_block()?;
                    return Ok(Stmt::For(ForStmt { init: None, cond: None, post: None, body, line }));
                }

                // Could be: for cond { ... } or for init; cond; post { ... } or for k, v := range x
                let first = self.parse_simple_stmt()?;
                if let Some(Stmt::Range(r)) = first {
                    return Ok(Stmt::Range(r));
                }

                if self.eat(&Tok::Semicolon) {
                    let cond = if !matches!(self.peek(), Tok::Semicolon) {
                        self.parse_expr().ok()
                    } else {
                        None
                    };
                    self.expect(&Tok::Semicolon)?;
                    let post = if !matches!(self.peek(), Tok::LBrace) {
                        self.parse_simple_stmt()?
                    } else {
                        None
                    };
                    let body = self.parse_block()?;
                    Ok(Stmt::For(ForStmt { init: first.map(Box::new), cond, post: post.map(Box::new), body, line }))
                } else if let Some(Stmt::Expr(ExprStmt { expr, .. })) = first {
                    let body = self.parse_block()?;
                    Ok(Stmt::For(ForStmt { init: None, cond: Some(expr), post: None, body, line }))
                } else {
                    let body = self.parse_block()?;
                    Ok(Stmt::For(ForStmt { init: first.map(Box::new), cond: None, post: None, body, line }))
                }
            }
            Tok::Switch => {
                self.advance();
                let first = self.parse_simple_stmt()?;
                let tag = if self.eat(&Tok::Semicolon) {
                    if !matches!(self.peek(), Tok::LBrace) {
                        self.parse_expr().ok().map(Box::new)
                    } else {
                        None
                    }
                } else if let Some(Stmt::Expr(ExprStmt { expr, .. })) = first {
                    Some(Box::new(expr))
                } else {
                    None
                };

                self.expect(&Tok::LBrace)?;
                let mut cases = Vec::new();
                while !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
                    let case_start = self.pos;
                    if self.eat(&Tok::Case) {
                        let mut exprs = Vec::new();
                        while !matches!(self.peek(), Tok::Colon | Tok::Eof) {
                            let e_start = self.pos;
                            if let Ok(e) = self.parse_expr() { exprs.push(e); }
                            if !self.eat(&Tok::Comma) { break; }
                            if self.pos <= e_start { self.advance(); }
                        }
                        self.expect(&Tok::Colon)?;
                        let mut body_stmts = Vec::new();
                        while !matches!(self.peek(), Tok::Case | Tok::Default | Tok::RBrace | Tok::Eof) {
                            let s_start = self.pos;
                            if let Ok(s) = self.parse_stmt() { body_stmts.push(s); }
                            self.eat(&Tok::Semicolon);
                            if self.pos <= s_start && !matches!(self.peek(), Tok::Case | Tok::Default | Tok::RBrace | Tok::Eof) {
                                self.advance();
                            }
                        }
                        cases.push(SwitchCase { exprs, body: Block { stmts: body_stmts } });
                    } else if self.eat(&Tok::Default) {
                        self.expect(&Tok::Colon)?;
                        let mut body_stmts = Vec::new();
                        while !matches!(self.peek(), Tok::Case | Tok::Default | Tok::RBrace | Tok::Eof) {
                            let s_start = self.pos;
                            if let Ok(s) = self.parse_stmt() { body_stmts.push(s); }
                            self.eat(&Tok::Semicolon);
                            if self.pos <= s_start && !matches!(self.peek(), Tok::Case | Tok::Default | Tok::RBrace | Tok::Eof) {
                                self.advance();
                            }
                        }
                        cases.push(SwitchCase { exprs: Vec::new(), body: Block { stmts: body_stmts } });
                    } else {
                        self.advance();
                    }
                    if self.pos <= case_start && !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
                        self.advance();
                    }
                }
                self.expect(&Tok::RBrace)?;
                Ok(Stmt::Switch(SwitchStmt { init: None, tag, cases, line }))
            }
            Tok::Select => {
                self.advance();
                self.expect(&Tok::LBrace)?;
                let mut cases = Vec::new();
                let mut default = None;
                while !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
                    let sel_start = self.pos;
                    if self.eat(&Tok::Case) {
                        let comm = self.parse_stmt().ok();
                        self.expect(&Tok::Colon)?;
                        let mut body_stmts = Vec::new();
                        while !matches!(self.peek(), Tok::Case | Tok::Default | Tok::RBrace | Tok::Eof) {
                            let s_start = self.pos;
                            if let Ok(s) = self.parse_stmt() { body_stmts.push(s); }
                            self.eat(&Tok::Semicolon);
                            if self.pos <= s_start && !matches!(self.peek(), Tok::Case | Tok::Default | Tok::RBrace | Tok::Eof) {
                                self.advance();
                            }
                        }
                        cases.push(SelectCase { comm, body: Block { stmts: body_stmts } });
                    } else if self.eat(&Tok::Default) {
                        self.expect(&Tok::Colon)?;
                        let mut body_stmts = Vec::new();
                        while !matches!(self.peek(), Tok::Case | Tok::Default | Tok::RBrace | Tok::Eof) {
                            let s_start = self.pos;
                            if let Ok(s) = self.parse_stmt() { body_stmts.push(s); }
                            self.eat(&Tok::Semicolon);
                            if self.pos <= s_start && !matches!(self.peek(), Tok::Case | Tok::Default | Tok::RBrace | Tok::Eof) {
                                self.advance();
                            }
                        }
                        default = Some(Block { stmts: body_stmts });
                    } else {
                        self.advance();
                    }
                    if self.pos <= sel_start && !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
                        self.advance();
                    }
                }
                self.expect(&Tok::RBrace)?;
                Ok(Stmt::Select(SelectStmt { cases, default, line }))
            }
            Tok::Return => {
                self.advance();
                let mut values = Vec::new();
                while !matches!(self.peek(), Tok::Semicolon | Tok::RBrace | Tok::Eof) {
                    values.push(self.parse_expr()?);
                    if !self.eat(&Tok::Comma) { break; }
                }
                Ok(Stmt::Return(ReturnStmt { values, line }))
            }
            Tok::Go => {
                self.advance();
                let call = self.parse_expr()?;
                Ok(Stmt::Go(GoStmt { call, line }))
            }
            Tok::Defer => {
                self.advance();
                let call = self.parse_expr()?;
                Ok(Stmt::Defer(DeferStmt { call, line }))
            }
            Tok::Break => {
                self.advance();
                let lbl = if let Tok::Ident(l) = self.peek().clone() { self.advance(); Some(l) } else { None };
                Ok(Stmt::Branch(BranchKind::Break, lbl, line))
            }
            Tok::Continue => {
                self.advance();
                let lbl = if let Tok::Ident(l) = self.peek().clone() { self.advance(); Some(l) } else { None };
                Ok(Stmt::Branch(BranchKind::Continue, lbl, line))
            }
            Tok::Goto => {
                self.advance();
                let lbl = if let Tok::Ident(l) = self.peek().clone() { self.advance(); Some(l) } else { None };
                Ok(Stmt::Branch(BranchKind::Goto, lbl, line))
            }
            Tok::Fallthrough => {
                self.advance();
                Ok(Stmt::Branch(BranchKind::Fallthrough, None, line))
            }
            Tok::Close => {
                self.advance();
                self.expect(&Tok::LParen)?;
                let ch = self.parse_expr()?;
                self.expect(&Tok::RParen)?;
                Ok(Stmt::Close(CloseStmt { ch: Box::new(ch), line }))
            }
            Tok::LBrace => {
                let blk = self.parse_block()?;
                Ok(Stmt::Block(blk))
            }
            Tok::Semicolon => {
                self.advance();
                Ok(Stmt::Block(Block { stmts: Vec::new() }))
            }
            Tok::RBrace | Tok::Eof => {
                Err("end of block".into())
            }
            _ => {
                if let Some(s) = self.parse_simple_stmt()? {
                    Ok(s)
                } else {
                    let expr = self.parse_expr()?;
                    Ok(Stmt::Expr(ExprStmt { expr, line }))
                }
            }
        }
    }

    fn parse_simple_stmt(&mut self) -> Result<Option<Stmt>, String> {
        let line = self.peek_line();
        if matches!(self.peek(), Tok::Semicolon | Tok::LBrace) {
            return Ok(None);
        }

        let exprs = self.parse_expr_list()?;
        if exprs.is_empty() {
            return Ok(None);
        }

        // Check for range: for k, v := range iter
        if matches!(self.peek(), Tok::Define | Tok::Assign) {
            let is_def = matches!(self.peek(), Tok::Define);
            self.advance();
            if self.eat(&Tok::Range) {
                let iter = self.parse_expr()?;
                let body = self.parse_block()?;
                let k = exprs.get(0).and_then(|e| match e { Expr::Ident(i) => Some(i.name.clone()), _ => None });
                let v = exprs.get(1).and_then(|e| match e { Expr::Ident(i) => Some(i.name.clone()), _ => None });
                return Ok(Some(Stmt::Range(RangeStmt { key: k, val: v, iter, body, line })));
            } else {
                let rhs = self.parse_expr_list()?;
                return Ok(Some(Stmt::Assign(AssignStmt { lhs: exprs, rhs, is_define: is_def, line })));
            }
        }

        // Compound assignments
        if matches!(self.peek(),
            Tok::PlusAssign | Tok::MinusAssign | Tok::StarAssign | Tok::SlashAssign | Tok::PercentAssign
            | Tok::AmpAssign | Tok::PipeAssign | Tok::CaretAssign | Tok::ShlAssign | Tok::ShrAssign | Tok::AmpCaretAssign
        ) {
            self.advance();
            let rhs = self.parse_expr_list()?;
            return Ok(Some(Stmt::Assign(AssignStmt { lhs: exprs, rhs, is_define: false, line })));
        }

        // Channel send: ch <- val
        if self.eat(&Tok::Arrow) {
            let val = self.parse_expr()?;
            return Ok(Some(Stmt::Send(SendStmt { ch: Box::new(exprs[0].clone()), val: Box::new(val), line })));
        }

        // Inc / Dec
        if self.eat(&Tok::PlusPlus) {
            return Ok(Some(Stmt::IncDec(exprs[0].clone(), true, line)));
        }
        if self.eat(&Tok::MinusMinus) {
            return Ok(Some(Stmt::IncDec(exprs[0].clone(), false, line)));
        }

        // Plain expression statement
        Ok(Some(Stmt::Expr(ExprStmt { expr: exprs[0].clone(), line })))
    }

    fn parse_expr_list(&mut self) -> Result<Vec<Expr>, String> {
        let mut list = Vec::new();
        list.push(self.parse_expr()?);
        while self.eat(&Tok::Comma) {
            list.push(self.parse_expr()?);
        }
        Ok(list)
    }

    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_binary(1)
    }

    fn parse_binary(&mut self, min_prec: u8) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;

        while let Some((op, prec)) = self.peek_bin_op() {
            if prec < min_prec { break; }
            let op_line = self.peek_line();
            if op_line > self.prev_line {
                // Binary operator on a new line cannot continue previous line in Go
                break;
            }
            self.advance();
            let right = self.parse_binary(prec + 1)?;
            left = Expr::Binary(BinaryExpr { op, lhs: Box::new(left), rhs: Box::new(right), line: op_line });
        }

        Ok(left)
    }

    fn peek_bin_op(&self) -> Option<(BinOp, u8)> {
        match self.peek() {
            Tok::OrOr => Some((BinOp::Or, 1)),
            Tok::AndAnd => Some((BinOp::And, 2)),
            Tok::Eq => Some((BinOp::Eq, 3)),
            Tok::Neq => Some((BinOp::Ne, 3)),
            Tok::Lt => Some((BinOp::Lt, 3)),
            Tok::Le => Some((BinOp::Le, 3)),
            Tok::Gt => Some((BinOp::Gt, 3)),
            Tok::Ge => Some((BinOp::Ge, 3)),
            Tok::Plus => Some((BinOp::Add, 4)),
            Tok::Minus => Some((BinOp::Sub, 4)),
            Tok::Pipe => Some((BinOp::BitOr, 4)),
            Tok::Caret => Some((BinOp::BitXor, 4)),
            Tok::Star => Some((BinOp::Mul, 5)),
            Tok::Slash => Some((BinOp::Div, 5)),
            Tok::Percent => Some((BinOp::Mod, 5)),
            Tok::Shl => Some((BinOp::Shl, 5)),
            Tok::Shr => Some((BinOp::Shr, 5)),
            Tok::Amp => Some((BinOp::BitAnd, 5)),
            Tok::AmpCaret => Some((BinOp::BitClear, 5)),
            _ => None,
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        let line = self.peek_line();
        if self.eat(&Tok::Plus) {
            return self.parse_unary();
        }
        if self.eat(&Tok::Minus) {
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryExpr { op: UnOp::Neg, operand: Box::new(operand), line }));
        }
        if self.eat(&Tok::Caret) {
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryExpr { op: UnOp::BitNot, operand: Box::new(operand), line }));
        }
        if self.eat(&Tok::Star) {
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryExpr { op: UnOp::Deref, operand: Box::new(operand), line }));
        }
        if self.eat(&Tok::Amp) {
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryExpr { op: UnOp::Addr, operand: Box::new(operand), line }));
        }
        if self.eat(&Tok::Arrow) {
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryExpr { op: UnOp::Receive, operand: Box::new(operand), line }));
        }
        // ! is lexed as Neq if standalone
        if matches!(self.peek(), Tok::Neq) {
            // Check if followed by '=' (which is already Neq) or single '!'
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryExpr { op: UnOp::Not, operand: Box::new(operand), line }));
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek().clone() {
                Tok::Dot => {
                    self.advance();
                    // Type assertion: x.(type) or x.(T)
                    if self.eat(&Tok::LParen) {
                        let ty = if let Tok::Ident(n) = self.peek().clone() {
                            if n == "type" {
                                self.advance();
                                None
                            } else {
                                self.parse_type().ok()
                            }
                        } else {
                            self.parse_type().ok()
                        };
                        self.expect(&Tok::RParen)?;
                        expr = Expr::TypeAssert(TypeAssertExpr { base: Box::new(expr), ty, line: self.peek_line() });
                    } else if let Tok::Ident(n) = self.peek().clone() {
                        self.advance();
                        expr = Expr::Field(FieldExpr { base: Box::new(expr), name: n, line: self.peek_line() });
                    } else {
                        break;
                    }
                }
                Tok::LBracket => {
                    self.advance();
                    // Slice: a[low:high] or a[low:high:max] or index a[i]
                    let mut low = None;
                    let mut high = None;
                    let mut max = None;

                    if !matches!(self.peek(), Tok::Colon) {
                        low = Some(Box::new(self.parse_expr()?));
                    }

                    if self.eat(&Tok::Colon) {
                        if !matches!(self.peek(), Tok::Colon | Tok::RBracket) {
                            high = Some(Box::new(self.parse_expr()?));
                        }
                        if self.eat(&Tok::Colon) {
                            max = Some(Box::new(self.parse_expr()?));
                        }
                        self.expect(&Tok::RBracket)?;
                        expr = Expr::Slice(SliceExpr { base: Box::new(expr), low, high, max, line: self.peek_line() });
                    } else {
                        self.expect(&Tok::RBracket)?;
                        let idx = low.unwrap_or_else(|| Box::new(Expr::Literal(Literal::Int(0, self.peek_line()))));
                        let l = index_line(&idx);
                        expr = Expr::Index(IndexExpr { base: Box::new(expr), index: idx, line: l });
                    }
                }
                Tok::LParen => {
                    // Function call
                    self.advance();
                    let mut args = Vec::new();
                    while !matches!(self.peek(), Tok::RParen | Tok::Eof) {
                        args.push(self.parse_expr()?);
                        self.eat(&Tok::Ellipsis);
                        if !self.eat(&Tok::Comma) { break; }
                    }
                    self.expect(&Tok::RParen)?;
                    expr = Expr::Call(CallExpr { func: Box::new(expr), args, line: self.peek_line() });
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let line = self.peek_line();
        match self.peek().clone() {
            Tok::Int(v) => { self.advance(); Ok(Expr::Literal(Literal::Int(v, line))) }
            Tok::Float(v) => { self.advance(); Ok(Expr::Literal(Literal::Float(v, line))) }
            Tok::Str(s) => { self.advance(); Ok(Expr::Literal(Literal::Str(s, line))) }
            Tok::Rune(c) => { self.advance(); Ok(Expr::Literal(Literal::Int(c as i64, line))) }
            Tok::True => { self.advance(); Ok(Expr::Literal(Literal::Bool(true, line))) }
            Tok::False => { self.advance(); Ok(Expr::Literal(Literal::Bool(false, line))) }
            Tok::Nil => { self.advance(); Ok(Expr::Literal(Literal::Nil(line))) }
            Tok::Make => {
                self.advance();
                self.expect(&Tok::LParen)?;
                let ty = self.parse_type().unwrap_or(Type::Basic(BasicType::Any));
                let mut cap = None;
                while self.eat(&Tok::Comma) {
                    cap = self.parse_expr().ok().map(Box::new);
                }
                self.expect(&Tok::RParen)?;
                Ok(Expr::ChanMake(ChanMake { ty, cap, line }))
            }
            Tok::New => {
                self.advance();
                self.expect(&Tok::LParen)?;
                let ty = self.parse_type().unwrap_or(Type::Basic(BasicType::Any));
                self.expect(&Tok::RParen)?;
                Ok(Expr::Call(CallExpr {
                    func: Box::new(Expr::Ident(Ident { name: "builtin.new".into(), line })),
                    args: vec![],
                    line,
                }))
            }
            Tok::Len => {
                self.advance();
                self.expect(&Tok::LParen)?;
                let arg = self.parse_expr()?;
                self.expect(&Tok::RParen)?;
                Ok(Expr::Call(CallExpr {
                    func: Box::new(Expr::Ident(Ident { name: "builtin.len".into(), line })),
                    args: vec![arg],
                    line,
                }))
            }
            Tok::Cap => {
                self.advance();
                self.expect(&Tok::LParen)?;
                let arg = self.parse_expr()?;
                self.expect(&Tok::RParen)?;
                Ok(Expr::Call(CallExpr {
                    func: Box::new(Expr::Ident(Ident { name: "builtin.cap".into(), line })),
                    args: vec![arg],
                    line,
                }))
            }
            Tok::Close => {
                self.advance();
                self.expect(&Tok::LParen)?;
                let arg = self.parse_expr()?;
                self.expect(&Tok::RParen)?;
                Ok(Expr::Call(CallExpr {
                    func: Box::new(Expr::Ident(Ident { name: "builtin.close".into(), line })),
                    args: vec![arg],
                    line,
                }))
            }
            Tok::Panic => {
                self.advance();
                self.expect(&Tok::LParen)?;
                let arg = self.parse_expr()?;
                self.expect(&Tok::RParen)?;
                Ok(Expr::Call(CallExpr {
                    func: Box::new(Expr::Ident(Ident { name: "builtin.panic".into(), line })),
                    args: vec![arg],
                    line,
                }))
            }
            Tok::Ident(name) => {
                self.advance();
                // Check for composite literal: Type{ ... } or pkg.Type{ ... }
                let mut full_name = name.clone();
                if self.peek() == &Tok::Dot {
                    // Peek ahead: if Dot followed by Ident and then LBrace
                    self.advance();
                    if let Tok::Ident(n2) = self.peek().clone() {
                        self.advance();
                        full_name.push('.');
                        full_name.push_str(&n2);
                    }
                }

                if matches!(self.peek(), Tok::LBrace) {
                    // Try parsing composite literal
                    let elems = self.parse_composite_elements()?;
                    return Ok(Expr::CompositeLit(CompositeLit {
                        ty: Some(Type::Named(full_name)),
                        elems,
                        line,
                    }));
                }

                Ok(Expr::Ident(Ident { name: full_name, line }))
            }
            Tok::LParen => {
                self.advance();
                let inner = self.parse_expr()?;
                self.expect(&Tok::RParen)?;
                Ok(Expr::Paren(Box::new(inner)))
            }
            Tok::LBracket => {
                // Slice / array literal: []int{1, 2, 3}
                let ty = self.parse_type()?;
                if matches!(self.peek(), Tok::LBrace) {
                    let elems = self.parse_composite_elements()?;
                    return Ok(Expr::CompositeLit(CompositeLit {
                        ty: Some(ty),
                        elems,
                        line,
                    }));
                }
                Ok(Expr::Literal(Literal::Nil(line)))
            }
            Tok::Map => {
                let ty = self.parse_type()?;
                if matches!(self.peek(), Tok::LBrace) {
                    let elems = self.parse_composite_elements()?;
                    return Ok(Expr::CompositeLit(CompositeLit {
                        ty: Some(ty),
                        elems,
                        line,
                    }));
                }
                Ok(Expr::Literal(Literal::Nil(line)))
            }
            Tok::Func => {
                // Function literal: func(params) { ... }
                self.advance();
                self.expect(&Tok::LParen)?;
                let mut params = Vec::new();
                while !matches!(self.peek(), Tok::RParen | Tok::Eof) {
                    let mut names = Vec::new();
                    if let Tok::Ident(n) = self.peek().clone() {
                        names.push(n);
                        self.advance();
                    }
                    let ty = self.parse_type().unwrap_or(Type::Basic(BasicType::Any));
                    if names.is_empty() {
                        params.push(Param { name: None, ty });
                    } else {
                        for n in names { params.push(Param { name: Some(n), ty: ty.clone() }); }
                    }
                    if !self.eat(&Tok::Comma) { break; }
                }
                self.expect(&Tok::RParen)?;

                let mut results = Vec::new();
                if self.eat(&Tok::LParen) {
                    while !matches!(self.peek(), Tok::RParen | Tok::Eof) {
                        let ty = self.parse_type().unwrap_or(Type::Basic(BasicType::Any));
                        results.push(Param { name: None, ty });
                        if !self.eat(&Tok::Comma) { break; }
                    }
                    self.expect(&Tok::RParen)?;
                } else if !matches!(self.peek(), Tok::LBrace | Tok::Eof) {
                    let ty = self.parse_type().unwrap_or(Type::Basic(BasicType::Any));
                    results.push(Param { name: None, ty });
                }

                let body = self.parse_block()?;
                Ok(Expr::FuncLit(FuncLit { params, results, body, line }))
            }
            _ => Err(format!("line {}: unexpected token in expression: {:?}", line, self.peek())),
        }
    }

    fn parse_composite_elements(&mut self) -> Result<Vec<Expr>, String> {
        self.expect(&Tok::LBrace)?;
        let mut elems = Vec::new();
        while !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
            let elem_start = self.pos;
            let elem_line = self.peek_line();
            let expr = if matches!(self.peek(), Tok::LBrace) {
                // Anonymous struct/slice literal: {0, 0, 1}
                let sub_elems = self.parse_composite_elements()?;
                Expr::CompositeLit(CompositeLit { ty: None, elems: sub_elems, line: elem_line })
            } else {
                let expr = self.parse_expr()?;
                if self.eat(&Tok::Colon) {
                    // key: value
                    let val = self.parse_expr()?;
                    Expr::KeyValue(Box::new(expr), Box::new(val), elem_line)
                } else {
                    expr
                }
            };
            elems.push(expr);
            self.eat(&Tok::Comma);
            self.eat(&Tok::Semicolon);
            if self.pos <= elem_start && !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
                self.advance();
            }
        }
        self.expect(&Tok::RBrace)?;
        Ok(elems)
    }
}

fn index_line(e: &Expr) -> usize {
    match e {
        Expr::Ident(i) => i.line,
        Expr::Literal(l) => match l {
            Literal::Int(_, l) | Literal::Float(_, l) | Literal::Str(_, l)
            | Literal::Bool(_, l) | Literal::Nil(l) => *l,
        },
        Expr::Binary(b) => b.line,
        Expr::Unary(u) => u.line,
        Expr::Call(c) => c.line,
        _ => 0,
    }
}
