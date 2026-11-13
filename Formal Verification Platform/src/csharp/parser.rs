//! Fast, resilient recursive descent parser for C# source code (.cs).

use crate::csharp::ast::*;

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
    As,
    Async,
    Await,
    Base,
    Bool,
    Break,
    Byte,
    Case,
    Catch,
    CharKw,
    Checked,
    Class,
    Const,
    Continue,
    Decimal,
    Default,
    Delegate,
    Do,
    Double,
    Else,
    Enum,
    Event,
    Explicit,
    Extern,
    Finally,
    Fixed,
    FloatKw,
    For,
    ForEach,
    Goto,
    If,
    Implicit,
    In,
    IntKw,
    Interface,
    Internal,
    Is,
    Lock,
    Long,
    Namespace,
    New,
    Not,
    Object,
    Operator,
    Out,
    Override,
    Params,
    Private,
    Protected,
    Public,
    Readonly,
    Record,
    Ref,
    Return,
    Sbyte,
    Sealed,
    Short,
    Sizeof,
    Stackalloc,
    Static,
    StringKw,
    Struct,
    Switch,
    This,
    Throw,
    Try,
    Typeof,
    Uint,
    Ulong,
    Unchecked,
    Unsafe,
    Ushort,
    Using,
    Var,
    Virtual,
    Void,
    Volatile,
    While,
    Yield,
    Get,
    Set,
    Init,

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
    QuestionDot,
    QuestionBracket,
    QuestionQuestion,
    QuestionQuestionAssign,
    ColonColon,
    Arrow,
    DotDot,
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
    AmpAssign,
    PipeAssign,
    CaretAssign,
    ShlAssign,
    ShrAssign,

    Eq,
    Ne,
    BangEqual,
    Lt,
    Gt,
    Le,
    Ge,

    And,
    Or,
    Bang,
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

    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Tok,
    pub loc: SourceLoc,
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

    fn loc(&self) -> SourceLoc {
        SourceLoc::new(self.line, self.col)
    }

    fn skip_whitespace_comments_and_preprocessor(&mut self) {
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
                Some(b'#') => {
                    // Preprocessor directive: #region, #if, #nullable, etc.
                    while let Some(b) = self.peek() {
                        self.advance();
                        if b == b'\n' {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    pub fn tokenize(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_comments_and_preprocessor();
            let loc = self.loc();
            let Some(b) = self.peek() else {
                tokens.push(Token {
                    kind: Tok::Eof,
                    loc,
                });
                break;
            };

            // Character literal
            if b == b'\'' {
                tokens.push(Token {
                    kind: self.read_char(),
                    loc,
                });
                continue;
            }

            // String literals
            if b == b'"' {
                tokens.push(Token {
                    kind: self.read_string(),
                    loc,
                });
                continue;
            }

            // Verbatim string @"..." or interpolated verbatim $@"..." / @$"..."
            if b == b'@' && self.peek_at(1) == Some(b'"') {
                self.advance(); // @
                tokens.push(Token {
                    kind: self.read_verbatim_string(),
                    loc,
                });
                continue;
            }

            if b == b'$' {
                if self.peek_at(1) == Some(b'@') && self.peek_at(2) == Some(b'"') {
                    self.advance(); // $
                    self.advance(); // @
                    tokens.push(Token {
                        kind: self.read_verbatim_string(),
                        loc,
                    });
                    continue;
                } else if self.peek_at(1) == Some(b'"') {
                    self.advance(); // $
                    tokens.push(Token {
                        kind: self.read_string(),
                        loc,
                    });
                    continue;
                }
            }

            // Numbers
            if b.is_ascii_digit() {
                tokens.push(Token {
                    kind: self.read_number(),
                    loc,
                });
                continue;
            }

            // Identifiers or keywords
            if is_ident_start(b) || b == b'@' {
                tokens.push(Token {
                    kind: self.read_ident_or_kw(),
                    loc,
                });
                continue;
            }

            // Two-character / multi-character operators
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
                b'~' => Tok::BitNot,

                b'?' => {
                    if self.peek() == Some(b'.') {
                        self.advance();
                        Tok::QuestionDot
                    } else if self.peek() == Some(b'[') {
                        self.advance();
                        Tok::QuestionBracket
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

                b':' => {
                    if self.peek() == Some(b':') {
                        self.advance();
                        Tok::ColonColon
                    } else {
                        Tok::Colon
                    }
                }

                b'.' => {
                    if self.peek() == Some(b'.') {
                        self.advance();
                        Tok::DotDot
                    } else {
                        Tok::Dot
                    }
                }

                b'=' => {
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::Eq
                    } else if self.peek() == Some(b'>') {
                        self.advance();
                        Tok::Arrow
                    } else {
                        Tok::Assign
                    }
                }

                b'!' => {
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::BangEqual
                    } else {
                        Tok::Bang
                    }
                }

                b'+' => {
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
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::MulAssign
                    } else {
                        Tok::Star
                    }
                }

                b'/' => {
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::DivAssign
                    } else {
                        Tok::Slash
                    }
                }

                b'%' => {
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::RemAssign
                    } else {
                        Tok::Percent
                    }
                }

                b'&' => {
                    if self.peek() == Some(b'&') {
                        self.advance();
                        Tok::And
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::AmpAssign
                    } else {
                        Tok::Amp
                    }
                }

                b'|' => {
                    if self.peek() == Some(b'|') {
                        self.advance();
                        Tok::Or
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::PipeAssign
                    } else {
                        Tok::Pipe
                    }
                }

                b'^' => {
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::CaretAssign
                    } else {
                        Tok::Caret
                    }
                }

                b'<' => {
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::Le
                    } else if self.peek() == Some(b'<') {
                        self.advance();
                        if self.peek() == Some(b'=') {
                            self.advance();
                            Tok::ShlAssign
                        } else {
                            Tok::Shl
                        }
                    } else {
                        Tok::Lt
                    }
                }

                b'>' => {
                    if self.peek() == Some(b'=') {
                        self.advance();
                        Tok::Ge
                    } else if self.peek() == Some(b'>') {
                        self.advance();
                        if self.peek() == Some(b'=') {
                            self.advance();
                            Tok::ShrAssign
                        } else {
                            Tok::Shr
                        }
                    } else {
                        Tok::Gt
                    }
                }

                _ => Tok::Ident(format!("{}", b as char)),
            };

            tokens.push(Token { kind, loc });
        }
        tokens
    }

    fn read_char(&mut self) -> Tok {
        self.advance(); // consume opening '
        let mut ch = '\0';
        if let Some(b) = self.peek() {
            if b == b'\\' {
                self.advance();
                ch = match self.advance() {
                    Some(b'n') => '\n',
                    Some(b'r') => '\r',
                    Some(b't') => '\t',
                    Some(b'0') => '\0',
                    Some(b'\\') => '\\',
                    Some(b'\'') => '\'',
                    Some(b'"') => '"',
                    Some(other) => other as char,
                    None => '\0',
                };
            } else {
                ch = b as char;
                self.advance();
            }
        }
        if self.peek() == Some(b'\'') {
            self.advance();
        }
        Tok::Char(ch)
    }

    fn read_string(&mut self) -> Tok {
        self.advance(); // consume opening "
        let mut s = String::new();
        while let Some(b) = self.peek() {
            if b == b'"' {
                self.advance();
                break;
            }
            if b == b'\\' {
                self.advance();
                match self.advance() {
                    Some(b'n') => s.push('\n'),
                    Some(b'r') => s.push('\r'),
                    Some(b't') => s.push('\t'),
                    Some(b'0') => s.push('\0'),
                    Some(b'\\') => s.push('\\'),
                    Some(b'"') => s.push('"'),
                    Some(b'\'') => s.push('\''),
                    Some(other) => s.push(other as char),
                    None => break,
                }
            } else {
                s.push(b as char);
                self.advance();
            }
        }
        Tok::Str(s)
    }

    fn read_verbatim_string(&mut self) -> Tok {
        self.advance(); // consume "
        let mut s = String::new();
        while let Some(b) = self.peek() {
            if b == b'"' {
                self.advance();
                if self.peek() == Some(b'"') {
                    // Escaped double quote
                    s.push('"');
                    self.advance();
                } else {
                    break;
                }
            } else {
                s.push(b as char);
                self.advance();
            }
        }
        Tok::Str(s)
    }

    fn read_number(&mut self) -> Tok {
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
                    self.skip_number_suffix();
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
                    self.skip_number_suffix();
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

        if let Some(b'f' | b'F' | b'd' | b'D' | b'm' | b'M') = self.peek() {
            is_float = true;
            self.advance();
        } else {
            self.skip_number_suffix();
        }

        if is_float {
            let val = num_str.parse::<f64>().unwrap_or(0.0);
            Tok::Float(val)
        } else {
            let val = num_str.parse::<i64>().unwrap_or(0);
            Tok::Int(val)
        }
    }

    fn skip_number_suffix(&mut self) {
        while let Some(b'u' | b'U' | b'l' | b'L') = self.peek() {
            self.advance();
        }
    }

    fn read_ident_or_kw(&mut self) -> Tok {
        if self.peek() == Some(b'@') {
            self.advance(); // Escaped identifier, e.g. @var
        }

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
            "as" => Tok::As,
            "async" => Tok::Async,
            "await" => Tok::Await,
            "base" => Tok::Base,
            "bool" => Tok::Bool,
            "break" => Tok::Break,
            "byte" => Tok::Byte,
            "case" => Tok::Case,
            "catch" => Tok::Catch,
            "char" => Tok::CharKw,
            "checked" => Tok::Checked,
            "class" => Tok::Class,
            "const" => Tok::Const,
            "continue" => Tok::Continue,
            "decimal" => Tok::Decimal,
            "default" => Tok::Default,
            "delegate" => Tok::Delegate,
            "do" => Tok::Do,
            "double" => Tok::Double,
            "else" => Tok::Else,
            "enum" => Tok::Enum,
            "event" => Tok::Event,
            "explicit" => Tok::Explicit,
            "extern" => Tok::Extern,
            "false" => Tok::False,
            "finally" => Tok::Finally,
            "fixed" => Tok::Fixed,
            "float" => Tok::FloatKw,
            "for" => Tok::For,
            "foreach" => Tok::ForEach,
            "goto" => Tok::Goto,
            "if" => Tok::If,
            "implicit" => Tok::Implicit,
            "in" => Tok::In,
            "int" => Tok::IntKw,
            "interface" => Tok::Interface,
            "internal" => Tok::Internal,
            "is" => Tok::Is,
            "lock" => Tok::Lock,
            "long" => Tok::Long,
            "namespace" => Tok::Namespace,
            "new" => Tok::New,
            "not" => Tok::Not,
            "null" => Tok::Null,
            "object" => Tok::Object,
            "operator" => Tok::Operator,
            "out" => Tok::Out,
            "override" => Tok::Override,
            "params" => Tok::Params,
            "private" => Tok::Private,
            "protected" => Tok::Protected,
            "public" => Tok::Public,
            "readonly" => Tok::Readonly,
            "record" => Tok::Record,
            "ref" => Tok::Ref,
            "return" => Tok::Return,
            "sbyte" => Tok::Sbyte,
            "sealed" => Tok::Sealed,
            "short" => Tok::Short,
            "sizeof" => Tok::Sizeof,
            "stackalloc" => Tok::Stackalloc,
            "static" => Tok::Static,
            "string" => Tok::StringKw,
            "struct" => Tok::Struct,
            "switch" => Tok::Switch,
            "this" => Tok::This,
            "throw" => Tok::Throw,
            "true" => Tok::True,
            "try" => Tok::Try,
            "typeof" => Tok::Typeof,
            "uint" => Tok::Uint,
            "ulong" => Tok::Ulong,
            "unchecked" => Tok::Unchecked,
            "unsafe" => Tok::Unsafe,
            "ushort" => Tok::Ushort,
            "using" => Tok::Using,
            "var" => Tok::Var,
            "virtual" => Tok::Virtual,
            "void" => Tok::Void,
            "volatile" => Tok::Volatile,
            "while" => Tok::While,
            "yield" => Tok::Yield,
            "get" => Tok::Get,
            "set" => Tok::Set,
            "init" => Tok::Init,
            _ => Tok::Ident(s),
        }
    }
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Parse C# source code into a `CompilationUnit`.
pub fn parse(src: &str) -> Result<CompilationUnit, String> {
    let lexer = Lexer::new(src);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse_unit()
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Tok {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos].kind
        } else {
            &Tok::Eof
        }
    }

    fn peek_next(&self) -> &Tok {
        if self.pos + 1 < self.tokens.len() {
            &self.tokens[self.pos + 1].kind
        } else {
            &Tok::Eof
        }
    }

    fn loc(&self) -> SourceLoc {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].loc
        } else {
            self.tokens.last().map(|t| t.loc).unwrap_or_default()
        }
    }

    fn advance(&mut self) -> Token {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            tok
        } else {
            Token {
                kind: Tok::Eof,
                loc: self.loc(),
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

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.peek() {
            Tok::Ident(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            Tok::Var => {
                self.advance();
                Ok("var".to_string())
            }
            Tok::Record => {
                self.advance();
                Ok("record".to_string())
            }
            Tok::Yield => {
                self.advance();
                Ok("yield".to_string())
            }
            Tok::Get => {
                self.advance();
                Ok("get".to_string())
            }
            Tok::Set => {
                self.advance();
                Ok("set".to_string())
            }
            Tok::Init => {
                self.advance();
                Ok("init".to_string())
            }
            other => Err(format!(
                "expected identifier, got {:?} at line {}",
                other,
                self.loc().line
            )),
        }
    }

    pub fn parse_unit(&mut self) -> Result<CompilationUnit, String> {
        let mut usings = Vec::new();
        let mut namespace = None;
        let mut types = Vec::new();

        while !self.check(&Tok::Eof) {
            // Attributes before usings/namespace
            if self.check(&Tok::LBracket) {
                self.skip_attribute();
                continue;
            }

            // Using directives
            if self.match_tok(&Tok::Using) {
                if self.match_tok(&Tok::Static) {
                    let _ = self.parse_qualified_name();
                } else {
                    let name = self.parse_qualified_name();
                    usings.push(name);
                }
                let _ = self.match_tok(&Tok::Semicolon);
                continue;
            }

            // Namespace declaration
            if self.match_tok(&Tok::Namespace) {
                let ns_name = self.parse_qualified_name();
                if self.match_tok(&Tok::Semicolon) {
                    // File-scoped namespace
                    namespace = Some(ns_name);
                } else if self.match_tok(&Tok::LBrace) {
                    // Braced namespace
                    namespace = Some(ns_name);
                    while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
                        if let Some(td) = self.parse_type_decl() {
                            types.push(td);
                        } else {
                            self.advance();
                        }
                    }
                    let _ = self.match_tok(&Tok::RBrace);
                }
                continue;
            }

            // Global using
            if let Tok::Ident(ref id) = self.peek() {
                if id == "global" && self.peek_next() == &Tok::Using {
                    self.advance();
                    self.advance();
                    let _ = self.parse_qualified_name();
                    let _ = self.match_tok(&Tok::Semicolon);
                    continue;
                }
            }

            // Type declaration
            if let Some(td) = self.parse_type_decl() {
                types.push(td);
            } else {
                self.advance();
            }
        }

        Ok(CompilationUnit {
            namespace,
            usings,
            types,
        })
    }

    fn parse_qualified_name(&mut self) -> String {
        let mut name = self.expect_ident().unwrap_or_default();
        while self.match_tok(&Tok::Dot) {
            if let Ok(part) = self.expect_ident() {
                name.push('.');
                name.push_str(&part);
            } else {
                break;
            }
        }
        name
    }

    fn skip_attribute(&mut self) {
        if self.match_tok(&Tok::LBracket) {
            let mut depth = 1;
            while depth > 0 && !self.check(&Tok::Eof) {
                match self.advance().kind {
                    Tok::LBracket => depth += 1,
                    Tok::RBracket => depth -= 1,
                    _ => {}
                }
            }
        }
    }

    fn parse_modifiers(&mut self) -> Vec<String> {
        let mut mods = Vec::new();
        loop {
            if self.check(&Tok::LBracket) {
                self.skip_attribute();
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
                Tok::Internal => {
                    self.advance();
                    mods.push("internal".into());
                }
                Tok::Static => {
                    self.advance();
                    mods.push("static".into());
                }
                Tok::Readonly => {
                    self.advance();
                    mods.push("readonly".into());
                }
                Tok::Const => {
                    self.advance();
                    mods.push("const".into());
                }
                Tok::Virtual => {
                    self.advance();
                    mods.push("virtual".into());
                }
                Tok::Override => {
                    self.advance();
                    mods.push("override".into());
                }
                Tok::Abstract => {
                    self.advance();
                    mods.push("abstract".into());
                }
                Tok::Sealed => {
                    self.advance();
                    mods.push("sealed".into());
                }
                Tok::Async => {
                    self.advance();
                    mods.push("async".into());
                }
                Tok::Unsafe => {
                    self.advance();
                    mods.push("unsafe".into());
                }
                Tok::Extern => {
                    self.advance();
                    mods.push("extern".into());
                }
                Tok::Ref => {
                    self.advance();
                    mods.push("ref".into());
                }
                Tok::Ident(ref s) if s == "partial" => {
                    self.advance();
                    mods.push("partial".into());
                }
                _ => break,
            }
        }
        mods
    }

    fn parse_type_decl(&mut self) -> Option<TypeDecl> {
        let loc = self.loc();
        let modifiers = self.parse_modifiers();

        let kind = if self.match_tok(&Tok::Class) {
            TypeKind::Class
        } else if self.match_tok(&Tok::Struct) {
            TypeKind::Struct
        } else if self.match_tok(&Tok::Interface) {
            TypeKind::Interface
        } else if self.match_tok(&Tok::Record) {
            // record or record struct / record class
            if self.match_tok(&Tok::Struct) {
                TypeKind::Struct
            } else {
                let _ = self.match_tok(&Tok::Class);
                TypeKind::Record
            }
        } else if self.match_tok(&Tok::Enum) {
            TypeKind::Enum
        } else {
            return None;
        };

        let name = self.expect_ident().ok()?;
        let type_params = self.skip_type_params();

        // Positional record parameters: record Foo(int X, int Y);
        if kind == TypeKind::Record && self.check(&Tok::LParen) {
            let _ = self.parse_param_list();
        }

        // Base types: : BaseClass, IFoo
        let mut base_types = Vec::new();
        if self.match_tok(&Tok::Colon) {
            loop {
                if let Ok(t) = self.parse_type() {
                    base_types.push(t);
                }
                if !self.match_tok(&Tok::Comma) {
                    break;
                }
            }
        }

        // Skip where constraints: where T : class
        self.skip_where_clauses();

        // Enum body
        if kind == TypeKind::Enum {
            if self.match_tok(&Tok::LBrace) {
                self.skip_to_matching_rbrace();
            }
            return Some(TypeDecl {
                name,
                kind,
                modifiers,
                type_params,
                base_types,
                methods: Vec::new(),
                constructors: Vec::new(),
                properties: Vec::new(),
                fields: Vec::new(),
                nested_types: Vec::new(),
                loc,
            });
        }

        // Semicolon-terminated record: record Foo(int X);
        if self.match_tok(&Tok::Semicolon) {
            return Some(TypeDecl {
                name,
                kind,
                modifiers,
                type_params,
                base_types,
                methods: Vec::new(),
                constructors: Vec::new(),
                properties: Vec::new(),
                fields: Vec::new(),
                nested_types: Vec::new(),
                loc,
            });
        }

        if !self.match_tok(&Tok::LBrace) {
            return None;
        }

        let mut methods = Vec::new();
        let mut constructors = Vec::new();
        let mut properties = Vec::new();
        let mut fields = Vec::new();
        let mut nested_types = Vec::new();

        while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
            if self.match_tok(&Tok::Semicolon) {
                continue;
            }

            let member_loc = self.loc();
            let member_mods = self.parse_modifiers();

            // Nested type
            if self.check(&Tok::Class)
                || self.check(&Tok::Struct)
                || self.check(&Tok::Interface)
                || self.check(&Tok::Record)
                || self.check(&Tok::Enum)
            {
                if let Some(nested) = self.parse_type_decl() {
                    nested_types.push(nested);
                    continue;
                }
            }

            // Constructor: Name matches class name followed by '('
            if let Tok::Ident(ref id) = self.peek() {
                if id == &name && self.peek_next() == &Tok::LParen {
                    let cname = self.advance();
                    let Tok::Ident(cname) = cname.kind else { unreachable!() };
                    let params = self.parse_param_list();
                    // Optional constructor initializer: : this(...) or : base(...)
                    if self.match_tok(&Tok::Colon) {
                        let _ = self.match_tok(&Tok::This) || self.match_tok(&Tok::Base);
                        if self.check(&Tok::LParen) {
                            let _ = self.parse_argument_list();
                        }
                    }
                    let body = if self.check(&Tok::LBrace) {
                        Some(self.parse_block())
                    } else if self.match_tok(&Tok::Arrow) {
                        let expr = self.parse_expr().ok();
                        let _ = self.match_tok(&Tok::Semicolon);
                        expr.map(|e| Block {
                            stmts: vec![Stmt::Expr(e, member_loc)],
                            loc: member_loc,
                        })
                    } else {
                        let _ = self.match_tok(&Tok::Semicolon);
                        None
                    };
                    constructors.push(ConstructorDecl {
                        name: cname,
                        modifiers: member_mods,
                        params,
                        body,
                        loc: member_loc,
                    });
                    continue;
                }
            }

            // Finalizer: ~Name()
            if self.match_tok(&Tok::BitNot) {
                let _ = self.expect_ident();
                if self.check(&Tok::LParen) {
                    let _ = self.parse_param_list();
                }
                if self.check(&Tok::LBrace) {
                    let _ = self.parse_block();
                }
                continue;
            }

            // Parse member type
            let Ok(member_ty) = self.parse_type() else {
                self.sync_to_next_member();
                continue;
            };

            // Operator overload: operator + (...)
            if self.match_tok(&Tok::Operator) {
                self.skip_operator_tokens();
                if self.check(&Tok::LParen) {
                    let _ = self.parse_param_list();
                }
                if self.check(&Tok::LBrace) {
                    let _ = self.parse_block();
                } else if self.match_tok(&Tok::Arrow) {
                    let _ = self.parse_expr();
                    let _ = self.match_tok(&Tok::Semicolon);
                }
                continue;
            }

            // Indexer: this[...]
            if self.match_tok(&Tok::This) {
                if self.check(&Tok::LBracket) {
                    self.skip_brackets();
                }
                if self.check(&Tok::LBrace) {
                    self.skip_to_matching_rbrace();
                } else if self.match_tok(&Tok::Arrow) {
                    let _ = self.parse_expr();
                    let _ = self.match_tok(&Tok::Semicolon);
                }
                continue;
            }

            let Ok(member_name) = self.expect_ident() else {
                self.sync_to_next_member();
                continue;
            };

            let _member_tparams = self.skip_type_params();

            // Check if Method
            if self.check(&Tok::LParen) {
                let params = self.parse_param_list();
                self.skip_where_clauses();

                let body = if self.check(&Tok::LBrace) {
                    Some(self.parse_block())
                } else if self.match_tok(&Tok::Arrow) {
                    let expr = self.parse_expr().ok();
                    let _ = self.match_tok(&Tok::Semicolon);
                    expr.map(|e| Block {
                        stmts: vec![Stmt::Return(Some(e), member_loc)],
                        loc: member_loc,
                    })
                } else {
                    let _ = self.match_tok(&Tok::Semicolon);
                    None
                };

                methods.push(MethodDecl {
                    name: member_name,
                    modifiers: member_mods,
                    return_ty: Some(member_ty),
                    params,
                    body,
                    loc: member_loc,
                });
            } else if self.check(&Tok::LBrace) {
                // Property with { get; set; }
                let (getter, setter) = self.parse_property_accessors();
                properties.push(PropertyDecl {
                    name: member_name,
                    modifiers: member_mods,
                    ty: member_ty,
                    getter,
                    setter,
                    loc: member_loc,
                });
            } else if self.match_tok(&Tok::Arrow) {
                // Expression-bodied property: public int X => expr;
                let expr = self.parse_expr().ok();
                let _ = self.match_tok(&Tok::Semicolon);
                let getter = expr.map(|e| Block {
                    stmts: vec![Stmt::Return(Some(e), member_loc)],
                    loc: member_loc,
                });
                properties.push(PropertyDecl {
                    name: member_name,
                    modifiers: member_mods,
                    ty: member_ty,
                    getter,
                    setter: None,
                    loc: member_loc,
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
                    modifiers: member_mods.clone(),
                    ty: member_ty.clone(),
                    init,
                    loc: member_loc,
                });

                while self.match_tok(&Tok::Comma) {
                    if let Ok(next_name) = self.expect_ident() {
                        let next_init = if self.match_tok(&Tok::Assign) {
                            self.parse_expr().ok()
                        } else {
                            None
                        };
                        fields.push(FieldDecl {
                            name: next_name,
                            modifiers: member_mods.clone(),
                            ty: member_ty.clone(),
                            init: next_init,
                            loc: member_loc,
                        });
                    } else {
                        break;
                    }
                }
                let _ = self.match_tok(&Tok::Semicolon);
            }
        }

        let _ = self.match_tok(&Tok::RBrace);

        Some(TypeDecl {
            name,
            kind,
            modifiers,
            type_params,
            base_types,
            methods,
            constructors,
            properties,
            fields,
            nested_types,
            loc,
        })
    }

    fn parse_property_accessors(&mut self) -> (Option<Block>, Option<Block>) {
        if !self.match_tok(&Tok::LBrace) {
            return (None, None);
        }
        let mut getter = None;
        let mut setter = None;

        while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
            let _acc_mods = self.parse_modifiers();
            let loc = self.loc();
            if self.match_tok(&Tok::Get) {
                if self.check(&Tok::LBrace) {
                    getter = Some(self.parse_block());
                } else if self.match_tok(&Tok::Arrow) {
                    let expr = self.parse_expr().ok();
                    let _ = self.match_tok(&Tok::Semicolon);
                    getter = expr.map(|e| Block {
                        stmts: vec![Stmt::Return(Some(e), loc)],
                        loc,
                    });
                } else {
                    let _ = self.match_tok(&Tok::Semicolon);
                }
            } else if self.match_tok(&Tok::Set) || self.match_tok(&Tok::Init) {
                if self.check(&Tok::LBrace) {
                    setter = Some(self.parse_block());
                } else if self.match_tok(&Tok::Arrow) {
                    let expr = self.parse_expr().ok();
                    let _ = self.match_tok(&Tok::Semicolon);
                    setter = expr.map(|e| Block {
                        stmts: vec![Stmt::Expr(e, loc)],
                        loc,
                    });
                } else {
                    let _ = self.match_tok(&Tok::Semicolon);
                }
            } else {
                self.advance();
            }
        }
        let _ = self.match_tok(&Tok::RBrace);
        // Optional property initializer: = expr;
        if self.match_tok(&Tok::Assign) {
            let _ = self.parse_expr();
            let _ = self.match_tok(&Tok::Semicolon);
        }
        (getter, setter)
    }

    fn skip_brackets(&mut self) {
        if self.match_tok(&Tok::LBracket) {
            let mut depth = 1;
            while depth > 0 && !self.check(&Tok::Eof) {
                match self.advance().kind {
                    Tok::LBracket => depth += 1,
                    Tok::RBracket => depth -= 1,
                    _ => {}
                }
            }
        }
    }

    fn skip_operator_tokens(&mut self) {
        match self.peek() {
            Tok::Plus
            | Tok::Minus
            | Tok::Star
            | Tok::Slash
            | Tok::Percent
            | Tok::Amp
            | Tok::Pipe
            | Tok::Caret
            | Tok::Bang
            | Tok::BitNot
            | Tok::Eq
            | Tok::Ne
            | Tok::Lt
            | Tok::Gt
            | Tok::Le
            | Tok::Ge
            | Tok::PlusPlus
            | Tok::MinusMinus
            | Tok::Shl
            | Tok::Shr => {
                self.advance();
            }
            _ => {}
        }
    }

    fn skip_type_params(&mut self) -> Vec<String> {
        let mut params = Vec::new();
        if self.match_tok(&Tok::Lt) {
            let mut depth = 1;
            while depth > 0 && !self.check(&Tok::Eof) {
                match self.peek() {
                    Tok::Lt => {
                        depth += 1;
                        self.advance();
                    }
                    Tok::Gt => {
                        depth -= 1;
                        self.advance();
                    }
                    Tok::Ident(s) => {
                        if depth == 1 {
                            params.push(s.clone());
                        }
                        self.advance();
                    }
                    _ => {
                        self.advance();
                    }
                }
            }
        }
        params
    }

    fn skip_where_clauses(&mut self) {
        while let Tok::Ident(ref id) = self.peek() {
            if id == "where" {
                self.advance();
                while !self.check(&Tok::LBrace)
                    && !self.check(&Tok::Semicolon)
                    && !self.check(&Tok::Arrow)
                    && !self.check(&Tok::Eof)
                {
                    if let Tok::Ident(ref next_id) = self.peek() {
                        if next_id == "where" {
                            break;
                        }
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn sync_to_next_member(&mut self) {
        while !self.check(&Tok::Eof) && !self.check(&Tok::RBrace) {
            if self.match_tok(&Tok::Semicolon) {
                break;
            }
            if self.check(&Tok::LBrace) {
                self.skip_to_matching_rbrace();
                break;
            }
            self.advance();
        }
    }

    fn skip_to_matching_rbrace(&mut self) {
        if self.match_tok(&Tok::LBrace) {
            let mut depth = 1;
            while depth > 0 && !self.check(&Tok::Eof) {
                match self.advance().kind {
                    Tok::LBrace => depth += 1,
                    Tok::RBrace => depth -= 1,
                    _ => {}
                }
            }
        }
    }

    pub fn parse_type(&mut self) -> Result<CSharpType, String> {
        let mut ty = match self.peek() {
            Tok::IntKw => {
                self.advance();
                CSharpType::new("int")
            }
            Tok::Long => {
                self.advance();
                CSharpType::new("long")
            }
            Tok::Short => {
                self.advance();
                CSharpType::new("short")
            }
            Tok::Byte => {
                self.advance();
                CSharpType::new("byte")
            }
            Tok::Sbyte => {
                self.advance();
                CSharpType::new("sbyte")
            }
            Tok::Uint => {
                self.advance();
                CSharpType::new("uint")
            }
            Tok::Ulong => {
                self.advance();
                CSharpType::new("ulong")
            }
            Tok::Ushort => {
                self.advance();
                CSharpType::new("ushort")
            }
            Tok::FloatKw => {
                self.advance();
                CSharpType::new("float")
            }
            Tok::Double => {
                self.advance();
                CSharpType::new("double")
            }
            Tok::Decimal => {
                self.advance();
                CSharpType::new("decimal")
            }
            Tok::Bool => {
                self.advance();
                CSharpType::new("bool")
            }
            Tok::CharKw => {
                self.advance();
                CSharpType::new("char")
            }
            Tok::Void => {
                self.advance();
                CSharpType::new("void")
            }
            Tok::StringKw => {
                self.advance();
                CSharpType::new("string")
            }
            Tok::Object => {
                self.advance();
                CSharpType::new("object")
            }
            Tok::Var => {
                self.advance();
                CSharpType::new("var")
            }
            Tok::Ident(_) => {
                let name = self.parse_qualified_name();
                CSharpType::new(&name)
            }
            Tok::LParen => {
                // Tuple type: (int, string)
                self.advance();
                let mut tuple_parts = Vec::new();
                while !self.check(&Tok::RParen) && !self.check(&Tok::Eof) {
                    if let Ok(part) = self.parse_type() {
                        tuple_parts.push(part);
                    }
                    let _ = self.expect_ident(); // optional tuple name
                    if !self.match_tok(&Tok::Comma) {
                        break;
                    }
                }
                let _ = self.match_tok(&Tok::RParen);
                CSharpType {
                    name: "ValueTuple".into(),
                    is_primitive: false,
                    is_nullable: false,
                    is_span: false,
                    array_dims: 0,
                    generic_args: tuple_parts,
                }
            }
            other => {
                return Err(format!(
                    "expected type, got {:?} at line {}",
                    other,
                    self.loc().line
                ))
            }
        };

        // Generics: <T, U>
        if self.match_tok(&Tok::Lt) {
            let mut args = Vec::new();
            while !self.check(&Tok::Gt) && !self.check(&Tok::Eof) {
                if let Ok(arg) = self.parse_type() {
                    args.push(arg);
                }
                if !self.match_tok(&Tok::Comma) {
                    break;
                }
            }
            let _ = self.match_tok(&Tok::Gt);
            if ty.name == "Span" || ty.name == "ReadOnlySpan" {
                ty.is_span = true;
            }
            ty.generic_args = args;
        }

        // Nullable modifier: ?
        if self.match_tok(&Tok::Question) {
            ty.is_nullable = true;
        }

        // Array brackets: [] or [,]
        while self.check(&Tok::LBracket)
            && (self.peek_next() == &Tok::RBracket || self.peek_next() == &Tok::Comma)
        {
            self.advance();
            let mut dims = 1;
            while self.match_tok(&Tok::Comma) {
                dims += 1;
            }
            let _ = self.match_tok(&Tok::RBracket);
            ty.array_dims += dims;
            // Array can also have ? after: int[]?
            if self.match_tok(&Tok::Question) {
                ty.is_nullable = true;
            }
        }

        Ok(ty)
    }

    fn parse_param_list(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        if !self.match_tok(&Tok::LParen) {
            return params;
        }

        while !self.check(&Tok::RParen) && !self.check(&Tok::Eof) {
            let loc = self.loc();
            let mut mods = Vec::new();

            // Param attributes: [NotNull]
            if self.check(&Tok::LBracket) {
                self.skip_attribute();
            }

            loop {
                match self.peek() {
                    Tok::Ref => {
                        self.advance();
                        mods.push("ref".into());
                    }
                    Tok::Out => {
                        self.advance();
                        mods.push("out".into());
                    }
                    Tok::In => {
                        self.advance();
                        mods.push("in".into());
                    }
                    Tok::Params => {
                        self.advance();
                        mods.push("params".into());
                    }
                    Tok::This => {
                        self.advance();
                        mods.push("this".into());
                    }
                    _ => break,
                }
            }

            let ty = match self.parse_type() {
                Ok(t) => t,
                Err(_) => {
                    self.sync_to_param_end();
                    continue;
                }
            };

            let name = match self.expect_ident() {
                Ok(n) => n,
                Err(_) => format!("arg{}", params.len()),
            };

            let default_val = if self.match_tok(&Tok::Assign) {
                self.parse_expr().ok()
            } else {
                None
            };

            params.push(Param {
                name,
                ty,
                modifiers: mods,
                default_val,
                loc,
            });

            if !self.match_tok(&Tok::Comma) {
                break;
            }
        }

        let _ = self.match_tok(&Tok::RParen);
        params
    }

    fn sync_to_param_end(&mut self) {
        while !self.check(&Tok::RParen) && !self.check(&Tok::Comma) && !self.check(&Tok::Eof) {
            self.advance();
        }
    }

    pub fn parse_block(&mut self) -> Block {
        let loc = self.loc();
        let mut stmts = Vec::new();

        if !self.match_tok(&Tok::LBrace) {
            return Block { stmts, loc };
        }

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
        Block { stmts, loc }
    }

    fn sync_to_next_stmt(&mut self) {
        while !self.check(&Tok::Eof) && !self.check(&Tok::RBrace) {
            if self.match_tok(&Tok::Semicolon) {
                break;
            }
            if self.check(&Tok::LBrace) {
                self.skip_to_matching_rbrace();
                break;
            }
            self.advance();
        }
    }

    pub fn parse_stmt(&mut self) -> Result<Stmt, String> {
        let loc = self.loc();

        if self.check(&Tok::LBrace) {
            return Ok(Stmt::Block(self.parse_block()));
        }

        if self.match_tok(&Tok::If) {
            if !self.match_tok(&Tok::LParen) {
                return Err("expected '(' after if".into());
            }
            let cond = self.parse_expr()?;
            let _ = self.match_tok(&Tok::RParen);
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
                loc,
            });
        }

        if self.match_tok(&Tok::While) {
            if !self.match_tok(&Tok::LParen) {
                return Err("expected '(' after while".into());
            }
            let cond = self.parse_expr()?;
            let _ = self.match_tok(&Tok::RParen);
            let body = Box::new(self.parse_stmt()?);
            return Ok(Stmt::While { cond, body, loc });
        }

        if self.match_tok(&Tok::Do) {
            let body = Box::new(self.parse_stmt()?);
            let _ = self.match_tok(&Tok::While);
            let _ = self.match_tok(&Tok::LParen);
            let cond = self.parse_expr()?;
            let _ = self.match_tok(&Tok::RParen);
            let _ = self.match_tok(&Tok::Semicolon);
            return Ok(Stmt::DoWhile { body, cond, loc });
        }

        if self.match_tok(&Tok::For) {
            if !self.match_tok(&Tok::LParen) {
                return Err("expected '(' after for".into());
            }
            let init = if self.match_tok(&Tok::Semicolon) {
                None
            } else {
                let s = self.parse_for_init()?;
                Some(s)
            };
            let cond = if self.match_tok(&Tok::Semicolon) {
                None
            } else {
                let c = self.parse_expr().ok();
                let _ = self.match_tok(&Tok::Semicolon);
                c
            };
            let update = if self.check(&Tok::RParen) {
                None
            } else {
                let mut exprs = Vec::new();
                loop {
                    if let Ok(e) = self.parse_expr() {
                        exprs.push(e);
                    }
                    if !self.match_tok(&Tok::Comma) {
                        break;
                    }
                }
                Some(exprs)
            };
            let _ = self.match_tok(&Tok::RParen);
            let body = Box::new(self.parse_stmt()?);
            return Ok(Stmt::For {
                init,
                cond,
                update,
                body,
                loc,
            });
        }

        if self.match_tok(&Tok::ForEach) {
            if !self.match_tok(&Tok::LParen) {
                return Err("expected '(' after foreach".into());
            }
            let var_ty = self.parse_type().unwrap_or_else(|_| CSharpType::new("var"));
            let var_name = self.expect_ident().unwrap_or_else(|_| "item".into());
            let _ = self.match_tok(&Tok::In);
            let iterable = self.parse_expr()?;
            let _ = self.match_tok(&Tok::RParen);
            let body = Box::new(self.parse_stmt()?);
            return Ok(Stmt::ForEach {
                var_name,
                var_ty,
                iterable,
                body,
                loc,
            });
        }

        if self.match_tok(&Tok::Switch) {
            if !self.match_tok(&Tok::LParen) {
                return Err("expected '(' after switch".into());
            }
            let selector = self.parse_expr()?;
            let _ = self.match_tok(&Tok::RParen);
            let cases = self.parse_switch_body();
            return Ok(Stmt::Switch {
                selector,
                cases,
                loc,
            });
        }

        if self.match_tok(&Tok::Return) {
            let expr = if self.check(&Tok::Semicolon) {
                None
            } else {
                self.parse_expr().ok()
            };
            let _ = self.match_tok(&Tok::Semicolon);
            return Ok(Stmt::Return(expr, loc));
        }

        if self.match_tok(&Tok::Throw) {
            let expr = self.parse_expr()?;
            let _ = self.match_tok(&Tok::Semicolon);
            return Ok(Stmt::Throw(expr, loc));
        }

        if self.match_tok(&Tok::Try) {
            let body = self.parse_block();
            let mut catches = Vec::new();
            while self.match_tok(&Tok::Catch) {
                let catch_loc = self.loc();
                let mut param_name = None;
                let mut exception_ty = None;
                if self.match_tok(&Tok::LParen) {
                    if let Ok(ty) = self.parse_type() {
                        exception_ty = Some(ty);
                        if let Ok(name) = self.expect_ident() {
                            param_name = Some(name);
                        }
                    }
                    let _ = self.match_tok(&Tok::RParen);
                }
                let catch_body = self.parse_block();
                catches.push(CatchClause {
                    param_name,
                    exception_ty,
                    body: catch_body,
                    loc: catch_loc,
                });
            }
            let finally_body = if self.match_tok(&Tok::Finally) {
                Some(self.parse_block())
            } else {
                None
            };
            return Ok(Stmt::Try {
                body,
                catches,
                finally_body,
                loc,
            });
        }

        if self.match_tok(&Tok::Using) {
            if self.match_tok(&Tok::LParen) {
                // using (var x = ...) { }
                let init = if self.is_variable_decl() {
                    Some(Box::new(self.parse_var_decl_stmt()?))
                } else {
                    None
                };
                let expr = if init.is_none() {
                    self.parse_expr().ok()
                } else {
                    None
                };
                let _ = self.match_tok(&Tok::RParen);
                let body = Box::new(self.parse_stmt()?);
                return Ok(Stmt::Using {
                    init,
                    expr,
                    body,
                    loc,
                });
            } else {
                // C# 8 using var x = ...;
                let var_decl = self.parse_var_decl_stmt()?;
                return Ok(var_decl);
            }
        }

        if self.match_tok(&Tok::Lock) {
            let _ = self.match_tok(&Tok::LParen);
            let lock_expr = self.parse_expr()?;
            let _ = self.match_tok(&Tok::RParen);
            let body = self.parse_block();
            return Ok(Stmt::Lock {
                lock_expr,
                body,
                loc,
            });
        }

        if self.match_tok(&Tok::Break) {
            let _ = self.match_tok(&Tok::Semicolon);
            return Ok(Stmt::Break(loc));
        }

        if self.match_tok(&Tok::Continue) {
            let _ = self.match_tok(&Tok::Semicolon);
            return Ok(Stmt::Continue(loc));
        }

        // Variable declaration vs expression statement
        if self.is_variable_decl() {
            return self.parse_var_decl_stmt();
        }

        // Expression statement
        let expr = self.parse_expr()?;
        let _ = self.match_tok(&Tok::Semicolon);
        Ok(Stmt::Expr(expr, loc))
    }

    fn is_variable_decl(&self) -> bool {
        match self.peek() {
            Tok::Var => true,
            Tok::IntKw
            | Tok::Long
            | Tok::Short
            | Tok::Byte
            | Tok::Sbyte
            | Tok::Uint
            | Tok::Ulong
            | Tok::Ushort
            | Tok::FloatKw
            | Tok::Double
            | Tok::Decimal
            | Tok::Bool
            | Tok::CharKw
            | Tok::StringKw
            | Tok::Object => {
                matches!(
                    self.peek_next(),
                    Tok::Ident(_) | Tok::Question | Tok::LBracket | Tok::Lt
                )
            }
            Tok::Ident(_) => {
                // If followed by identifier, it's `Type varName`
                if matches!(self.peek_next(), Tok::Ident(_)) {
                    return true;
                }
                // If followed by `?`, `<`, `[]` followed by ident
                if matches!(
                    self.peek_next(),
                    Tok::Question | Tok::Lt | Tok::LBracket
                ) {
                    // Lookahead further
                    let mut i = self.pos + 1;
                    let mut depth = 0;
                    while i < self.tokens.len() {
                        match &self.tokens[i].kind {
                            Tok::Lt | Tok::LBracket => depth += 1,
                            Tok::Gt | Tok::RBracket => {
                                if depth > 0 {
                                    depth -= 1;
                                }
                            }
                            Tok::Question => {}
                            Tok::Ident(_) if depth == 0 => return true,
                            _ if depth == 0 => return false,
                            _ => {}
                        }
                        i += 1;
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn parse_var_decl_stmt(&mut self) -> Result<Stmt, String> {
        let loc = self.loc();
        let ty = self.parse_type()?;
        let mut vars = Vec::new();

        loop {
            let name = self.expect_ident()?;
            let init = if self.match_tok(&Tok::Assign) {
                self.parse_expr().ok()
            } else {
                None
            };
            vars.push((name, init));
            if !self.match_tok(&Tok::Comma) {
                break;
            }
        }
        let _ = self.match_tok(&Tok::Semicolon);
        Ok(Stmt::VarDecl { ty, vars, loc })
    }

    fn parse_for_init(&mut self) -> Result<Vec<Stmt>, String> {
        if self.is_variable_decl() {
            let s = self.parse_var_decl_stmt()?;
            Ok(vec![s])
        } else {
            let mut exprs = Vec::new();
            loop {
                let loc = self.loc();
                let e = self.parse_expr()?;
                exprs.push(Stmt::Expr(e, loc));
                if !self.match_tok(&Tok::Comma) {
                    break;
                }
            }
            let _ = self.match_tok(&Tok::Semicolon);
            Ok(exprs)
        }
    }

    fn parse_switch_body(&mut self) -> Vec<SwitchCase> {
        let mut cases = Vec::new();
        if !self.match_tok(&Tok::LBrace) {
            return cases;
        }

        while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
            let loc = self.loc();
            let mut labels = Vec::new();

            while self.check(&Tok::Case) || self.check(&Tok::Default) {
                if self.match_tok(&Tok::Case) {
                    if let Ok(e) = self.parse_expr() {
                        labels.push(e);
                    }
                    let _ = self.match_tok(&Tok::Colon);
                } else if self.match_tok(&Tok::Default) {
                    labels.push(Expr::Ident("default".into(), loc));
                    let _ = self.match_tok(&Tok::Colon);
                }
            }

            let mut stmts = Vec::new();
            while !self.check(&Tok::Case)
                && !self.check(&Tok::Default)
                && !self.check(&Tok::RBrace)
                && !self.check(&Tok::Eof)
            {
                if let Ok(s) = self.parse_stmt() {
                    stmts.push(s);
                } else {
                    self.sync_to_next_stmt();
                }
            }

            cases.push(SwitchCase { labels, stmts, loc });
        }

        let _ = self.match_tok(&Tok::RBrace);
        cases
    }

    // ─── EXPRESSION PARSING ──────────────────────────────────────────────────

    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, String> {
        let expr = self.parse_ternary()?;

        let op = if self.match_tok(&Tok::Assign) {
            AssignOp::Assign
        } else if self.match_tok(&Tok::PlusAssign) {
            AssignOp::AddAssign
        } else if self.match_tok(&Tok::MinusAssign) {
            AssignOp::SubAssign
        } else if self.match_tok(&Tok::MulAssign) {
            AssignOp::MulAssign
        } else if self.match_tok(&Tok::DivAssign) {
            AssignOp::DivAssign
        } else if self.match_tok(&Tok::RemAssign) {
            AssignOp::RemAssign
        } else if self.match_tok(&Tok::AmpAssign) {
            AssignOp::BitAndAssign
        } else if self.match_tok(&Tok::PipeAssign) {
            AssignOp::BitOrAssign
        } else if self.match_tok(&Tok::CaretAssign) {
            AssignOp::BitXorAssign
        } else if self.match_tok(&Tok::ShlAssign) {
            AssignOp::ShlAssign
        } else if self.match_tok(&Tok::ShrAssign) {
            AssignOp::ShrAssign
        } else if self.match_tok(&Tok::QuestionQuestionAssign) {
            AssignOp::NullCoalescingAssign
        } else {
            return Ok(expr);
        };

        let loc = expr.loc();
        let val = self.parse_assignment()?;
        Ok(Expr::Assign {
            op,
            target: Box::new(expr),
            value: Box::new(val),
            loc,
        })
    }

    fn parse_ternary(&mut self) -> Result<Expr, String> {
        let expr = self.parse_null_coalescing()?;

        if self.match_tok(&Tok::Question) {
            let loc = expr.loc();
            let then_expr = self.parse_expr()?;
            let _ = self.match_tok(&Tok::Colon);
            let else_expr = self.parse_ternary()?;
            return Ok(Expr::Ternary {
                cond: Box::new(expr),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
                loc,
            });
        }

        Ok(expr)
    }

    fn parse_null_coalescing(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_logical_or()?;

        while self.match_tok(&Tok::QuestionQuestion) {
            let loc = expr.loc();
            let rhs = self.parse_logical_or()?;
            expr = Expr::NullCoalescing {
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                loc,
            };
        }

        Ok(expr)
    }

    fn parse_logical_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_logical_and()?;

        while self.match_tok(&Tok::Or) {
            let loc = expr.loc();
            let right = self.parse_logical_and()?;
            expr = Expr::Binary {
                op: BinaryOp::Or,
                left: Box::new(expr),
                right: Box::new(right),
                loc,
            };
        }

        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_bitwise_or()?;

        while self.match_tok(&Tok::And) {
            let loc = expr.loc();
            let right = self.parse_bitwise_or()?;
            expr = Expr::Binary {
                op: BinaryOp::And,
                left: Box::new(expr),
                right: Box::new(right),
                loc,
            };
        }

        Ok(expr)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_bitwise_xor()?;

        while self.match_tok(&Tok::Pipe) {
            let loc = expr.loc();
            let right = self.parse_bitwise_xor()?;
            expr = Expr::Binary {
                op: BinaryOp::BitOr,
                left: Box::new(expr),
                right: Box::new(right),
                loc,
            };
        }

        Ok(expr)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_bitwise_and()?;

        while self.match_tok(&Tok::Caret) {
            let loc = expr.loc();
            let right = self.parse_bitwise_and()?;
            expr = Expr::Binary {
                op: BinaryOp::BitXor,
                left: Box::new(expr),
                right: Box::new(right),
                loc,
            };
        }

        Ok(expr)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_equality()?;

        while self.match_tok(&Tok::Amp) {
            let loc = expr.loc();
            let right = self.parse_equality()?;
            expr = Expr::Binary {
                op: BinaryOp::BitAnd,
                left: Box::new(expr),
                right: Box::new(right),
                loc,
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_relational()?;

        loop {
            let op = if self.match_tok(&Tok::Eq) {
                BinaryOp::Eq
            } else if self.match_tok(&Tok::BangEqual) {
                BinaryOp::Ne
            } else {
                break;
            };

            let loc = expr.loc();
            let right = self.parse_relational()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                loc,
            };
        }

        Ok(expr)
    }

    fn parse_relational(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_shift()?;

        loop {
            let loc = expr.loc();

            // is / as checks
            if self.match_tok(&Tok::Is) {
                let is_negated = self.match_tok(&Tok::Not);
                if self.match_tok(&Tok::Null) {
                    let pattern_ty = CSharpType::new("null");
                    expr = Expr::IsPattern {
                        expr: Box::new(expr),
                        pattern_ty,
                        pattern_var: None,
                        is_negated,
                        loc,
                    };
                    continue;
                }
                if let Ok(target_ty) = self.parse_type() {
                    let pattern_var = self.expect_ident().ok();
                    expr = Expr::IsPattern {
                        expr: Box::new(expr),
                        pattern_ty: target_ty,
                        pattern_var,
                        is_negated,
                        loc,
                    };
                    continue;
                }
            }

            if self.match_tok(&Tok::As) {
                if let Ok(target_ty) = self.parse_type() {
                    expr = Expr::AsCast {
                        expr: Box::new(expr),
                        target_ty,
                        loc,
                    };
                    continue;
                }
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

            let right = self.parse_shift()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                loc,
            };
        }

        Ok(expr)
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_additive()?;

        loop {
            let op = if self.match_tok(&Tok::Shl) {
                BinaryOp::Shl
            } else if self.match_tok(&Tok::Shr) {
                BinaryOp::Shr
            } else {
                break;
            };

            let loc = expr.loc();
            let right = self.parse_additive()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                loc,
            };
        }

        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_multiplicative()?;

        loop {
            let op = if self.match_tok(&Tok::Plus) {
                BinaryOp::Add
            } else if self.match_tok(&Tok::Minus) {
                BinaryOp::Sub
            } else {
                break;
            };

            let loc = expr.loc();
            let right = self.parse_multiplicative()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                loc,
            };
        }

        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_unary()?;

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

            let loc = expr.loc();
            let right = self.parse_unary()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                loc,
            };
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        let loc = self.loc();

        if self.match_tok(&Tok::Bang) {
            let inner = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(inner),
                loc,
            });
        }

        if self.match_tok(&Tok::BitNot) {
            let inner = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::BitNot,
                expr: Box::new(inner),
                loc,
            });
        }

        if self.match_tok(&Tok::Minus) {
            let inner = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Minus,
                expr: Box::new(inner),
                loc,
            });
        }

        if self.match_tok(&Tok::Plus) {
            let inner = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Plus,
                expr: Box::new(inner),
                loc,
            });
        }

        if self.match_tok(&Tok::PlusPlus) {
            let inner = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::PreInc,
                expr: Box::new(inner),
                loc,
            });
        }

        if self.match_tok(&Tok::MinusMinus) {
            let inner = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::PreDec,
                expr: Box::new(inner),
                loc,
            });
        }

        // Cast: (Type)expr
        if self.is_cast() {
            let _ = self.match_tok(&Tok::LParen);
            let target_ty = self.parse_type()?;
            let _ = self.match_tok(&Tok::RParen);
            let inner = self.parse_unary()?;
            return Ok(Expr::Cast {
                target_ty,
                expr: Box::new(inner),
                loc,
            });
        }

        self.parse_postfix()
    }

    fn is_cast(&self) -> bool {
        if self.peek() != &Tok::LParen {
            return false;
        }

        // Check if inside parentheses is a type name followed by ')' and then an expression starter
        let mut i = self.pos + 1;
        let mut depth = 0;
        let mut found_rparen = None;

        while i < self.tokens.len() {
            match &self.tokens[i].kind {
                Tok::LParen => depth += 1,
                Tok::RParen => {
                    if depth == 0 {
                        found_rparen = Some(i);
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
            i += 1;
        }

        let Some(rparen_idx) = found_rparen else { return false };
        if rparen_idx + 1 >= self.tokens.len() { return false; }

        let after = &self.tokens[rparen_idx + 1].kind;
        let after_is_expr_start = matches!(
            after,
            Tok::Ident(_)
                | Tok::Int(_)
                | Tok::Float(_)
                | Tok::Str(_)
                | Tok::Char(_)
                | Tok::True
                | Tok::False
                | Tok::Null
                | Tok::This
                | Tok::Base
                | Tok::New
                | Tok::LParen
                | Tok::Bang
                | Tok::BitNot
        );

        if !after_is_expr_start {
            return false;
        }

        // Check if tokens between pos+1 and rparen look like a type
        let first_inside = &self.tokens[self.pos + 1].kind;
        matches!(
            first_inside,
            Tok::IntKw
                | Tok::Long
                | Tok::Short
                | Tok::Byte
                | Tok::Sbyte
                | Tok::Uint
                | Tok::Ulong
                | Tok::Ushort
                | Tok::FloatKw
                | Tok::Double
                | Tok::Decimal
                | Tok::Bool
                | Tok::CharKw
                | Tok::StringKw
                | Tok::Object
                | Tok::Ident(_)
        )
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            let loc = expr.loc();

            // Null-conditional member access: ?.Member
            if self.match_tok(&Tok::QuestionDot) {
                let member = self.expect_ident()?;
                if self.check(&Tok::LParen) {
                    let args = self.parse_argument_list();
                    expr = Expr::MethodCall {
                        target: Some(Box::new(expr)),
                        method: member,
                        args,
                        is_null_conditional: true,
                        loc,
                    };
                } else {
                    expr = Expr::MemberAccess {
                        target: Box::new(expr),
                        member,
                        is_null_conditional: true,
                        loc,
                    };
                }
                continue;
            }

            // Null-conditional element access: ?[Index]
            if self.match_tok(&Tok::QuestionBracket) {
                let index = self.parse_expr()?;
                let _ = self.match_tok(&Tok::RBracket);
                expr = Expr::ElementAccess {
                    target: Box::new(expr),
                    index: Box::new(index),
                    is_null_conditional: true,
                    loc,
                };
                continue;
            }

            // Member access or method call: .Member
            if self.match_tok(&Tok::Dot) {
                let member = self.expect_ident()?;
                if self.check(&Tok::LParen) {
                    let args = self.parse_argument_list();
                    expr = Expr::MethodCall {
                        target: Some(Box::new(expr)),
                        method: member,
                        args,
                        is_null_conditional: false,
                        loc,
                    };
                } else {
                    expr = Expr::MemberAccess {
                        target: Box::new(expr),
                        member,
                        is_null_conditional: false,
                        loc,
                    };
                }
                continue;
            }

            // Element access: [Index]
            if self.match_tok(&Tok::LBracket) {
                let index = self.parse_expr()?;
                let _ = self.match_tok(&Tok::RBracket);
                expr = Expr::ElementAccess {
                    target: Box::new(expr),
                    index: Box::new(index),
                    is_null_conditional: false,
                    loc,
                };
                continue;
            }

            // Method invocation on primary: foo(...)
            if self.check(&Tok::LParen) {
                if let Expr::Ident(name, l) = expr {
                    let args = self.parse_argument_list();
                    expr = Expr::MethodCall {
                        target: None,
                        method: name,
                        args,
                        is_null_conditional: false,
                        loc: l,
                    };
                    continue;
                }
            }

            // Postfix ++
            if self.match_tok(&Tok::PlusPlus) {
                expr = Expr::Unary {
                    op: UnaryOp::PostInc,
                    expr: Box::new(expr),
                    loc,
                };
                continue;
            }

            // Postfix --
            if self.match_tok(&Tok::MinusMinus) {
                expr = Expr::Unary {
                    op: UnaryOp::PostDec,
                    expr: Box::new(expr),
                    loc,
                };
                continue;
            }

            // Null-forgiving operator: !
            if self.match_tok(&Tok::Bang) {
                expr = Expr::Unary {
                    op: UnaryOp::NullForgiving,
                    expr: Box::new(expr),
                    loc,
                };
                continue;
            }

            break;
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let loc = self.loc();

        if self.match_tok(&Tok::Null) {
            return Ok(Expr::Null(loc));
        }

        if self.match_tok(&Tok::True) {
            return Ok(Expr::Bool(true, loc));
        }

        if self.match_tok(&Tok::False) {
            return Ok(Expr::Bool(false, loc));
        }

        if let Tok::Int(val) = self.peek() {
            let val = *val;
            self.advance();
            return Ok(Expr::Int(val, loc));
        }

        if let Tok::Float(val) = self.peek() {
            let val = *val;
            self.advance();
            return Ok(Expr::Float(val, loc));
        }

        if let Tok::Str(s) = self.peek() {
            let s = s.clone();
            self.advance();
            return Ok(Expr::StringLit(s, loc));
        }

        if let Tok::Char(c) = self.peek() {
            let c = *c;
            self.advance();
            return Ok(Expr::CharLit(c, loc));
        }

        if self.match_tok(&Tok::This) {
            return Ok(Expr::This(loc));
        }

        if self.match_tok(&Tok::Base) {
            return Ok(Expr::Base(loc));
        }

        if self.match_tok(&Tok::Default) {
            let ty = if self.match_tok(&Tok::LParen) {
                let t = self.parse_type().ok();
                let _ = self.match_tok(&Tok::RParen);
                t
            } else {
                None
            };
            return Ok(Expr::Default(ty, loc));
        }

        // New object or array
        if self.match_tok(&Tok::New) {
            return self.parse_new_expr(loc);
        }

        // Lambda or parenthesized expr / tuple
        if self.match_tok(&Tok::LParen) {
            if self.match_tok(&Tok::RParen) {
                // () => expr
                if self.match_tok(&Tok::Arrow) {
                    let body = self.parse_expr()?;
                    return Ok(Expr::Lambda {
                        params: Vec::new(),
                        body_expr: Some(Box::new(body)),
                        body_block: None,
                        loc,
                    });
                }
                return Ok(Expr::Tuple(Vec::new(), loc));
            }

            let first = self.parse_expr()?;
            if self.match_tok(&Tok::Comma) {
                // Tuple: (a, b, ...)
                let mut parts = vec![first];
                loop {
                    parts.push(self.parse_expr()?);
                    if !self.match_tok(&Tok::Comma) {
                        break;
                    }
                }
                let _ = self.match_tok(&Tok::RParen);
                return Ok(Expr::Tuple(parts, loc));
            }

            let _ = self.match_tok(&Tok::RParen);

            // Check if lambda: (x) => ...
            if self.match_tok(&Tok::Arrow) {
                let param_name = match first {
                    Expr::Ident(name, _) => name,
                    _ => "arg".into(),
                };
                if self.check(&Tok::LBrace) {
                    let body = self.parse_block();
                    return Ok(Expr::Lambda {
                        params: vec![param_name],
                        body_expr: None,
                        body_block: Some(body),
                        loc,
                    });
                } else {
                    let body = self.parse_expr()?;
                    return Ok(Expr::Lambda {
                        params: vec![param_name],
                        body_expr: Some(Box::new(body)),
                        body_block: None,
                        loc,
                    });
                }
            }

            return Ok(first);
        }

        // Identifier or single-param lambda: x => expr
        if let Tok::Ident(name) = self.peek() {
            let name = name.clone();
            self.advance();

            if self.match_tok(&Tok::Arrow) {
                if self.check(&Tok::LBrace) {
                    let body = self.parse_block();
                    return Ok(Expr::Lambda {
                        params: vec![name],
                        body_expr: None,
                        body_block: Some(body),
                        loc,
                    });
                } else {
                    let body = self.parse_expr()?;
                    return Ok(Expr::Lambda {
                        params: vec![name],
                        body_expr: Some(Box::new(body)),
                        body_block: None,
                        loc,
                    });
                }
            }

            return Ok(Expr::Ident(name, loc));
        }

        Err(format!(
            "unexpected token {:?} in expression at line {}",
            self.peek(),
            loc.line
        ))
    }

    fn parse_new_expr(&mut self, loc: SourceLoc) -> Result<Expr, String> {
        // Target type can be target-typed new: new() or new [...] or new Type(...)
        if self.check(&Tok::LParen) {
            let args = self.parse_argument_list();
            return Ok(Expr::NewObject {
                ty: CSharpType::new("object"),
                args,
                loc,
            });
        }

        if self.check(&Tok::LBracket) {
            // Target-typed array: new[] { 1, 2, 3 }
            self.advance(); // [
            let _ = self.match_tok(&Tok::RBracket);
            let inits = self.parse_array_initializer();
            return Ok(Expr::NewArray {
                element_ty: CSharpType::new("var"),
                dims: Vec::new(),
                initializer: inits,
                loc,
            });
        }

        let ty = self.parse_type()?;

        // Array creation: new int[5] or new int[] { 1, 2 }
        if ty.array_dims > 0 || self.check(&Tok::LBracket) {
            let mut dims = Vec::new();
            if self.match_tok(&Tok::LBracket) {
                if !self.check(&Tok::RBracket) {
                    dims.push(self.parse_expr()?);
                }
                let _ = self.match_tok(&Tok::RBracket);
            }
            let initializer = if self.check(&Tok::LBrace) {
                self.parse_array_initializer()
            } else {
                None
            };
            return Ok(Expr::NewArray {
                element_ty: ty,
                dims,
                initializer,
                loc,
            });
        }

        // Object creation: new Foo(args) { Initializers }
        let args = if self.check(&Tok::LParen) {
            self.parse_argument_list()
        } else {
            Vec::new()
        };

        // Object initializer { A = 1, B = 2 }
        if self.check(&Tok::LBrace) {
            self.skip_to_matching_rbrace();
        }

        Ok(Expr::NewObject { ty, args, loc })
    }

    fn parse_array_initializer(&mut self) -> Option<Vec<Expr>> {
        if !self.match_tok(&Tok::LBrace) {
            return None;
        }
        let mut items = Vec::new();
        while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
            if let Ok(e) = self.parse_expr() {
                items.push(e);
            }
            if !self.match_tok(&Tok::Comma) {
                break;
            }
        }
        let _ = self.match_tok(&Tok::RBrace);
        Some(items)
    }

    fn parse_argument_list(&mut self) -> Vec<Expr> {
        let mut args = Vec::new();
        if !self.match_tok(&Tok::LParen) {
            return args;
        }

        while !self.check(&Tok::RParen) && !self.check(&Tok::Eof) {
            // Named argument: argName: expr
            if let Tok::Ident(_) = self.peek() {
                if self.peek_next() == &Tok::Colon {
                    self.advance();
                    self.advance();
                }
            }
            // ref/out modifiers: ref x
            let _ = self.match_tok(&Tok::Ref) || self.match_tok(&Tok::Out) || self.match_tok(&Tok::In);

            if let Ok(e) = self.parse_expr() {
                args.push(e);
            }
            if !self.match_tok(&Tok::Comma) {
                break;
            }
        }

        let _ = self.match_tok(&Tok::RParen);
        args
    }
}
