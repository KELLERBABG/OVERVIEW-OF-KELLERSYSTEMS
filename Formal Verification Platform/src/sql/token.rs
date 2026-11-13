//! Lexer and token definitions for the Keller-VIR SQL frontend.

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords - DML & Queries
    Select,
    From,
    Where,
    Insert,
    Into,
    Values,
    Update,
    Set,
    Delete,
    With,
    Recursive,
    As,
    Union,
    All,
    Intersect,
    Except,
    Join,
    Inner,
    Left,
    Right,
    Full,
    Outer,
    Cross,
    Natural,
    On,
    Using,
    Group,
    By,
    Having,
    Order,
    Asc,
    Desc,
    Limit,
    Offset,
    Distinct,

    // Keywords - DDL
    Create,
    Alter,
    Drop,
    Table,
    View,
    Index,
    Trigger,
    Function,
    Procedure,
    Constraint,
    Primary,
    Key,
    Foreign,
    References,
    Check,
    Default,
    Unique,
    Add,
    Column,
    Modify,
    Change,
    Rename,
    To,
    Cascade,
    Restrict,
    Sequence,
    Schema,
    Database,

    // Keywords - Logic & Predicates
    Case,
    When,
    Then,
    Else,
    End,
    Nullif,
    Coalesce,
    Cast,
    Is,
    Null,
    Not,
    And,
    Or,
    Like,
    In,
    Between,
    Exists,
    True,
    False,

    // Dialect & Special Keywords
    Identity,
    AutoIncrement,
    Serial,
    Generated,
    Always,
    Copy,
    Stdin,
    Begin,
    Commit,
    Rollback,
    Declare,
    If,
    Loop,
    Execute,
    Return,
    Returns,
    Replace,
    Before,
    After,
    Row,
    Each,

    // Literals & Identifiers
    Ident(String),
    StringLit(String),
    IntLit(i64),
    FloatLit(f64),

    // Symbols & Operators
    Semi,
    Comma,
    Dot,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Concat,
    ColonEq,
    DoubleColon,
    Colon,
    Question,

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, col: usize) -> Self {
        Token { kind, line, col }
    }
}

pub struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn current_line(&self) -> usize {
        self.line
    }

    fn peek_byte(&self) -> Option<u8> {
        if self.pos < self.bytes.len() {
            Some(self.bytes[self.pos])
        } else {
            None
        }
    }

    fn peek_byte_offset(&self, offset: usize) -> Option<u8> {
        if self.pos + offset < self.bytes.len() {
            Some(self.bytes[self.pos + offset])
        } else {
            None
        }
    }

    fn advance_byte(&mut self) -> Option<u8> {
        if self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
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
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
                self.advance_byte();
            } else if b == b'-' && self.peek_byte_offset(1) == Some(b'-') {
                // Line comment
                while let Some(c) = self.advance_byte() {
                    if c == b'\n' {
                        break;
                    }
                }
            } else if b == b'/' && self.peek_byte_offset(1) == Some(b'*') {
                // Block comment
                self.advance_byte();
                self.advance_byte();
                while self.pos < self.bytes.len() {
                    if self.bytes[self.pos] == b'*' && self.peek_byte_offset(1) == Some(b'/') {
                        self.advance_byte();
                        self.advance_byte();
                        break;
                    }
                    self.advance_byte();
                }
            } else {
                break;
            }
        }
    }

    /// When `COPY ... FROM stdin;` is encountered, fast-skip lines until `\.\n` (or `\.` at EOF).
    pub fn skip_copy_stdin_data(&mut self) {
        while self.pos < self.bytes.len() {
            // Check for line starting with `\.` followed by \r?\n or EOF
            if (self.pos == 0 || self.bytes[self.pos - 1] == b'\n')
                && self.bytes[self.pos] == b'\\'
                && self.peek_byte_offset(1) == Some(b'.')
            {
                let next = self.peek_byte_offset(2);
                if next == Some(b'\n') || next == Some(b'\r') || next.is_none() {
                    self.advance_byte(); // \
                    self.advance_byte(); // .
                    if self.peek_byte() == Some(b'\r') {
                        self.advance_byte();
                    }
                    if self.peek_byte() == Some(b'\n') {
                        self.advance_byte();
                    }
                    break;
                }
            }
            self.advance_byte();
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let line = self.line;
        let col = self.col;

        let Some(b) = self.peek_byte() else {
            return Token::new(TokenKind::Eof, line, col);
        };

        // Identifiers: quoted
        if b == b'`' {
            self.advance_byte();
            let start = self.pos;
            while let Some(c) = self.advance_byte() {
                if c == b'`' {
                    let text = &self.src[start..self.pos - 1];
                    return Token::new(TokenKind::Ident(text.to_string()), line, col);
                }
            }
            let text = &self.src[start..self.pos];
            return Token::new(TokenKind::Ident(text.to_string()), line, col);
        }

        if b == b'[' {
            // Check if this is a bracketed identifier like `[Album]` or a symbol
            // If the next character is an ident char, treat as bracketed identifier
            if let Some(next) = self.peek_byte_offset(1) {
                if next.is_ascii_alphanumeric() || next == b'_' || next == b' ' {
                    self.advance_byte();
                    let start = self.pos;
                    while let Some(c) = self.advance_byte() {
                        if c == b']' {
                            let text = &self.src[start..self.pos - 1];
                            return Token::new(TokenKind::Ident(text.trim().to_string()), line, col);
                        }
                    }
                    let text = &self.src[start..self.pos];
                    return Token::new(TokenKind::Ident(text.to_string()), line, col);
                }
            }
            self.advance_byte();
            return Token::new(TokenKind::LBracket, line, col);
        }

        if b == b']' {
            self.advance_byte();
            return Token::new(TokenKind::RBracket, line, col);
        }

        if b == b'"' {
            // ANSI SQL double-quoted identifier
            self.advance_byte();
            let start = self.pos;
            while let Some(c) = self.advance_byte() {
                if c == b'"' {
                    if self.peek_byte() == Some(b'"') {
                        self.advance_byte(); // escaped quote
                    } else {
                        let text = self.src[start..self.pos - 1].replace("\"\"", "\"");
                        return Token::new(TokenKind::Ident(text), line, col);
                    }
                }
            }
            let text = &self.src[start..self.pos];
            return Token::new(TokenKind::Ident(text.to_string()), line, col);
        }

        // Single-quoted string literal
        if b == b'\'' {
            self.advance_byte();
            let mut s = String::new();
            while let Some(c) = self.advance_byte() {
                if c == b'\'' {
                    if self.peek_byte() == Some(b'\'') {
                        self.advance_byte();
                        s.push('\'');
                    } else {
                        return Token::new(TokenKind::StringLit(s), line, col);
                    }
                } else if c == b'\\' && self.peek_byte() == Some(b'\'') {
                    self.advance_byte();
                    s.push('\'');
                } else {
                    s.push(c as char);
                }
            }
            return Token::new(TokenKind::StringLit(s), line, col);
        }

        // PostgreSQL dollar-quoted string: $tag$...$tag$ or $$...$$
        if b == b'$' {
            if let Some(tag_end) = self.try_match_dollar_tag() {
                let full_tag = self.src[self.pos..tag_end].to_string();
                for _ in 0..full_tag.len() {
                    self.advance_byte();
                }
                let mut content = String::new();
                while self.pos < self.bytes.len() {
                    if self.src[self.pos..].starts_with(&full_tag) {
                        for _ in 0..full_tag.len() {
                            self.advance_byte();
                        }
                        return Token::new(TokenKind::StringLit(content), line, col);
                    }
                    if let Some(c) = self.advance_byte() {
                        content.push(c as char);
                    }
                }
                return Token::new(TokenKind::StringLit(content), line, col);
            }
        }

        // Numbers: digits or leading decimal dot
        if b.is_ascii_digit() || (b == b'.' && self.peek_byte_offset(1).map_or(false, |x| x.is_ascii_digit())) {
            return self.lex_number(line, col);
        }

        // Identifiers and Keywords
        if b.is_ascii_alphabetic() || b == b'_' || b == b'@' || b == b'#' {
            let start = self.pos;
            while let Some(c) = self.peek_byte() {
                if c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c == b'#' || c == b'@' {
                    self.advance_byte();
                } else {
                    break;
                }
            }
            let word = &self.src[start..self.pos];

            // Check if SQL Server 'GO' on its own
            if word.eq_ignore_ascii_case("GO") {
                return Token::new(TokenKind::Semi, line, col);
            }

            let kind = match_keyword(word);
            return Token::new(kind, line, col);
        }

        // Punctuation and Multi-character Operators
        self.advance_byte();
        match b {
            b';' => Token::new(TokenKind::Semi, line, col),
            b',' => Token::new(TokenKind::Comma, line, col),
            b'.' => Token::new(TokenKind::Dot, line, col),
            b'(' => Token::new(TokenKind::LParen, line, col),
            b')' => Token::new(TokenKind::RParen, line, col),
            b'+' => Token::new(TokenKind::Plus, line, col),
            b'-' => Token::new(TokenKind::Minus, line, col),
            b'*' => Token::new(TokenKind::Star, line, col),
            b'%' => Token::new(TokenKind::Percent, line, col),
            b'?' => Token::new(TokenKind::Question, line, col),
            b'/' => {
                // In Oracle SQL*Plus, `/` alone on a line acts as a statement delimiter!
                if col == 1 {
                    // check if the rest of line is whitespace
                    let mut i = self.pos;
                    let mut alone = true;
                    while i < self.bytes.len() && self.bytes[i] != b'\n' {
                        if self.bytes[i] != b' ' && self.bytes[i] != b'\t' && self.bytes[i] != b'\r' {
                            alone = false;
                            break;
                        }
                        i += 1;
                    }
                    if alone {
                        return Token::new(TokenKind::Semi, line, col);
                    }
                }
                Token::new(TokenKind::Slash, line, col)
            }
            b'=' => Token::new(TokenKind::Eq, line, col),
            b'<' => {
                if self.peek_byte() == Some(b'=') {
                    self.advance_byte();
                    Token::new(TokenKind::LtEq, line, col)
                } else if self.peek_byte() == Some(b'>') {
                    self.advance_byte();
                    Token::new(TokenKind::NotEq, line, col)
                } else {
                    Token::new(TokenKind::Lt, line, col)
                }
            }
            b'>' => {
                if self.peek_byte() == Some(b'=') {
                    self.advance_byte();
                    Token::new(TokenKind::GtEq, line, col)
                } else {
                    Token::new(TokenKind::Gt, line, col)
                }
            }
            b'!' => {
                if self.peek_byte() == Some(b'=') {
                    self.advance_byte();
                    Token::new(TokenKind::NotEq, line, col)
                } else {
                    Token::new(TokenKind::Ident("!".to_string()), line, col)
                }
            }
            b'|' => {
                if self.peek_byte() == Some(b'|') {
                    self.advance_byte();
                    Token::new(TokenKind::Concat, line, col)
                } else {
                    Token::new(TokenKind::Ident("|".to_string()), line, col)
                }
            }
            b':' => {
                if self.peek_byte() == Some(b':') {
                    self.advance_byte();
                    Token::new(TokenKind::DoubleColon, line, col)
                } else if self.peek_byte() == Some(b'=') {
                    self.advance_byte();
                    Token::new(TokenKind::ColonEq, line, col)
                } else {
                    Token::new(TokenKind::Colon, line, col)
                }
            }
            _ => Token::new(TokenKind::Ident((b as char).to_string()), line, col),
        }
    }

    fn try_match_dollar_tag(&self) -> Option<usize> {
        // Look ahead for $tag$
        let mut idx = self.pos + 1;
        while idx < self.bytes.len() {
            let b = self.bytes[idx];
            if b == b'$' {
                return Some(idx + 1);
            }
            if !b.is_ascii_alphanumeric() && b != b'_' {
                return None;
            }
            idx += 1;
        }
        None
    }

    fn lex_number(&mut self, line: usize, col: usize) -> Token {
        let start = self.pos;
        let mut has_dot = false;
        let mut is_hex = false;

        if self.bytes[self.pos] == b'0' && self.peek_byte_offset(1).map_or(false, |b| b == b'x' || b == b'X') {
            is_hex = true;
            self.advance_byte();
            self.advance_byte();
            while let Some(b) = self.peek_byte() {
                if b.is_ascii_hexdigit() {
                    self.advance_byte();
                } else {
                    break;
                }
            }
        } else {
            while let Some(b) = self.peek_byte() {
                if b.is_ascii_digit() {
                    self.advance_byte();
                } else if b == b'.' && !has_dot {
                    has_dot = true;
                    self.advance_byte();
                } else if b == b'e' || b == b'E' {
                    self.advance_byte();
                    if let Some(sign) = self.peek_byte() {
                        if sign == b'+' || sign == b'-' {
                            self.advance_byte();
                        }
                    }
                    while let Some(d) = self.peek_byte() {
                        if d.is_ascii_digit() {
                            self.advance_byte();
                        } else {
                            break;
                        }
                    }
                    has_dot = true;
                } else {
                    break;
                }
            }
        }

        let num_str = &self.src[start..self.pos];
        if is_hex {
            let val = i64::from_str_radix(&num_str[2..], 16).unwrap_or(0);
            Token::new(TokenKind::IntLit(val), line, col)
        } else if has_dot {
            let val = num_str.parse::<f64>().unwrap_or(0.0);
            Token::new(TokenKind::FloatLit(val), line, col)
        } else {
            let val = num_str.parse::<i64>().unwrap_or(0);
            Token::new(TokenKind::IntLit(val), line, col)
        }
    }
}

fn match_keyword(word: &str) -> TokenKind {
    let lower = word.to_ascii_lowercase();
    match lower.as_str() {
        "select" => TokenKind::Select,
        "from" => TokenKind::From,
        "where" => TokenKind::Where,
        "insert" => TokenKind::Insert,
        "into" => TokenKind::Into,
        "values" => TokenKind::Values,
        "update" => TokenKind::Update,
        "set" => TokenKind::Set,
        "delete" => TokenKind::Delete,
        "with" => TokenKind::With,
        "recursive" => TokenKind::Recursive,
        "as" => TokenKind::As,
        "union" => TokenKind::Union,
        "all" => TokenKind::All,
        "intersect" => TokenKind::Intersect,
        "except" => TokenKind::Except,
        "join" => TokenKind::Join,
        "inner" => TokenKind::Inner,
        "left" => TokenKind::Left,
        "right" => TokenKind::Right,
        "full" => TokenKind::Full,
        "outer" => TokenKind::Outer,
        "cross" => TokenKind::Cross,
        "natural" => TokenKind::Natural,
        "on" => TokenKind::On,
        "using" => TokenKind::Using,
        "group" => TokenKind::Group,
        "by" => TokenKind::By,
        "having" => TokenKind::Having,
        "order" => TokenKind::Order,
        "asc" => TokenKind::Asc,
        "desc" => TokenKind::Desc,
        "limit" => TokenKind::Limit,
        "offset" => TokenKind::Offset,
        "distinct" => TokenKind::Distinct,
        "create" => TokenKind::Create,
        "alter" => TokenKind::Alter,
        "drop" => TokenKind::Drop,
        "table" => TokenKind::Table,
        "view" => TokenKind::View,
        "index" => TokenKind::Index,
        "trigger" => TokenKind::Trigger,
        "function" => TokenKind::Function,
        "procedure" => TokenKind::Procedure,
        "constraint" => TokenKind::Constraint,
        "primary" => TokenKind::Primary,
        "key" => TokenKind::Key,
        "foreign" => TokenKind::Foreign,
        "references" => TokenKind::References,
        "check" => TokenKind::Check,
        "default" => TokenKind::Default,
        "unique" => TokenKind::Unique,
        "add" => TokenKind::Add,
        "column" => TokenKind::Column,
        "modify" => TokenKind::Modify,
        "change" => TokenKind::Change,
        "rename" => TokenKind::Rename,
        "to" => TokenKind::To,
        "cascade" => TokenKind::Cascade,
        "restrict" => TokenKind::Restrict,
        "sequence" => TokenKind::Sequence,
        "schema" => TokenKind::Schema,
        "database" => TokenKind::Database,
        "case" => TokenKind::Case,
        "when" => TokenKind::When,
        "then" => TokenKind::Then,
        "else" => TokenKind::Else,
        "end" => TokenKind::End,
        "nullif" => TokenKind::Nullif,
        "coalesce" => TokenKind::Coalesce,
        "cast" => TokenKind::Cast,
        "is" => TokenKind::Is,
        "null" => TokenKind::Null,
        "not" => TokenKind::Not,
        "and" => TokenKind::And,
        "or" => TokenKind::Or,
        "like" => TokenKind::Like,
        "in" => TokenKind::In,
        "between" => TokenKind::Between,
        "exists" => TokenKind::Exists,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "identity" => TokenKind::Identity,
        "auto_increment" | "autoincrement" => TokenKind::AutoIncrement,
        "serial" => TokenKind::Serial,
        "generated" => TokenKind::Generated,
        "always" => TokenKind::Always,
        "copy" => TokenKind::Copy,
        "stdin" => TokenKind::Stdin,
        "begin" => TokenKind::Begin,
        "commit" => TokenKind::Commit,
        "rollback" => TokenKind::Rollback,
        "declare" => TokenKind::Declare,
        "if" => TokenKind::If,
        "loop" => TokenKind::Loop,
        "execute" => TokenKind::Execute,
        "return" => TokenKind::Return,
        "returns" => TokenKind::Returns,
        "replace" => TokenKind::Replace,
        "before" => TokenKind::Before,
        "after" => TokenKind::After,
        "row" => TokenKind::Row,
        "each" => TokenKind::Each,
        _ => TokenKind::Ident(word.to_string()),
    }
}
