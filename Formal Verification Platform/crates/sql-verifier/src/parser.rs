//! SQL parser: converts raw SQL text into the AST.
//!
//! This is a hand-written recursive-descent parser sufficient for the subset
//! of SQL needed by the verifier. It handles SELECT, INSERT, UPDATE, DELETE,
//! CREATE TABLE, CTEs (including recursive), JOINs, subqueries, and common
//! aggregate functions.

use crate::ast::*;
use std::fmt;

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub position: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parse error at position {}: {}", self.position, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Token kinds for the SQL lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    Number(i64),
    Float(f64),
    StringLit(String),
    Keyword(String),
    Op(String),
    LParen,
    RParen,
    Comma,
    Semicolon,
    Star,
    Dot,
    EOF,
}

struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    fn new(sql: &str) -> Self {
        Self {
            input: sql.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        // Skip -- line comments and /* block comments */
        while let Some(ch) = self.peek() {
            if ch == '-' && self.input.get(self.pos + 1) == Some(&'-') {
                while let Some(c) = self.advance() {
                    if c == '\n' {
                        break;
                    }
                }
            } else if ch == '/' && self.input.get(self.pos + 1) == Some(&'*') {
                self.advance();
                self.advance();
                while let Some(c) = self.peek() {
                    if c == '*' && self.input.get(self.pos + 1) == Some(&'/') {
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
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            self.skip_comment();
            match self.peek() {
                None => {
                    tokens.push(Token::EOF);
                    break;
                }
                Some(ch) => {
                    if ch.is_ascii_digit() {
                        let start = self.pos;
                        let mut is_float = false;
                        while let Some(c) = self.peek() {
                            if c.is_ascii_digit() {
                                self.advance();
                            } else if c == '.' && !is_float {
                                is_float = true;
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        let num_str: String = self.input[start..self.pos].iter().collect();
                        if is_float {
                            tokens.push(Token::Float(num_str.parse().unwrap_or(0.0)));
                        } else {
                            tokens.push(Token::Number(num_str.parse().unwrap_or(0)));
                        }
                    } else if ch.is_ascii_alphabetic() || ch == '_' {
                        let start = self.pos;
                        while let Some(c) = self.peek() {
                            if c.is_ascii_alphanumeric() || c == '_' {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        let word: String = self.input[start..self.pos].iter().collect();
                        let upper = word.to_uppercase();
                        // Check for keywords
                        if is_keyword(&upper) {
                            tokens.push(Token::Keyword(upper));
                        } else {
                            tokens.push(Token::Ident(word));
                        }
                    } else if ch == '\'' {
                        self.advance();
                        let mut s = String::new();
                        while let Some(c) = self.peek() {
                            if c == '\'' {
                                // Check for escaped quote ''
                                if self.input.get(self.pos + 1) == Some(&'\'') {
                                    s.push('\'');
                                    self.advance();
                                    self.advance();
                                } else {
                                    self.advance();
                                    break;
                                }
                            } else {
                                s.push(c);
                                self.advance();
                            }
                        }
                        tokens.push(Token::StringLit(s));
                    } else if ch == '(' {
                        self.advance();
                        tokens.push(Token::LParen);
                    } else if ch == ')' {
                        self.advance();
                        tokens.push(Token::RParen);
                    } else if ch == ',' {
                        self.advance();
                        tokens.push(Token::Comma);
                    } else if ch == ';' {
                        self.advance();
                        tokens.push(Token::Semicolon);
                    } else if ch == '*' {
                        self.advance();
                        tokens.push(Token::Star);
                    } else if ch == '.' {
                        self.advance();
                        tokens.push(Token::Dot);
                    } else if ch == '=' {
                        self.advance();
                        tokens.push(Token::Op("=".into()));
                    } else if ch == '<' {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            tokens.push(Token::Op("<=".into()));
                        } else if self.peek() == Some('>') {
                            self.advance();
                            tokens.push(Token::Op("<>".into()));
                        } else {
                            tokens.push(Token::Op("<".into()));
                        }
                    } else if ch == '>' {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            tokens.push(Token::Op(">=".into()));
                        } else {
                            tokens.push(Token::Op(">".into()));
                        }
                    } else if ch == '!' {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            tokens.push(Token::Op("!=".into()));
                        } else {
                            return Err(ParseError {
                                message: "Unexpected '!'".into(),
                                position: self.pos,
                            });
                        }
                    } else if ch == '+' {
                        self.advance();
                        tokens.push(Token::Op("+".into()));
                    } else if ch == '-' {
                        // Could be negative number or minus operator
                        if let Some(next) = self.input.get(self.pos + 1) {
                            if next.is_ascii_digit() {
                                let start = self.pos;
                                self.advance();
                                while let Some(c) = self.peek() {
                                    if c.is_ascii_digit() {
                                        self.advance();
                                    } else if c == '.' {
                                        while let Some(c2) = self.peek() {
                                            if c2.is_ascii_digit() {
                                                self.advance();
                                            } else {
                                                break;
                                            }
                                        }
                                        break;
                                    } else {
                                        break;
                                    }
                                }
                                let num_str: String = self.input[start..self.pos].iter().collect();
                                if num_str.contains('.') {
                                    tokens.push(Token::Float(num_str.parse().unwrap_or(0.0)));
                                } else {
                                    tokens.push(Token::Number(num_str.parse().unwrap_or(0)));
                                }
                            } else {
                                self.advance();
                                tokens.push(Token::Op("-".into()));
                            }
                        } else {
                            self.advance();
                            tokens.push(Token::Op("-".into()));
                        }
                    } else if ch == '/' {
                        self.advance();
                        tokens.push(Token::Op("/".into()));
                    } else if ch == '%' {
                        self.advance();
                        tokens.push(Token::Op("%".into()));
                    } else if ch == '|' {
                        self.advance();
                        if self.peek() == Some('|') {
                            self.advance();
                            tokens.push(Token::Op("||".into()));
                        } else {
                            tokens.push(Token::Op("|".into()));
                        }
                    } else if ch == '&' {
                        self.advance();
                        tokens.push(Token::Op("&".into()));
                    } else if ch == '^' {
                        self.advance();
                        tokens.push(Token::Op("^".into()));
                    } else if ch == '~' {
                        self.advance();
                        tokens.push(Token::Op("~".into()));
                    } else {
                        return Err(ParseError {
                            message: format!("Unexpected character '{}'", ch),
                            position: self.pos,
                        });
                    }
                }
            }
        }
        Ok(tokens)
    }
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "SELECT" | "FROM" | "WHERE" | "GROUP" | "BY" | "HAVING" | "ORDER" | "LIMIT" | "OFFSET"
            | "INSERT" | "INTO" | "VALUES" | "UPDATE" | "SET" | "DELETE" | "CREATE" | "TABLE"
            | "INDEX" | "VIEW" | "DROP" | "ALTER" | "JOIN" | "LEFT" | "RIGHT" | "FULL" | "INNER"
            | "OUTER" | "CROSS" | "ON" | "USING" | "AS" | "AND" | "OR" | "NOT" | "IN" | "BETWEEN"
            | "IS" | "NULL" | "LIKE" | "ILIKE" | "EXISTS" | "CASE" | "WHEN" | "THEN" | "ELSE"
            | "END" | "DISTINCT" | "UNION" | "ALL" | "INTERSECT" | "EXCEPT" | "ASC" | "DESC"
            | "WITH" | "RECURSIVE" | "CAST" | "AS" | "PRIMARY" | "KEY" | "FOREIGN" | "REFERENCES"
            | "DEFAULT" | "UNIQUE" | "CHECK" | "CONSTRAINT" | "IF" | "RETURNING" | "CONFLICT"
            | "DO" | "NOTHING" | "ABORT" | "FAIL" | "IGNORE" | "REPLACE" | "TEMP" | "TEMPORARY"
            | "VIRTUAL" | "STRICT" | "WITHOUT" | "ROWID" | "PRAGMA" | "ATTACH" | "DETACH"
            | "BEGIN" | "COMMIT" | "ROLLBACK" | "TRANSACTION" | "VACUUM" | "ANALYZE" | "REINDEX"
            | "EXPLAIN" | "PRAGMA" | "COLLATE" | "NOCASE" | "BINARY" | "GLOB" | "REGEXP"
            | "OVER" | "PARTITION" | "WINDOW" | "FILTER" | "RESPECT" | "IGNORE" | "NULLS"
            | "FIRST" | "LAST" | "CURRENT" | "ROW" | "ROWS" | "RANGE" | "UNBOUNDED" | "PRECEDING"
            | "FOLLOWING" | "INTERVAL" | "YEAR" | "MONTH" | "DAY" | "HOUR" | "MINUTE" | "SECOND"
    )
}

/// The main SQL parser.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(sql: &str) -> Result<Self, ParseError> {
        let mut lexer = Lexer::new(sql);
        let tokens = lexer.tokenize()?;
        Ok(Self { tokens, pos: 0 })
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), ParseError> {
        let tok = self.peek().clone();
        if std::mem::discriminant(&tok) == std::mem::discriminant(expected) {
            // For Ident/Keyword/StringLit/Number/Float/Op, compare values
            match (&tok, expected) {
                (Token::Ident(a), Token::Ident(b)) => {
                    if a != b {
                        return Err(ParseError {
                            message: format!("Expected '{}', got '{}'", b, a),
                            position: self.pos,
                        });
                    }
                }
                (Token::Keyword(a), Token::Keyword(b)) => {
                    if a != b {
                        return Err(ParseError {
                            message: format!("Expected '{}', got '{}'", b, a),
                            position: self.pos,
                        });
                    }
                }
                (Token::StringLit(a), Token::StringLit(b)) => {
                    if a != b {
                        return Err(ParseError {
                            message: format!("Expected '{}', got '{}'", b, a),
                            position: self.pos,
                        });
                    }
                }
                (Token::Number(a), Token::Number(b)) => {
                    if a != b {
                        return Err(ParseError {
                            message: format!("Expected '{}', got '{}'", b, a),
                            position: self.pos,
                        });
                    }
                }
                (Token::Op(a), Token::Op(b)) => {
                    if a != b {
                        return Err(ParseError {
                            message: format!("Expected '{}', got '{}'", b, a),
                            position: self.pos,
                        });
                    }
                }
                _ => {}
            }
        } else {
            return Err(ParseError {
                message: format!("Unexpected token: {:?}", tok),
                position: self.pos,
            });
        }
        self.advance();
        Ok(())
    }

    fn expect_keyword(&mut self, kw: &str) -> Result<(), ParseError> {
        match self.peek() {
            Token::Keyword(k) if k == kw => {
                self.advance();
                Ok(())
            }
            other => Err(ParseError {
                message: format!("Expected keyword '{}', got {:?}", kw, other),
                position: self.pos,
            }),
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.peek() {
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            // Allow keywords as identifiers in some contexts
            Token::Keyword(kw) => {
                let name = kw.clone();
                self.advance();
                Ok(name)
            }
            other => Err(ParseError {
                message: format!("Expected identifier, got {:?}", other),
                position: self.pos,
            }),
        }
    }

    fn expect_comma(&mut self) -> Result<(), ParseError> {
        match self.peek() {
            Token::Comma => {
                self.advance();
                Ok(())
            }
            other => Err(ParseError {
                message: format!("Expected ',', got {:?}", other),
                position: self.pos,
            }),
        }
    }

    fn expect_lparen(&mut self) -> Result<(), ParseError> {
        match self.peek() {
            Token::LParen => {
                self.advance();
                Ok(())
            }
            other => Err(ParseError {
                message: format!("Expected '(', got {:?}", other),
                position: self.pos,
            }),
        }
    }

    fn expect_rparen(&mut self) -> Result<(), ParseError> {
        match self.peek() {
            Token::RParen => {
                self.advance();
                Ok(())
            }
            other => Err(ParseError {
                message: format!("Expected ')', got {:?}", other),
                position: self.pos,
            }),
        }
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek(), Token::EOF)
    }

    fn parse_statement(&mut self) -> Result<SqlStatement, ParseError> {
        match self.peek() {
            Token::Keyword(kw) => match kw.as_str() {
                "SELECT" | "WITH" => {
                    let stmt = self.parse_select()?;
                    Ok(SqlStatement::Select(stmt))
                }
                "INSERT" => {
                    let stmt = self.parse_insert()?;
                    Ok(SqlStatement::Insert(stmt))
                }
                "UPDATE" => {
                    let stmt = self.parse_update()?;
                    Ok(SqlStatement::Update(stmt))
                }
                "DELETE" => {
                    let stmt = self.parse_delete()?;
                    Ok(SqlStatement::Delete(stmt))
                }
                "CREATE" => {
                    let stmt = self.parse_create()?;
                    Ok(SqlStatement::Create(stmt))
                }
                _ => Err(ParseError {
                    message: format!("Unsupported statement keyword: {}", kw),
                    position: self.pos,
                }),
            },
            other => Err(ParseError {
                message: format!("Expected statement, got {:?}", other),
                position: self.pos,
            }),
        }
    }

    fn parse_select(&mut self) -> Result<SelectStmt, ParseError> {
        let mut ctes = Vec::new();

        // Handle WITH clause (CTEs)
        if matches!(self.peek(), Token::Keyword(k) if k == "WITH") {
            self.advance();
            // Check for RECURSIVE
            let recursive = if matches!(self.peek(), Token::Keyword(k) if k == "RECURSIVE") {
                self.advance();
                true
            } else {
                false
            };

            loop {
                let name = self.expect_ident()?;
                let mut columns = Vec::new();
                if matches!(self.peek(), Token::LParen) {
                    self.advance();
                    loop {
                        let col = self.expect_ident()?;
                        columns.push(col);
                        if matches!(self.peek(), Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.expect_rparen()?;
                }

                // Expect AS
                if matches!(self.peek(), Token::Keyword(k) if k == "AS") {
                    self.advance();
                }

                let body = self.parse_select_inner()?;

                ctes.push(CteDef {
                    name,
                    columns,
                    body: Box::new(body),
                    recursive,
                });

                if matches!(self.peek(), Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        let select = self.parse_select_inner()?;
        let mut result = select;
        result.ctes = ctes;
        Ok(result)
    }

    fn parse_select_inner(&mut self) -> Result<SelectStmt, ParseError> {
        // Handle DISTINCT
        let distinct = if matches!(self.peek(), Token::Keyword(k) if k == "DISTINCT") {
            self.advance();
            true
        } else {
            false
        };

        self.expect_keyword("SELECT")?;

        // Parse select list
        let mut select_list = Vec::new();
        loop {
            let item = self.parse_select_item()?;
            select_list.push(item);
            if matches!(self.peek(), Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        // FROM clause
        let from_clause = if matches!(self.peek(), Token::Keyword(k) if k == "FROM") {
            self.advance();
            Some(self.parse_from_clause()?)
        } else {
            None
        };

        // WHERE clause
        let where_clause = if matches!(self.peek(), Token::Keyword(k) if k == "WHERE") {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        // GROUP BY
        let mut group_by = Vec::new();
        if matches!(self.peek(), Token::Keyword(k) if k == "GROUP") {
            self.advance();
            self.expect_keyword("BY")?;
            loop {
                let expr = self.parse_expression()?;
                group_by.push(expr);
                if matches!(self.peek(), Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // HAVING
        let having = if matches!(self.peek(), Token::Keyword(k) if k == "HAVING") {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        // ORDER BY
        let mut order_by = Vec::new();
        if matches!(self.peek(), Token::Keyword(k) if k == "ORDER") {
            self.advance();
            self.expect_keyword("BY")?;
            loop {
                let expr = self.parse_expression()?;
                let direction = if matches!(self.peek(), Token::Keyword(k) if k == "DESC") {
                    self.advance();
                    OrderDirection::Desc
                } else {
                    if matches!(self.peek(), Token::Keyword(k) if k == "ASC") {
                        self.advance();
                    }
                    OrderDirection::Asc
                };
                order_by.push(OrderByItem { expr, direction });
                if matches!(self.peek(), Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // LIMIT
        let limit = if matches!(self.peek(), Token::Keyword(k) if k == "LIMIT") {
            self.advance();
            let val = match self.peek() {
                Token::Number(n) => {
                    let v = *n as u64;
                    self.advance();
                    v
                }
                _ => return Err(ParseError {
                    message: "Expected number after LIMIT".into(),
                    position: self.pos,
                }),
            };
            Some(val)
        } else {
            None
        };

        // OFFSET
        let offset = if matches!(self.peek(), Token::Keyword(k) if k == "OFFSET") {
            self.advance();
            let val = match self.peek() {
                Token::Number(n) => {
                    let v = *n as u64;
                    self.advance();
                    v
                }
                _ => return Err(ParseError {
                    message: "Expected number after OFFSET".into(),
                    position: self.pos,
                }),
            };
            Some(val)
        } else {
            None
        };

        Ok(SelectStmt {
            ctes: Vec::new(),
            select_list,
            from_clause,
            where_clause,
            group_by,
            having,
            order_by,
            limit,
            offset,
            distinct,
        })
    }

    fn parse_select_item(&mut self) -> Result<SelectItem, ParseError> {
        // Check for *
        if matches!(self.peek(), Token::Star) {
            self.advance();
            return Ok(SelectItem::Star);
        }

        let expr = self.parse_expression()?;

        // Check for alias
        let alias = if matches!(self.peek(), Token::Keyword(k) if k == "AS") {
            self.advance();
            Some(self.expect_ident()?)
        } else if matches!(self.peek(), Token::Ident(_)) {
            // Could be an implicit alias
            let name = self.expect_ident()?;
            // Check it's not a keyword that starts a new clause
            if is_clause_keyword(&name) {
                // Put it back - this is actually a clause, not an alias
                // We can't easily "un-advance", so we handle this differently
                // For simplicity, treat as alias only if it's a plain ident
                Some(name)
            } else {
                Some(name)
            }
        } else {
            None
        };

        // Check if expression is an aggregate
        if let Expression::FunctionCall(fc) = &expr {
            if is_agg_function(&fc.name) {
                let kind = agg_kind_from_name(&fc.name);
                let arg = if fc.args.is_empty() {
                    None
                } else {
                    Some(Box::new(fc.args[0].clone()))
                };
                return Ok(SelectItem::Agg(AggFunction {
                    kind,
                    arg,
                    distinct: fc.distinct,
                }));
            }
        }

        if let Expression::Column(ref col) = expr {
            Ok(SelectItem::Column(col.clone()))
        } else {
            Ok(SelectItem::Expression(expr, alias))
        }
    }

    fn parse_from_clause(&mut self) -> Result<FromClause, ParseError> {
        let mut tables = Vec::new();
        let first_table = self.parse_table_ref()?;
        tables.push(first_table);

        // Additional comma-separated tables (implicit cross join)
        while matches!(self.peek(), Token::Comma) {
            self.advance();
            let table = self.parse_table_ref()?;
            tables.push(table);
        }

        // JOINs
        let mut joins = Vec::new();
        loop {
            if let Some(join) = self.try_parse_join()? {
                joins.push(join);
            } else {
                break;
            }
        }

        Ok(FromClause { tables, joins })
    }

    fn parse_table_ref(&mut self) -> Result<TableRef, ParseError> {
        if matches!(self.peek(), Token::LParen) {
            // Subquery
            self.advance();
            let sub = self.parse_select_inner()?;
            self.expect_rparen()?;
            let alias = if matches!(self.peek(), Token::Keyword(k) if k == "AS") {
                self.advance();
                self.expect_ident()?
            } else {
                self.expect_ident()?
            };
            Ok(TableRef::Subquery(Box::new(sub), alias))
        } else {
            let name = self.expect_ident()?;
            let alias = if matches!(self.peek(), Token::Keyword(k) if k == "AS") {
                self.advance();
                Some(self.expect_ident()?)
            } else if matches!(self.peek(), Token::Ident(_)) {
                // Check if next token is a JOIN keyword or end of from clause
                let next_is_join = matches!(self.peek(), Token::Keyword(k) if k == "JOIN" || k == "LEFT" || k == "RIGHT" || k == "FULL" || k == "CROSS" || k == "INNER");
                if !next_is_join && !matches!(self.peek(), Token::Comma) && !matches!(self.peek(), Token::Keyword(k) if k == "WHERE" || k == "GROUP" || k == "ORDER" || k == "LIMIT" || k == "HAVING") {
                    Some(self.expect_ident()?)
                } else {
                    None
                }
            } else {
                None
            };
            Ok(TableRef::Simple(name, alias))
        }
    }

    fn try_parse_join(&mut self) -> Result<Option<JoinClause>, ParseError> {
        let join_kind = if matches!(self.peek(), Token::Keyword(k) if k == "JOIN") {
            JoinKind::Inner
        } else if matches!(self.peek(), Token::Keyword(k) if k == "LEFT") {
            self.advance();
            if matches!(self.peek(), Token::Keyword(k) if k == "OUTER") {
                self.advance();
            }
            self.expect_keyword("JOIN")?;
            JoinKind::LeftOuter
        } else if matches!(self.peek(), Token::Keyword(k) if k == "RIGHT") {
            self.advance();
            if matches!(self.peek(), Token::Keyword(k) if k == "OUTER") {
                self.advance();
            }
            self.expect_keyword("JOIN")?;
            JoinKind::RightOuter
        } else if matches!(self.peek(), Token::Keyword(k) if k == "FULL") {
            self.advance();
            if matches!(self.peek(), Token::Keyword(k) if k == "OUTER") {
                self.advance();
            }
            self.expect_keyword("JOIN")?;
            JoinKind::FullOuter
        } else if matches!(self.peek(), Token::Keyword(k) if k == "CROSS") {
            self.advance();
            self.expect_keyword("JOIN")?;
            JoinKind::Cross
        } else if matches!(self.peek(), Token::Keyword(k) if k == "INNER") {
            self.advance();
            self.expect_keyword("JOIN")?;
            JoinKind::Inner
        } else {
            return Ok(None);
        };

        let table = self.parse_table_ref()?;

        let on = if matches!(self.peek(), Token::Keyword(k) if k == "ON") {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        let using = if matches!(self.peek(), Token::Keyword(k) if k == "USING") {
            self.advance();
            self.expect_lparen()?;
            let mut cols = Vec::new();
            loop {
                let col = self.expect_ident()?;
                cols.push(col);
                if matches!(self.peek(), Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect_rparen()?;
            cols
        } else {
            Vec::new()
        };

        Ok(Some(JoinClause {
            kind: join_kind,
            table,
            on,
            using,
        }))
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_and_expr()?;
        while matches!(self.peek(), Token::Keyword(k) if k == "OR") {
            self.advance();
            let right = self.parse_and_expr()?;
            left = Expression::BinaryOp(
                BinaryOpKind::Or,
                Box::new(left),
                Box::new(right),
            );
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_not_expr()?;
        while matches!(self.peek(), Token::Keyword(k) if k == "AND") {
            self.advance();
            let right = self.parse_not_expr()?;
            left = Expression::BinaryOp(
                BinaryOpKind::And,
                Box::new(left),
                Box::new(right),
            );
        }
        Ok(left)
    }

    fn parse_not_expr(&mut self) -> Result<Expression, ParseError> {
        if matches!(self.peek(), Token::Keyword(k) if k == "NOT") {
            self.advance();
            let expr = self.parse_not_expr()?;
            Ok(Expression::UnaryOp(UnaryOpKind::Not, Box::new(expr)))
        } else {
            self.parse_comparison()
        }
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let left = self.parse_additive()?;

        // IS [NOT] NULL
        if matches!(self.peek(), Token::Keyword(k) if k == "IS") {
            self.advance();
            let negated = if matches!(self.peek(), Token::Keyword(k) if k == "NOT") {
                self.advance();
                true
            } else {
                false
            };
            self.expect_keyword("NULL")?;
            return Ok(Expression::IsNull(Box::new(left), negated));
        }

        // IN / NOT IN
        if matches!(self.peek(), Token::Keyword(k) if k == "IN") {
            let negated = false;
            self.advance();
            let list = self.parse_in_list()?;
            return Ok(Expression::InList(Box::new(left), list, negated));
        }
        if matches!(self.peek(), Token::Keyword(k) if k == "NOT") {
            // Look ahead for IN
            if self.tokens.get(self.pos + 1).map(|t| matches!(t, Token::Keyword(k) if k == "IN")).unwrap_or(false) {
                self.advance(); // NOT
                self.advance(); // IN
                let list = self.parse_in_list()?;
                return Ok(Expression::InList(Box::new(left), list, true));
            }
        }

        // BETWEEN / NOT BETWEEN
        if matches!(self.peek(), Token::Keyword(k) if k == "BETWEEN") {
            self.advance();
            let low = self.parse_additive()?;
            self.expect_keyword("AND")?;
            let high = self.parse_additive()?;
            return Ok(Expression::Between(
                Box::new(left),
                Box::new(low),
                Box::new(high),
                false,
            ));
        }

        // LIKE / NOT LIKE
        if matches!(self.peek(), Token::Keyword(k) if k == "LIKE") {
            self.advance();
            let right = self.parse_additive()?;
            return Ok(Expression::BinaryOp(
                BinaryOpKind::Like,
                Box::new(left),
                Box::new(right),
            ));
        }

        // Comparison operators
        match self.peek() {
            Token::Op(op) => {
                let op_str = op.clone();
                if matches!(op_str.as_str(), "=" | "!=" | "<" | "<=" | ">" | ">=") {
                    self.advance();
                    let right = self.parse_additive()?;
                    let kind = match op_str.as_str() {
                        "=" => BinaryOpKind::Eq,
                        "!=" => BinaryOpKind::Neq,
                        "<" => BinaryOpKind::Lt,
                        "<=" => BinaryOpKind::LtEq,
                        ">" => BinaryOpKind::Gt,
                        ">=" => BinaryOpKind::GtEq,
                        _ => unreachable!(),
                    };
                    return Ok(Expression::BinaryOp(
                        kind,
                        Box::new(left),
                        Box::new(right),
                    ));
                }
            }
            _ => {}
        }

        Ok(left)
    }

    fn parse_in_list(&mut self) -> Result<Vec<Expression>, ParseError> {
        self.expect_lparen()?;
        let mut items = Vec::new();
        loop {
            let expr = self.parse_expression()?;
            items.push(expr);
            if matches!(self.peek(), Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect_rparen()?;
        Ok(items)
    }

    fn parse_additive(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            match self.peek() {
                Token::Op(op) if op == "+" || op == "-" => {
                    let op_str = op.clone();
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    let kind = if op_str == "+" {
                        BinaryOpKind::Add
                    } else {
                        BinaryOpKind::Sub
                    };
                    left = Expression::BinaryOp(kind, Box::new(left), Box::new(right));
                }
                Token::Op(op) if op == "||" => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expression::BinaryOp(
                        BinaryOpKind::Concat,
                        Box::new(left),
                        Box::new(right),
                    );
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            match self.peek() {
                Token::Op(op) if op == "*" || op == "/" || op == "%" => {
                    let op_str = op.clone();
                    self.advance();
                    let right = self.parse_unary()?;
                    let kind = match op_str.as_str() {
                        "*" => BinaryOpKind::Mul,
                        "/" => BinaryOpKind::Div,
                        "%" => BinaryOpKind::Mod,
                        _ => unreachable!(),
                    };
                    left = Expression::BinaryOp(kind, Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        if matches!(self.peek(), Token::Op(op) if op == "-") {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expression::UnaryOp(UnaryOpKind::Neg, Box::new(expr)));
        }
        if matches!(self.peek(), Token::Op(op) if op == "~") {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expression::UnaryOp(UnaryOpKind::BitNot, Box::new(expr)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        match self.peek().clone() {
            Token::Number(n) => {
                self.advance();
                Ok(Expression::Literal(Literal::Integer(n)))
            }
            Token::Float(f) => {
                self.advance();
                Ok(Expression::Literal(Literal::Float(f)))
            }
            Token::StringLit(s) => {
                self.advance();
                Ok(Expression::Literal(Literal::String(s)))
            }
            Token::Keyword(k) if k == "NULL" => {
                self.advance();
                Ok(Expression::Literal(Literal::Null))
            }
            Token::Keyword(k) if k == "TRUE" || k == "FALSE" => {
                self.advance();
                let val = k == "TRUE";
                Ok(Expression::Literal(Literal::Boolean(val)))
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect_rparen()?;
                Ok(expr)
            }
            Token::Star => {
                // COUNT(*) style - handled in function call context
                self.advance();
                Ok(Expression::Literal(Literal::Null)) // placeholder
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();

                // Check for table.column
                if matches!(self.peek(), Token::Dot) {
                    self.advance();
                    let col = self.expect_ident()?;
                    return Ok(Expression::Column(ColumnRef {
                        table: Some(name),
                        column: col,
                    }));
                }

                // Check for function call
                if matches!(self.peek(), Token::LParen) {
                    self.advance();
                    let mut distinct = false;
                    if matches!(self.peek(), Token::Keyword(k) if k == "DISTINCT") {
                        self.advance();
                        distinct = true;
                    }

                    let mut args = Vec::new();
                    if !matches!(self.peek(), Token::RParen) {
                        loop {
                            // Handle * as argument (e.g., COUNT(*))
                            if matches!(self.peek(), Token::Star) {
                                self.advance();
                                args.push(Expression::Literal(Literal::Null));
                            } else {
                                let arg = self.parse_expression()?;
                                args.push(arg);
                            }
                            if matches!(self.peek(), Token::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect_rparen()?;

                    Ok(Expression::FunctionCall(FunctionCall {
                        name,
                        args,
                        distinct,
                    }))
                } else {
                    // Simple column reference
                    Ok(Expression::Column(ColumnRef {
                        table: None,
                        column: name,
                    }))
                }
            }
            Token::Keyword(k) if k == "CASE" => {
                self.advance();
                let mut operand = None;
                // Optional operand: CASE expr WHEN ...
                if !matches!(self.peek(), Token::Keyword(k2) if k2 == "WHEN") {
                    let op = self.parse_expression()?;
                    operand = Some(Box::new(op));
                }

                let mut whens = Vec::new();
                loop {
                    self.expect_keyword("WHEN")?;
                    let when_expr = self.parse_expression()?;
                    self.expect_keyword("THEN")?;
                    let then_expr = self.parse_expression()?;
                    whens.push((when_expr, then_expr));
                    if matches!(self.peek(), Token::Keyword(k2) if k2 == "WHEN") {
                        continue;
                    }
                    break;
                }

                let else_expr = if matches!(self.peek(), Token::Keyword(k2) if k2 == "ELSE") {
                    self.advance();
                    Some(Box::new(self.parse_expression()?))
                } else {
                    None
                };

                self.expect_keyword("END")?;
                Ok(Expression::Case(CaseExpr {
                    operand,
                    whens,
                    else_expr,
                }))
            }
            Token::Keyword(k) if k == "CAST" => {
                self.advance();
                self.expect_lparen()?;
                let expr = self.parse_expression()?;
                self.expect_keyword("AS")?;
                let data_type = self.parse_data_type()?;
                self.expect_rparen()?;
                Ok(Expression::Cast(Box::new(expr), data_type))
            }
            Token::Keyword(k) if k == "EXISTS" => {
                self.advance();
                self.expect_lparen()?;
                let sub = self.parse_select_inner()?;
                self.expect_rparen()?;
                Ok(Expression::Subquery(Box::new(sub)))
            }
            Token::Op(op) if op == "?" => {
                // Parameter placeholder - not standard but useful for injection detection
                self.advance();
                Ok(Expression::Parameter(0))
            }
            _ => Err(ParseError {
                message: format!("Unexpected token in expression: {:?}", self.peek()),
                position: self.pos,
            }),
        }
    }

    fn parse_data_type(&mut self) -> Result<DataType, ParseError> {
        let name = self.expect_ident()?;
        let upper = name.to_uppercase();

        // Check for size specifier
        if matches!(self.peek(), Token::LParen) {
            self.advance();
            let mut p1: Option<u64> = None;
            let mut p2: Option<u64> = None;
            if let Token::Number(n) = self.peek().clone() {
                p1 = Some(*n as u64);
                self.advance();
                if matches!(self.peek(), Token::Comma) {
                    self.advance();
                    if let Token::Number(n2) = self.peek().clone() {
                        p2 = Some(*n2 as u64);
                        self.advance();
                    }
                }
            }
            self.expect_rparen()?;

            match upper.as_str() {
                "VARCHAR" | "CHAR" => Ok(DataType::Varchar(p1)),
                "DECIMAL" | "NUMERIC" => Ok(DataType::Decimal(p1, p2)),
                _ => Ok(DataType::Custom(name)),
            }
        } else {
            match upper.as_str() {
                "INTEGER" | "INT" => Ok(DataType::Integer),
                "BIGINT" => Ok(DataType::BigInt),
                "SMALLINT" => Ok(DataType::SmallInt),
                "VARCHAR" | "CHAR" => Ok(DataType::Varchar(None)),
                "TEXT" => Ok(DataType::Text),
                "BOOLEAN" | "BOOL" => Ok(DataType::Boolean),
                "FLOAT" => Ok(DataType::Float),
                "DOUBLE" => Ok(DataType::Double),
                "DECIMAL" | "NUMERIC" => Ok(DataType::Decimal(None, None)),
                "DATE" => Ok(DataType::Date),
                "TIMESTAMP" => Ok(DataType::Timestamp),
                "BLOB" => Ok(DataType::Blob),
                _ => Ok(DataType::Custom(name)),
            }
        }
    }

    fn parse_insert(&mut self) -> Result<InsertStmt, ParseError> {
        self.expect_keyword("INSERT")?;
        // Optional OR clause (SQLite)
        if matches!(self.peek(), Token::Keyword(k) if k == "OR") {
            self.advance();
            let _ = self.expect_ident()?; // ABORT, FAIL, IGNORE, REPLACE, ROLLBACK
        }
        self.expect_keyword("INTO")?;
        let table = self.expect_ident()?;

        let mut columns = Vec::new();
        if matches!(self.peek(), Token::LParen) {
            self.advance();
            loop {
                let col = self.expect_ident()?;
                columns.push(col);
                if matches!(self.peek(), Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect_rparen()?;
        }

        // VALUES or SELECT
        let mut values = Vec::new();
        let select = if matches!(self.peek(), Token::Keyword(k) if k == "SELECT") || matches!(self.peek(), Token::Keyword(k) if k == "WITH") {
            let sel = self.parse_select_inner()?;
            Some(Box::new(sel))
        } else {
            self.expect_keyword("VALUES")?;
            loop {
                self.expect_lparen()?;
                let mut row = Vec::new();
                loop {
                    let expr = self.parse_expression()?;
                    row.push(expr);
                    if matches!(self.peek(), Token::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect_rparen()?;
                values.push(row);
                if matches!(self.peek(), Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            None
        };

        Ok(InsertStmt {
            table,
            columns,
            values,
            select,
        })
    }

    fn parse_update(&mut self) -> Result<UpdateStmt, ParseError> {
        self.expect_keyword("UPDATE")?;
        let table = self.expect_ident()?;
        self.expect_keyword("SET")?;

        let mut set_clauses = Vec::new();
        loop {
            let col = self.expect_ident()?;
            // Expect =
            if matches!(self.peek(), Token::Op(op) if op == "=") {
                self.advance();
            }
            let expr = self.parse_expression()?;
            set_clauses.push((col, expr));
            if matches!(self.peek(), Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        let where_clause = if matches!(self.peek(), Token::Keyword(k) if k == "WHERE") {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(UpdateStmt {
            table,
            set_clauses,
            where_clause,
        })
    }

    fn parse_delete(&mut self) -> Result<DeleteStmt, ParseError> {
        self.expect_keyword("DELETE")?;
        self.expect_keyword("FROM")?;
        let table = self.expect_ident()?;

        let where_clause = if matches!(self.peek(), Token::Keyword(k) if k == "WHERE") {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(DeleteStmt { table, where_clause })
    }

    fn parse_create(&mut self) -> Result<CreateStmt, ParseError> {
        self.expect_keyword("CREATE")?;
        // Optional TEMP/TEMPORARY
        if matches!(self.peek(), Token::Keyword(k) if k == "TEMP" || k == "TEMPORARY") {
            self.advance();
        }

        let kind = match self.peek() {
            Token::Keyword(k) => {
                let k = k.clone();
                self.advance();
                k
            }
            _ => return Err(ParseError {
                message: "Expected CREATE TABLE/INDEX/VIEW".into(),
                position: self.pos,
            }),
        };

        // Optional IF NOT EXISTS
        if matches!(self.peek(), Token::Keyword(k) if k == "IF") {
            self.advance();
            self.expect_keyword("NOT")?;
            self.expect_keyword("EXISTS")?;
        }

        let name = self.expect_ident()?;
        let mut columns = Vec::new();

        if kind == "TABLE" {
            self.expect_lparen()?;
            loop {
                // Parse column definition or constraint
                let col_name = self.expect_ident()?;
                let data_type = self.parse_data_type()?;

                let mut not_null = false;
                let mut primary_key = false;
                let mut default = None;

                // Parse column constraints
                loop {
                    if matches!(self.peek(), Token::Keyword(k) if k == "NOT") {
                        self.advance();
                        self.expect_keyword("NULL")?;
                        not_null = true;
                    } else if matches!(self.peek(), Token::Keyword(k) if k == "PRIMARY") {
                        self.advance();
                        self.expect_keyword("KEY")?;
                        primary_key = true;
                    } else if matches!(self.peek(), Token::Keyword(k) if k == "DEFAULT") {
                        self.advance();
                        let expr = self.parse_expression()?;
                        default = Some(expr);
                    } else if matches!(self.peek(), Token::Keyword(k) if k == "UNIQUE") {
                        self.advance();
                    } else if matches!(self.peek(), Token::Keyword(k) if k == "CHECK") {
                        self.advance();
                        // Skip CHECK constraint body
                        self.expect_lparen()?;
                        let _ = self.parse_expression()?;
                        self.expect_rparen()?;
                    } else if matches!(self.peek(), Token::Keyword(k) if k == "REFERENCES") {
                        self.advance();
                        let _ref_table = self.expect_ident()?;
                        // Skip optional column list
                        if matches!(self.peek(), Token::LParen) {
                            self.advance();
                            loop {
                                let _ = self.expect_ident()?;
                                if matches!(self.peek(), Token::Comma) {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                            self.expect_rparen()?;
                        }
                    } else {
                        break;
                    }
                }

                columns.push(ColumnDef {
                    name: col_name,
                    data_type,
                    not_null,
                    primary_key,
                    default,
                });

                if matches!(self.peek(), Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect_rparen()?;
        }

        Ok(CreateStmt {
            kind,
            name,
            columns,
        })
    }
}

fn is_clause_keyword(s: &str) -> bool {
    matches!(
        s.to_uppercase().as_str(),
        "WHERE" | "GROUP" | "ORDER" | "LIMIT" | "HAVING" | "FROM" | "JOIN" | "LEFT" | "RIGHT"
            | "FULL" | "CROSS" | "INNER" | "ON" | "USING" | "UNION" | "INTERSECT" | "EXCEPT"
    )
}

fn is_agg_function(name: &str) -> bool {
    matches!(
        name.to_uppercase().as_str(),
        "COUNT" | "SUM" | "AVG" | "MIN" | "MAX" | "TOTAL"
    )
}

fn agg_kind_from_name(name: &str) -> AggKind {
    match name.to_uppercase().as_str() {
        "COUNT" => AggKind::Count,
        "SUM" => AggKind::Sum,
        "AVG" => AggKind::Avg,
        "MIN" => AggKind::Min,
        "MAX" => AggKind::Max,
        "TOTAL" => AggKind::Total,
        _ => AggKind::Count, // fallback
    }
}

/// Parse a SQL string into a statement.
pub fn parse_sql(sql: &str) -> Result<SqlStatement, ParseError> {
    let mut parser = Parser::new(sql)?;
    parser.parse_statement()
}
