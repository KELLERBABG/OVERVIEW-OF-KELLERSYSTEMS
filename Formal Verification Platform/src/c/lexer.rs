//! C Tokenizer / Lexer.

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Void,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
    Signed,
    Unsigned,
    Bool,
    Struct,
    Union,
    Enum,
    Typedef,
    Const,
    Volatile,
    Static,
    Extern,
    Inline,
    Restrict,
    Sizeof,
    If,
    Else,
    Switch,
    Case,
    Default,
    While,
    Do,
    For,
    Return,
    Break,
    Continue,
    Goto,
    Null,

    // Identifiers & Literals
    Ident(String),
    IntLit(i64),
    FloatLit(f64),
    CharLit(char),
    StringLit(String),

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
    Question,
    Dot,
    Arrow,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Bang,
    AmpAmp,
    PipePipe,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Shl,
    Shr,
    Eq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    AmpEq,
    PipeEq,
    CaretEq,
    ShlEq,
    ShrEq,
    PlusPlus,
    MinusMinus,

    // Extensions & Macros
    Attribute, // __attribute__
    Asm,       // __asm__, asm

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer<'a> {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    _source: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            _source: source,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_ascii_whitespace() {
                self.advance();
            } else if ch == '/' && self.peek_next() == Some('/') {
                // Line comment
                self.advance();
                self.advance();
                while let Some(c) = self.peek() {
                    self.advance();
                    if c == '\n' {
                        break;
                    }
                }
            } else if ch == '/' && self.peek_next() == Some('*') {
                // Block comment
                self.advance();
                self.advance();
                while let Some(c) = self.advance() {
                    if c == '*' && self.peek() == Some('/') {
                        self.advance();
                        break;
                    }
                }
            } else if ch == '#' {
                // Preprocessor directive: consume line, handling backslash continuation
                self.advance();
                let mut in_quote = false;
                while let Some(c) = self.peek() {
                    if c == '"' {
                        in_quote = !in_quote;
                        self.advance();
                    } else if c == '\\' {
                        self.advance(); // consume backslash
                        if self.peek() == Some('\r') {
                            self.advance();
                        }
                        if self.peek() == Some('\n') {
                            self.advance();
                        }
                    } else if c == '\n' {
                        self.advance();
                        break;
                    } else {
                        self.advance();
                    }
                }
            } else {
                break;
            }
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }

        // Post-processing: concatenate consecutive string literals (standard C)
        let mut merged = Vec::with_capacity(tokens.len());
        let mut i = 0;
        while i < tokens.len() {
            if let TokenKind::StringLit(mut s) = tokens[i].kind.clone() {
                let start_line = tokens[i].line;
                let start_col = tokens[i].col;
                while i + 1 < tokens.len() {
                    if let TokenKind::StringLit(ref next_s) = tokens[i + 1].kind {
                        s.push_str(next_s);
                        i += 1;
                    } else {
                        break;
                    }
                }
                merged.push(Token {
                    kind: TokenKind::StringLit(s),
                    line: start_line,
                    col: start_col,
                });
            } else {
                merged.push(tokens[i].clone());
            }
            i += 1;
        }

        merged
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let line = self.line;
        let col = self.col;

        let ch = match self.advance() {
            Some(c) => c,
            None => {
                return Token {
                    kind: TokenKind::Eof,
                    line,
                    col,
                }
            }
        };

        match ch {
            '(' => Token { kind: TokenKind::LParen, line, col },
            ')' => Token { kind: TokenKind::RParen, line, col },
            '{' => Token { kind: TokenKind::LBrace, line, col },
            '}' => Token { kind: TokenKind::RBrace, line, col },
            '[' => Token { kind: TokenKind::LBracket, line, col },
            ']' => Token { kind: TokenKind::RBracket, line, col },
            ';' => Token { kind: TokenKind::Semicolon, line, col },
            ',' => Token { kind: TokenKind::Comma, line, col },
            ':' => Token { kind: TokenKind::Colon, line, col },
            '?' => Token { kind: TokenKind::Question, line, col },
            '~' => Token { kind: TokenKind::Tilde, line, col },

            '.' => {
                if let Some(next) = self.peek() {
                    if next.is_ascii_digit() {
                        return self.lex_number('.', line, col);
                    }
                }
                Token { kind: TokenKind::Dot, line, col }
            }

            '=' => {
                if self.match_char('=') {
                    Token { kind: TokenKind::EqEq, line, col }
                } else {
                    Token { kind: TokenKind::Eq, line, col }
                }
            }
            '!' => {
                if self.match_char('=') {
                    Token { kind: TokenKind::BangEq, line, col }
                } else {
                    Token { kind: TokenKind::Bang, line, col }
                }
            }
            '+' => {
                if self.match_char('+') {
                    Token { kind: TokenKind::PlusPlus, line, col }
                } else if self.match_char('=') {
                    Token { kind: TokenKind::PlusEq, line, col }
                } else {
                    Token { kind: TokenKind::Plus, line, col }
                }
            }
            '-' => {
                if self.match_char('>') {
                    Token { kind: TokenKind::Arrow, line, col }
                } else if self.match_char('-') {
                    Token { kind: TokenKind::MinusMinus, line, col }
                } else if self.match_char('=') {
                    Token { kind: TokenKind::MinusEq, line, col }
                } else {
                    Token { kind: TokenKind::Minus, line, col }
                }
            }
            '*' => {
                if self.match_char('=') {
                    Token { kind: TokenKind::StarEq, line, col }
                } else {
                    Token { kind: TokenKind::Star, line, col }
                }
            }
            '/' => {
                if self.match_char('=') {
                    Token { kind: TokenKind::SlashEq, line, col }
                } else {
                    Token { kind: TokenKind::Slash, line, col }
                }
            }
            '%' => {
                if self.match_char('=') {
                    Token { kind: TokenKind::PercentEq, line, col }
                } else {
                    Token { kind: TokenKind::Percent, line, col }
                }
            }
            '&' => {
                if self.match_char('&') {
                    Token { kind: TokenKind::AmpAmp, line, col }
                } else if self.match_char('=') {
                    Token { kind: TokenKind::AmpEq, line, col }
                } else {
                    Token { kind: TokenKind::Amp, line, col }
                }
            }
            '|' => {
                if self.match_char('|') {
                    Token { kind: TokenKind::PipePipe, line, col }
                } else if self.match_char('=') {
                    Token { kind: TokenKind::PipeEq, line, col }
                } else {
                    Token { kind: TokenKind::Pipe, line, col }
                }
            }
            '^' => {
                if self.match_char('=') {
                    Token { kind: TokenKind::CaretEq, line, col }
                } else {
                    Token { kind: TokenKind::Caret, line, col }
                }
            }
            '<' => {
                if self.match_char('<') {
                    if self.match_char('=') {
                        Token { kind: TokenKind::ShlEq, line, col }
                    } else {
                        Token { kind: TokenKind::Shl, line, col }
                    }
                } else if self.match_char('=') {
                    Token { kind: TokenKind::LtEq, line, col }
                } else {
                    Token { kind: TokenKind::Lt, line, col }
                }
            }
            '>' => {
                if self.match_char('>') {
                    if self.match_char('=') {
                        Token { kind: TokenKind::ShrEq, line, col }
                    } else {
                        Token { kind: TokenKind::Shr, line, col }
                    }
                } else if self.match_char('=') {
                    Token { kind: TokenKind::GtEq, line, col }
                } else {
                    Token { kind: TokenKind::Gt, line, col }
                }
            }

            '\'' => self.lex_char(line, col),
            '"' => self.lex_string(line, col),

            c if c.is_ascii_digit() => self.lex_number(c, line, col),
            c if c.is_alphabetic() || c == '_' => self.lex_ident_or_keyword(c, line, col),

            _ => {
                // Ignore unrecognized character and continue
                self.next_token()
            }
        }
    }

    fn lex_char(&mut self, line: usize, col: usize) -> Token {
        let mut val = '\0';
        if let Some(c) = self.advance() {
            if c == '\\' {
                val = self.lex_escape();
            } else {
                val = c;
            }
        }
        // Consume closing quote
        let _ = self.match_char('\'');
        Token {
            kind: TokenKind::CharLit(val),
            line,
            col,
        }
    }

    fn lex_escape(&mut self) -> char {
        match self.advance() {
            Some('n') => '\n',
            Some('t') => '\t',
            Some('r') => '\r',
            Some('0') => '\0',
            Some('\\') => '\\',
            Some('\'') => '\'',
            Some('"') => '"',
            Some('x') => {
                let mut h = 0u32;
                for _ in 0..2 {
                    if let Some(c) = self.peek() {
                        if let Some(d) = c.to_digit(16) {
                            self.advance();
                            h = h * 16 + d;
                        } else {
                            break;
                        }
                    }
                }
                char::from_u32(h).unwrap_or('\0')
            }
            Some(c) => c,
            None => '\0',
        }
    }

    fn lex_string(&mut self, line: usize, col: usize) -> Token {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c == '"' {
                self.advance();
                break;
            } else if c == '\\' {
                self.advance();
                s.push(self.lex_escape());
            } else if c == '\n' {
                self.advance();
                s.push('\n');
            } else {
                self.advance();
                s.push(c);
            }
        }
        Token {
            kind: TokenKind::StringLit(s),
            line,
            col,
        }
    }

    fn lex_number(&mut self, first: char, line: usize, col: usize) -> Token {
        let mut text = String::new();
        text.push(first);

        if first == '0' {
            if self.peek() == Some('x') || self.peek() == Some('X') {
                text.push(self.advance().unwrap());
                while let Some(c) = self.peek() {
                    if c.is_ascii_hexdigit() {
                        text.push(self.advance().unwrap());
                    } else {
                        break;
                    }
                }
                self.strip_int_suffix();
                let num_str = text.trim_start_matches("0x").trim_start_matches("0X");
                let val = i64::from_str_radix(if num_str.is_empty() { "0" } else { num_str }, 16).unwrap_or(0);
                return Token {
                    kind: TokenKind::IntLit(val),
                    line,
                    col,
                };
            } else if self.peek() == Some('b') || self.peek() == Some('B') {
                text.push(self.advance().unwrap());
                while let Some(c) = self.peek() {
                    if c == '0' || c == '1' {
                        text.push(self.advance().unwrap());
                    } else {
                        break;
                    }
                }
                self.strip_int_suffix();
                let num_str = text.trim_start_matches("0b").trim_start_matches("0B");
                let val = i64::from_str_radix(if num_str.is_empty() { "0" } else { num_str }, 2).unwrap_or(0);
                return Token {
                    kind: TokenKind::IntLit(val),
                    line,
                    col,
                };
            }
        }

        let mut is_float = first == '.';
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                text.push(self.advance().unwrap());
            } else if c == '.' && !is_float {
                is_float = true;
                text.push(self.advance().unwrap());
            } else if (c == 'e' || c == 'E') && !text.contains('e') && !text.contains('E') {
                is_float = true;
                text.push(self.advance().unwrap());
                if let Some(sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        text.push(self.advance().unwrap());
                    }
                }
            } else {
                break;
            }
        }

        if is_float {
            self.strip_float_suffix();
            let val = text.parse::<f64>().unwrap_or(0.0);
            Token {
                kind: TokenKind::FloatLit(val),
                line,
                col,
            }
        } else {
            self.strip_int_suffix();
            let val = if text.starts_with('0') && text.len() > 1 && text.chars().all(|c| ('0'..='7').contains(&c)) {
                i64::from_str_radix(&text, 8).unwrap_or(0)
            } else {
                text.parse::<i64>().unwrap_or(0)
            };
            Token {
                kind: TokenKind::IntLit(val),
                line,
                col,
            }
        }
    }

    fn strip_int_suffix(&mut self) {
        while let Some(c) = self.peek() {
            if c == 'u' || c == 'U' || c == 'l' || c == 'L' || c == 'z' || c == 'Z' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn strip_float_suffix(&mut self) {
        if let Some(c) = self.peek() {
            if c == 'f' || c == 'F' || c == 'l' || c == 'L' {
                self.advance();
            }
        }
    }

    fn lex_ident_or_keyword(&mut self, first: char, line: usize, col: usize) -> Token {
        let mut s = String::new();
        s.push(first);
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(self.advance().unwrap());
            } else {
                break;
            }
        }

        let kind = match s.as_str() {
            "void" => TokenKind::Void,
            "char" => TokenKind::Char,
            "short" => TokenKind::Short,
            "int" | "int8_t" | "int16_t" | "int32_t" | "int64_t" | "size_t" | "ssize_t" | "intptr_t" | "uintptr_t" => TokenKind::Int,
            "long" => TokenKind::Long,
            "float" => TokenKind::Float,
            "double" => TokenKind::Double,
            "signed" => TokenKind::Signed,
            "unsigned" | "uint8_t" | "uint16_t" | "uint32_t" | "uint64_t" => TokenKind::Unsigned,
            "_Bool" | "bool" => TokenKind::Bool,
            "struct" => TokenKind::Struct,
            "union" => TokenKind::Union,
            "enum" => TokenKind::Enum,
            "typedef" => TokenKind::Typedef,
            "const" => TokenKind::Const,
            "volatile" => TokenKind::Volatile,
            "static" => TokenKind::Static,
            "extern" => TokenKind::Extern,
            "inline" | "__inline" | "__inline__" => TokenKind::Inline,
            "restrict" | "__restrict" | "__restrict__" => TokenKind::Restrict,
            "sizeof" => TokenKind::Sizeof,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "switch" => TokenKind::Switch,
            "case" => TokenKind::Case,
            "default" => TokenKind::Default,
            "while" => TokenKind::While,
            "do" => TokenKind::Do,
            "for" => TokenKind::For,
            "return" => TokenKind::Return,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "goto" => TokenKind::Goto,
            "NULL" | "nullptr" => TokenKind::Null,
            "true" => TokenKind::IntLit(1),
            "false" => TokenKind::IntLit(0),
            "__attribute__" => TokenKind::Attribute,
            "asm" | "__asm" | "__asm__" => TokenKind::Asm,
            "INT_MAX" => TokenKind::IntLit(2147483647),
            "INT_MIN" => TokenKind::IntLit(-2147483648),
            "UINT_MAX" => TokenKind::IntLit(4294967295),
            _ => TokenKind::Ident(s),
        };

        Token { kind, line, col }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_basic() {
        let code = r#"
            #include <stdio.h>
            int main() {
                int *p = NULL;
                p[0] = 42;
                return 0;
            }
        "#;
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Int));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Ident("main".into())));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Null));
    }
}
