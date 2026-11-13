//! Resilient, high-performance recursive descent parser for SQL.

use crate::sql::ast::*;
use crate::sql::token::{Lexer, Token, TokenKind};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    peeked: Option<Token>,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: Lexer<'a>) -> Self {
        Parser {
            lexer,
            peeked: None,
        }
    }

    pub fn from_src(src: &'a str) -> Self {
        Self::new(Lexer::new(src))
    }

    fn peek(&mut self) -> &Token {
        if self.peeked.is_none() {
            self.peeked = Some(self.lexer.next_token());
        }
        self.peeked.as_ref().unwrap()
    }

    fn peek_kind(&mut self) -> TokenKind {
        self.peek().kind.clone()
    }

    fn next_token(&mut self) -> Token {
        if let Some(tok) = self.peeked.take() {
            tok
        } else {
            self.lexer.next_token()
        }
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.peek_kind() == kind {
            self.next_token();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, String> {
        let tok = self.next_token();
        if tok.kind == kind {
            Ok(tok)
        } else {
            Err(format!(
                "line {}:{}: expected {:?}, found {:?}",
                tok.line, tok.col, kind, tok.kind
            ))
        }
    }

    fn eat_semi(&mut self) {
        while self.eat(TokenKind::Semi) {}
    }

    /// Fast-forward until a statement terminator (`;`, `GO`, `/` or EOF).
    fn skip_until_semi(&mut self) {
        let mut paren_depth = 0;
        let mut begin_depth = 0;
        while self.peek_kind() != TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::LParen => {
                    paren_depth += 1;
                    self.next_token();
                }
                TokenKind::RParen => {
                    if paren_depth > 0 {
                        paren_depth -= 1;
                    }
                    self.next_token();
                }
                TokenKind::Begin => {
                    begin_depth += 1;
                    self.next_token();
                }
                TokenKind::End => {
                    if begin_depth > 0 {
                        begin_depth -= 1;
                    }
                    self.next_token();
                }
                TokenKind::Semi => {
                    if paren_depth == 0 && begin_depth == 0 {
                        self.next_token();
                        break;
                    } else {
                        self.next_token();
                    }
                }
                _ => {
                    self.next_token();
                }
            }
        }
    }

    /// Parse the next statement in the stream, or None at EOF.
    pub fn next_statement(&mut self) -> Option<Statement> {
        self.eat_semi();

        let tok = self.peek().clone();
        if tok.kind == TokenKind::Eof {
            return None;
        }

        match tok.kind {
            TokenKind::Create => {
                self.next_token(); // consume CREATE
                let is_replace = self.eat(TokenKind::Replace) || (self.eat(TokenKind::Or) && self.eat(TokenKind::Replace));
                match self.peek_kind() {
                    TokenKind::Table => Some(self.parse_create_table(tok.line)),
                    TokenKind::View => Some(self.parse_create_view(tok.line)),
                    TokenKind::Trigger => Some(self.parse_create_trigger(tok.line)),
                    TokenKind::Function => Some(self.parse_create_function(tok.line)),
                    TokenKind::Procedure => Some(self.parse_create_procedure(tok.line)),
                    _ => {
                        self.skip_until_semi();
                        Some(Statement::Other {
                            kind: "CREATE".to_string(),
                            line: tok.line,
                        })
                    }
                }
            }
            TokenKind::Alter => {
                self.next_token(); // consume ALTER
                if self.peek_kind() == TokenKind::Table {
                    Some(self.parse_alter_table(tok.line))
                } else {
                    self.skip_until_semi();
                    Some(Statement::Other {
                        kind: "ALTER".to_string(),
                        line: tok.line,
                    })
                }
            }
            TokenKind::Drop => {
                self.next_token(); // consume DROP
                if self.peek_kind() == TokenKind::Table {
                    Some(self.parse_drop_table(tok.line))
                } else {
                    self.skip_until_semi();
                    Some(Statement::Other {
                        kind: "DROP".to_string(),
                        line: tok.line,
                    })
                }
            }
            TokenKind::Insert => Some(self.parse_insert(tok.line)),
            TokenKind::Update => Some(self.parse_update(tok.line)),
            TokenKind::Delete => Some(self.parse_delete(tok.line)),
            TokenKind::Select => {
                let stmt = self.parse_select();
                self.eat_semi();
                Some(Statement::Select(Box::new(stmt)))
            }
            TokenKind::With => {
                let stmt = self.parse_select();
                self.eat_semi();
                Some(Statement::Select(Box::new(stmt)))
            }
            TokenKind::Copy => {
                // PostgreSQL COPY ... FROM stdin;
                self.next_token(); // consume COPY
                // Look for FROM STDIN
                let mut is_stdin = false;
                while self.peek_kind() != TokenKind::Semi && self.peek_kind() != TokenKind::Eof {
                    if self.peek_kind() == TokenKind::Stdin {
                        is_stdin = true;
                    }
                    self.next_token();
                }
                self.eat(TokenKind::Semi);
                if is_stdin {
                    self.lexer.skip_copy_stdin_data();
                }
                Some(Statement::Other {
                    kind: "COPY".to_string(),
                    line: tok.line,
                })
            }
            _ => {
                // Other statements: SET, USE, BEGIN, COMMIT, etc.
                self.skip_until_semi();
                Some(Statement::Other {
                    kind: format!("{:?}", tok.kind),
                    line: tok.line,
                })
            }
        }
    }

    /// Parse `CREATE TABLE [IF NOT EXISTS] name ( ... )`
    fn parse_create_table(&mut self, line: usize) -> Statement {
        self.next_token(); // consume TABLE
        // Optional IF NOT EXISTS
        if self.eat(TokenKind::If) {
            let _ = self.eat(TokenKind::Not);
            let _ = self.eat(TokenKind::Exists);
        }

        let name = self.parse_ident().unwrap_or_else(|_| "unknown".to_string());
        let mut columns = Vec::new();
        let mut constraints = Vec::new();

        if self.eat(TokenKind::LParen) {
            while self.peek_kind() != TokenKind::RParen && self.peek_kind() != TokenKind::Eof {
                if self.is_table_constraint_start() {
                    if let Some(c) = self.parse_table_constraint() {
                        constraints.push(c);
                    }
                } else if let Some(col) = self.parse_column_def() {
                    columns.push(col);
                } else {
                    // skip to next comma or rparen
                    while self.peek_kind() != TokenKind::Comma
                        && self.peek_kind() != TokenKind::RParen
                        && self.peek_kind() != TokenKind::Eof
                    {
                        self.next_token();
                    }
                }

                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            let _ = self.eat(TokenKind::RParen);
        }

        self.skip_until_semi();
        Statement::CreateTable(CreateTableStmt {
            name,
            columns,
            constraints,
            line,
        })
    }

    fn is_table_constraint_start(&mut self) -> bool {
        match self.peek_kind() {
            TokenKind::Constraint
            | TokenKind::Primary
            | TokenKind::Foreign
            | TokenKind::Unique
            | TokenKind::Check => true,
            TokenKind::Ident(ref s) => {
                let sl = s.to_ascii_lowercase();
                sl == "key" || sl == "index"
            }
            _ => false,
        }
    }

    fn parse_table_constraint(&mut self) -> Option<TableConstraint> {
        let _ = self.eat(TokenKind::Constraint);
        if let TokenKind::Ident(_) = self.peek_kind() {
            // constraint name
            self.next_token();
        }

        match self.peek_kind() {
            TokenKind::Primary => {
                self.next_token();
                let _ = self.eat(TokenKind::Key);
                let cols = self.parse_ident_list();
                Some(TableConstraint::PrimaryKey(cols))
            }
            TokenKind::Foreign => {
                self.next_token();
                let _ = self.eat(TokenKind::Key);
                let cols = self.parse_ident_list();
                let _ = self.eat(TokenKind::References);
                let ref_table = self.parse_ident().unwrap_or_default();
                let ref_cols = self.parse_ident_list();
                // Skip optional ON DELETE / ON UPDATE
                while self.peek_kind() != TokenKind::Comma
                    && self.peek_kind() != TokenKind::RParen
                    && self.peek_kind() != TokenKind::Eof
                {
                    self.next_token();
                }
                Some(TableConstraint::ForeignKey {
                    columns: cols,
                    ref_table,
                    ref_columns: ref_cols,
                })
            }
            TokenKind::Unique => {
                self.next_token();
                let cols = self.parse_ident_list();
                Some(TableConstraint::Unique(cols))
            }
            TokenKind::Check => {
                self.next_token();
                if self.eat(TokenKind::LParen) {
                    let expr = self.parse_expr();
                    let _ = self.eat(TokenKind::RParen);
                    Some(TableConstraint::Check(expr))
                } else {
                    None
                }
            }
            _ => {
                // e.g. KEY idx_name (col)
                while self.peek_kind() != TokenKind::Comma
                    && self.peek_kind() != TokenKind::RParen
                    && self.peek_kind() != TokenKind::Eof
                {
                    self.next_token();
                }
                None
            }
        }
    }

    fn parse_column_def(&mut self) -> Option<ColumnDef> {
        let line = self.peek().line;
        let name = self.parse_ident().ok()?;
        let mut data_type = String::new();

        // Data type (can be multi-token like CHARACTER VARYING(15) or INT UNSIGNED)
        while self.peek_kind() != TokenKind::Comma
            && self.peek_kind() != TokenKind::RParen
            && self.peek_kind() != TokenKind::Eof
        {
            if self.is_column_constraint_start() {
                break;
            }
            let tok = self.next_token();
            if !data_type.is_empty() {
                data_type.push(' ');
            }
            data_type.push_str(&format!("{:?}", tok.kind));
            if tok.kind == TokenKind::LParen {
                // consume until RParen for types like VARCHAR(255)
                while self.peek_kind() != TokenKind::RParen && self.peek_kind() != TokenKind::Eof {
                    self.next_token();
                }
                self.eat(TokenKind::RParen);
            }
        }

        let mut not_null = false;
        let mut has_default = false;
        let mut default_expr = None;
        let mut is_primary_key = false;
        let mut is_auto_increment = false;
        let mut check_expr = None;

        // Auto-increment / serial types
        let dt_lower = data_type.to_ascii_lowercase();
        if dt_lower.contains("serial") || dt_lower.contains("identity") || dt_lower.contains("auto_increment") {
            is_auto_increment = true;
            has_default = true;
        }

        // Parse column constraints
        while self.peek_kind() != TokenKind::Comma
            && self.peek_kind() != TokenKind::RParen
            && self.peek_kind() != TokenKind::Eof
        {
            match self.peek_kind() {
                TokenKind::Not => {
                    self.next_token();
                    if self.eat(TokenKind::Null) {
                        not_null = true;
                    }
                }
                TokenKind::Null => {
                    self.next_token();
                    not_null = false;
                }
                TokenKind::Default => {
                    self.next_token();
                    has_default = true;
                    // parse default expression until comma, rparen, or constraint keyword
                    let expr = self.parse_primary_expr().unwrap_or(Expr::Raw("default".to_string()));
                    default_expr = Some(expr);
                }
                TokenKind::Primary => {
                    self.next_token();
                    let _ = self.eat(TokenKind::Key);
                    is_primary_key = true;
                    not_null = true;
                }
                TokenKind::AutoIncrement | TokenKind::Identity | TokenKind::Serial => {
                    self.next_token();
                    is_auto_increment = true;
                    has_default = true;
                }
                TokenKind::Generated => {
                    self.next_token();
                    is_auto_increment = true;
                    has_default = true;
                    // consume until next constraint
                    while !self.is_column_constraint_start()
                        && self.peek_kind() != TokenKind::Comma
                        && self.peek_kind() != TokenKind::RParen
                        && self.peek_kind() != TokenKind::Eof
                    {
                        self.next_token();
                    }
                }
                TokenKind::Check => {
                    self.next_token();
                    if self.eat(TokenKind::LParen) {
                        check_expr = Some(self.parse_expr());
                        let _ = self.eat(TokenKind::RParen);
                    }
                }
                TokenKind::References => {
                    self.next_token();
                    let _ref_table = self.parse_ident().unwrap_or_default();
                    let _ref_cols = self.parse_ident_list();
                }
                _ => {
                    self.next_token();
                }
            }
        }

        Some(ColumnDef {
            name,
            data_type,
            not_null,
            has_default,
            default_expr,
            is_primary_key,
            is_auto_increment,
            check_expr,
            line,
        })
    }

    fn is_column_constraint_start(&mut self) -> bool {
        matches!(
            self.peek_kind(),
            TokenKind::Not
                | TokenKind::Null
                | TokenKind::Default
                | TokenKind::Primary
                | TokenKind::AutoIncrement
                | TokenKind::Identity
                | TokenKind::Serial
                | TokenKind::Generated
                | TokenKind::Check
                | TokenKind::References
                | TokenKind::Constraint
                | TokenKind::Unique
        )
    }

    /// Parse `ALTER TABLE name ...`
    fn parse_alter_table(&mut self, line: usize) -> Statement {
        self.next_token(); // consume TABLE
        let table_name = self.parse_ident().unwrap_or_else(|_| "unknown".to_string());
        let mut actions = Vec::new();

        while self.peek_kind() != TokenKind::Semi && self.peek_kind() != TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::Add => {
                    self.next_token();
                    if self.is_table_constraint_start() {
                        if let Some(c) = self.parse_table_constraint() {
                            actions.push(AlterAction::AddConstraint(c));
                        }
                    } else {
                        let _ = self.eat(TokenKind::Column);
                        if let Some(col) = self.parse_column_def() {
                            actions.push(AlterAction::AddColumn(col));
                        }
                    }
                }
                TokenKind::Drop => {
                    self.next_token();
                    if self.eat(TokenKind::Constraint) {
                        let name = self.parse_ident().unwrap_or_default();
                        actions.push(AlterAction::DropConstraint(name));
                    } else if self.eat(TokenKind::Foreign) && self.eat(TokenKind::Key) {
                        let name = self.parse_ident().unwrap_or_default();
                        actions.push(AlterAction::DropConstraint(name));
                    } else {
                        let _ = self.eat(TokenKind::Column);
                        let name = self.parse_ident().unwrap_or_default();
                        actions.push(AlterAction::DropColumn(name));
                    }
                }
                TokenKind::Modify | TokenKind::Change => {
                    self.next_token();
                    let _ = self.eat(TokenKind::Column);
                    if let Some(col) = self.parse_column_def() {
                        actions.push(AlterAction::ModifyColumn(col));
                    }
                }
                _ => {
                    self.next_token();
                }
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }

        self.skip_until_semi();
        Statement::AlterTable(AlterTableStmt {
            table_name,
            actions,
            line,
        })
    }

    /// Parse `DROP TABLE [IF EXISTS] name, ...`
    fn parse_drop_table(&mut self, _line: usize) -> Statement {
        self.next_token(); // consume TABLE
        if self.eat(TokenKind::If) {
            let _ = self.eat(TokenKind::Exists);
        }
        let mut names = Vec::new();
        while let Ok(id) = self.parse_ident() {
            names.push(id);
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.skip_until_semi();
        Statement::DropTable(names)
    }

    /// Parse `INSERT INTO name [(cols)] VALUES (...), (...)` or `INSERT INTO name [(cols)] SELECT ...`
    fn parse_insert(&mut self, line: usize) -> Statement {
        self.next_token(); // consume INSERT
        let _ = self.eat(TokenKind::Into);
        let table_name = self.parse_ident().unwrap_or_else(|_| "unknown".to_string());

        let mut columns = Vec::new();
        if self.peek_kind() == TokenKind::LParen {
            columns = self.parse_ident_list();
        }

        let mut values = Vec::new();
        let mut select_query = None;

        if self.eat(TokenKind::Values) {
            // Multiple rows: (r1), (r2)
            while self.eat(TokenKind::LParen) {
                let mut row = Vec::new();
                while self.peek_kind() != TokenKind::RParen && self.peek_kind() != TokenKind::Eof {
                    row.push(self.parse_expr());
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                let _ = self.eat(TokenKind::RParen);
                values.push(row);
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        } else if self.peek_kind() == TokenKind::Select {
            select_query = Some(Box::new(self.parse_select()));
        }

        self.skip_until_semi();
        Statement::Insert(InsertStmt {
            table_name,
            columns,
            values,
            select_query,
            line,
        })
    }

    /// Parse `UPDATE name SET col = expr, ... [WHERE condition]`
    fn parse_update(&mut self, line: usize) -> Statement {
        self.next_token(); // consume UPDATE
        let table_name = self.parse_ident().unwrap_or_else(|_| "unknown".to_string());
        let mut assignments = Vec::new();

        if self.eat(TokenKind::Set) {
            while self.peek_kind() != TokenKind::Where
                && self.peek_kind() != TokenKind::Semi
                && self.peek_kind() != TokenKind::Eof
            {
                if let Ok(col) = self.parse_ident() {
                    if self.eat(TokenKind::Eq) {
                        let expr = self.parse_expr();
                        assignments.push((col, expr));
                    }
                } else {
                    self.next_token();
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }

        let mut where_clause = None;
        if self.eat(TokenKind::Where) {
            where_clause = Some(self.parse_expr());
        }

        self.skip_until_semi();
        Statement::Update(UpdateStmt {
            table_name,
            assignments,
            where_clause,
            line,
        })
    }

    /// Parse `DELETE FROM name [WHERE condition]`
    fn parse_delete(&mut self, line: usize) -> Statement {
        self.next_token(); // consume DELETE
        let _ = self.eat(TokenKind::From);
        let table_name = self.parse_ident().unwrap_or_else(|_| "unknown".to_string());

        let mut where_clause = None;
        if self.eat(TokenKind::Where) {
            where_clause = Some(self.parse_expr());
        }

        self.skip_until_semi();
        Statement::Delete(DeleteStmt {
            table_name,
            where_clause,
            line,
        })
    }

    /// Parse `SELECT ...` or `WITH ... SELECT ...`
    pub fn parse_select(&mut self) -> SelectStmt {
        let line = self.peek().line;
        let mut with_cte = None;

        if self.eat(TokenKind::With) {
            let recursive = self.eat(TokenKind::Recursive);
            let mut ctes = Vec::new();
            while self.peek_kind() != TokenKind::Select && self.peek_kind() != TokenKind::Eof {
                let cte_line = self.peek().line;
                let cte_name = self.parse_ident().unwrap_or_default();
                let columns = if self.peek_kind() == TokenKind::LParen {
                    self.parse_ident_list()
                } else {
                    Vec::new()
                };
                let _ = self.eat(TokenKind::As);
                let _ = self.eat(TokenKind::LParen);
                let query = self.parse_select();
                let _ = self.eat(TokenKind::RParen);
                ctes.push(CteDef {
                    name: cte_name,
                    columns,
                    query,
                    line: cte_line,
                });
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            with_cte = Some(WithClause { recursive, ctes });
        }

        let _ = self.eat(TokenKind::Select);
        let distinct = self.eat(TokenKind::Distinct);

        // Projections
        let mut projections = Vec::new();
        while self.peek_kind() != TokenKind::From
            && self.peek_kind() != TokenKind::Semi
            && self.peek_kind() != TokenKind::RParen
            && self.peek_kind() != TokenKind::Eof
        {
            let expr = self.parse_expr();
            let mut alias = None;
            if self.eat(TokenKind::As) {
                alias = self.parse_ident().ok();
            } else if let TokenKind::Ident(ref s) = self.peek_kind() {
                if !is_reserved_query_keyword(s) {
                    alias = self.parse_ident().ok();
                }
            }
            projections.push(SelectItem { expr, alias });
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }

        // FROM
        let mut from = Vec::new();
        if self.eat(TokenKind::From) {
            while self.peek_kind() != TokenKind::Where
                && self.peek_kind() != TokenKind::Group
                && self.peek_kind() != TokenKind::Having
                && self.peek_kind() != TokenKind::Order
                && self.peek_kind() != TokenKind::Limit
                && self.peek_kind() != TokenKind::Union
                && self.peek_kind() != TokenKind::Semi
                && self.peek_kind() != TokenKind::RParen
                && self.peek_kind() != TokenKind::Eof
            {
                if self.is_join_start() {
                    break;
                }
                if let Ok(name) = self.parse_ident() {
                    let mut alias = None;
                    if self.eat(TokenKind::As) {
                        alias = self.parse_ident().ok();
                    } else if let TokenKind::Ident(ref s) = self.peek_kind() {
                        if !is_reserved_query_keyword(s) {
                            alias = self.parse_ident().ok();
                        }
                    }
                    from.push(TableRef { name, alias });
                } else {
                    self.next_token();
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }

        // JOINs
        let mut joins = Vec::new();
        while self.is_join_start() {
            let join_type = self.parse_join_type();
            let tbl_name = self.parse_ident().unwrap_or_default();
            let mut alias = None;
            if self.eat(TokenKind::As) {
                alias = self.parse_ident().ok();
            } else if let TokenKind::Ident(ref s) = self.peek_kind() {
                if !is_reserved_query_keyword(s) {
                    alias = self.parse_ident().ok();
                }
            }
            let mut on_clause = None;
            if self.eat(TokenKind::On) {
                on_clause = Some(self.parse_expr());
            }
            joins.push(JoinClause {
                join_type,
                table: TableRef {
                    name: tbl_name,
                    alias,
                },
                on_clause,
            });
        }

        // WHERE
        let mut where_clause = None;
        if self.eat(TokenKind::Where) {
            where_clause = Some(self.parse_expr());
        }

        // GROUP BY
        let mut group_by = Vec::new();
        if self.eat(TokenKind::Group) && self.eat(TokenKind::By) {
            while self.peek_kind() != TokenKind::Having
                && self.peek_kind() != TokenKind::Order
                && self.peek_kind() != TokenKind::Limit
                && self.peek_kind() != TokenKind::Union
                && self.peek_kind() != TokenKind::Semi
                && self.peek_kind() != TokenKind::RParen
                && self.peek_kind() != TokenKind::Eof
            {
                group_by.push(self.parse_expr());
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }

        // HAVING
        let mut having = None;
        if self.eat(TokenKind::Having) {
            having = Some(self.parse_expr());
        }

        // UNION [ALL]
        let mut union_all = None;
        let mut union = None;
        if self.eat(TokenKind::Union) {
            if self.eat(TokenKind::All) {
                union_all = Some(Box::new(self.parse_select()));
            } else {
                union = Some(Box::new(self.parse_select()));
            }
        }

        // ORDER BY
        let mut order_by = Vec::new();
        if self.eat(TokenKind::Order) && self.eat(TokenKind::By) {
            while self.peek_kind() != TokenKind::Limit
                && self.peek_kind() != TokenKind::Semi
                && self.peek_kind() != TokenKind::RParen
                && self.peek_kind() != TokenKind::Eof
            {
                let expr = self.parse_expr();
                let asc = if self.eat(TokenKind::Desc) {
                    false
                } else {
                    let _ = self.eat(TokenKind::Asc);
                    true
                };
                order_by.push(OrderByExpr { expr, asc });
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
        }

        // LIMIT
        let mut limit = None;
        if self.eat(TokenKind::Limit) {
            limit = Some(self.parse_expr());
        }

        SelectStmt {
            with_cte,
            distinct,
            projections,
            from,
            joins,
            where_clause,
            group_by,
            having,
            union_all,
            union,
            order_by,
            limit,
            line,
        }
    }

    fn is_join_start(&mut self) -> bool {
        matches!(
            self.peek_kind(),
            TokenKind::Join
                | TokenKind::Inner
                | TokenKind::Left
                | TokenKind::Right
                | TokenKind::Full
                | TokenKind::Cross
                | TokenKind::Natural
        )
    }

    fn parse_join_type(&mut self) -> String {
        let mut s = String::new();
        while self.is_join_start() {
            let tok = self.next_token();
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(&format!("{:?}", tok.kind).to_ascii_uppercase());
            if tok.kind == TokenKind::Join {
                break;
            }
        }
        s
    }

    /// Parse `CREATE VIEW name AS select`
    fn parse_create_view(&mut self, line: usize) -> Statement {
        self.next_token(); // consume VIEW
        let name = self.parse_ident().unwrap_or_else(|_| "unknown".to_string());
        let _ = self.eat(TokenKind::As);
        let query = self.parse_select();
        self.skip_until_semi();
        Statement::CreateView(ViewDef { name, query, line })
    }

    /// Parse `CREATE TRIGGER name ...`
    fn parse_create_trigger(&mut self, line: usize) -> Statement {
        self.next_token(); // consume TRIGGER
        let name = self.parse_ident().unwrap_or_else(|_| "unknown".to_string());
        let timing = if self.eat(TokenKind::Before) {
            "BEFORE".to_string()
        } else if self.eat(TokenKind::After) {
            "AFTER".to_string()
        } else {
            "TRIGGER".to_string()
        };

        let event = if self.eat(TokenKind::Insert) {
            "INSERT".to_string()
        } else if self.eat(TokenKind::Update) {
            "UPDATE".to_string()
        } else if self.eat(TokenKind::Delete) {
            "DELETE".to_string()
        } else {
            "".to_string()
        };

        let mut table_name = String::new();
        if self.eat(TokenKind::On) {
            table_name = self.parse_ident().unwrap_or_default();
        }

        let mut body = Vec::new();
        // Trigger body: parse inner statements if BEGIN ... END
        if self.eat(TokenKind::Begin) {
            while self.peek_kind() != TokenKind::End && self.peek_kind() != TokenKind::Eof {
                if let Some(inner) = self.next_statement() {
                    body.push(inner);
                } else {
                    break;
                }
            }
            let _ = self.eat(TokenKind::End);
        }

        self.skip_until_semi();
        Statement::CreateTrigger(TriggerDef {
            name,
            table_name,
            timing,
            event,
            body,
            line,
        })
    }

    /// Parse `CREATE FUNCTION name ...`
    fn parse_create_function(&mut self, line: usize) -> Statement {
        self.next_token(); // consume FUNCTION
        let name = self.parse_ident().unwrap_or_else(|_| "unknown".to_string());
        let mut params = Vec::new();
        if self.peek_kind() == TokenKind::LParen {
            params = self.parse_ident_list();
        }
        let mut return_type = None;
        if self.eat(TokenKind::Returns) || self.eat(TokenKind::Return) {
            return_type = self.parse_ident().ok();
        }

        let mut body = Vec::new();
        // Function body may contain statements
        if self.eat(TokenKind::Begin) {
            while self.peek_kind() != TokenKind::End && self.peek_kind() != TokenKind::Eof {
                if let Some(inner) = self.next_statement() {
                    body.push(inner);
                } else {
                    break;
                }
            }
            let _ = self.eat(TokenKind::End);
        }

        self.skip_until_semi();
        Statement::CreateFunction(RoutineDef {
            name,
            params,
            return_type,
            body,
            line,
        })
    }

    /// Parse `CREATE PROCEDURE name ...`
    fn parse_create_procedure(&mut self, line: usize) -> Statement {
        self.next_token(); // consume PROCEDURE
        let name = self.parse_ident().unwrap_or_else(|_| "unknown".to_string());
        let mut params = Vec::new();
        if self.peek_kind() == TokenKind::LParen {
            params = self.parse_ident_list();
        }

        let mut body = Vec::new();
        if self.eat(TokenKind::Begin) {
            while self.peek_kind() != TokenKind::End && self.peek_kind() != TokenKind::Eof {
                if let Some(inner) = self.next_statement() {
                    body.push(inner);
                } else {
                    break;
                }
            }
            let _ = self.eat(TokenKind::End);
        }

        self.skip_until_semi();
        Statement::CreateProcedure(RoutineDef {
            name,
            params,
            return_type: None,
            body,
            line,
        })
    }

    // ─── Expression Parsing ──────────────────────────────────────────────────

    pub fn parse_expr(&mut self) -> Expr {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Expr {
        let mut left = self.parse_and_expr();
        while self.eat(TokenKind::Or) {
            let line = self.peek().line;
            let right = self.parse_and_expr();
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOpKind::Or,
                right: Box::new(right),
                line,
            };
        }
        left
    }

    fn parse_and_expr(&mut self) -> Expr {
        let mut left = self.parse_not_expr();
        while self.eat(TokenKind::And) {
            let line = self.peek().line;
            let right = self.parse_not_expr();
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOpKind::And,
                right: Box::new(right),
                line,
            };
        }
        left
    }

    fn parse_not_expr(&mut self) -> Expr {
        let line = self.peek().line;
        if self.eat(TokenKind::Not) {
            let inner = self.parse_comparison_expr();
            Expr::UnaryOp {
                op: UnaryOpKind::Not,
                expr: Box::new(inner),
                line,
            }
        } else {
            self.parse_comparison_expr()
        }
    }

    fn parse_comparison_expr(&mut self) -> Expr {
        let mut left = self.parse_additive_expr();

        // Check for IS [NOT] NULL
        if self.eat(TokenKind::Is) {
            let line = self.peek().line;
            let is_not = self.eat(TokenKind::Not);
            let _ = self.eat(TokenKind::Null);
            return Expr::IsNull {
                expr: Box::new(left),
                is_not,
                line,
            };
        }

        // Check for [NOT] IN (list or subquery)
        let is_not_in = self.eat(TokenKind::Not);
        if self.eat(TokenKind::In) {
            let line = self.peek().line;
            if self.eat(TokenKind::LParen) {
                if self.peek_kind() == TokenKind::Select {
                    let subquery = self.parse_select();
                    let _ = self.eat(TokenKind::RParen);
                    return Expr::InSubquery {
                        expr: Box::new(left),
                        subquery: Box::new(subquery),
                        is_not: is_not_in,
                        line,
                    };
                } else {
                    let mut list = Vec::new();
                    while self.peek_kind() != TokenKind::RParen && self.peek_kind() != TokenKind::Eof {
                        list.push(self.parse_expr());
                        if !self.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                    let _ = self.eat(TokenKind::RParen);
                    return Expr::InList {
                        expr: Box::new(left),
                        list,
                        is_not: is_not_in,
                        line,
                    };
                }
            }
        }

        // Check for [NOT] BETWEEN low AND high
        if self.eat(TokenKind::Between) {
            let line = self.peek().line;
            let low = self.parse_additive_expr();
            let _ = self.eat(TokenKind::And);
            let high = self.parse_additive_expr();
            return Expr::Between {
                expr: Box::new(left),
                low: Box::new(low),
                high: Box::new(high),
                is_not: is_not_in,
                line,
            };
        }

        // Comparison operators: =, <>, !=, <, <=, >, >=, LIKE
        let line = self.peek().line;
        let op = match self.peek_kind() {
            TokenKind::Eq => Some(BinaryOpKind::Eq),
            TokenKind::NotEq => Some(BinaryOpKind::NotEq),
            TokenKind::Lt => Some(BinaryOpKind::Lt),
            TokenKind::LtEq => Some(BinaryOpKind::LtEq),
            TokenKind::Gt => Some(BinaryOpKind::Gt),
            TokenKind::GtEq => Some(BinaryOpKind::GtEq),
            TokenKind::Like => Some(BinaryOpKind::Like),
            _ => None,
        };

        if let Some(bop) = op {
            self.next_token();
            let right = self.parse_additive_expr();
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: bop,
                right: Box::new(right),
                line,
            };
        }

        left
    }

    fn parse_additive_expr(&mut self) -> Expr {
        let mut left = self.parse_multiplicative_expr();
        while matches!(
            self.peek_kind(),
            TokenKind::Plus | TokenKind::Minus | TokenKind::Concat
        ) {
            let line = self.peek().line;
            let tok = self.next_token();
            let op = match tok.kind {
                TokenKind::Plus => BinaryOpKind::Add,
                TokenKind::Minus => BinaryOpKind::Sub,
                TokenKind::Concat => BinaryOpKind::Concat,
                _ => unreachable!(),
            };
            let right = self.parse_multiplicative_expr();
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                line,
            };
        }
        left
    }

    fn parse_multiplicative_expr(&mut self) -> Expr {
        let mut left = self.parse_unary_expr();
        while matches!(
            self.peek_kind(),
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent
        ) {
            let line = self.peek().line;
            let tok = self.next_token();
            let op = match tok.kind {
                TokenKind::Star => BinaryOpKind::Mul,
                TokenKind::Slash => BinaryOpKind::Div,
                TokenKind::Percent => BinaryOpKind::Mod,
                _ => unreachable!(),
            };
            let right = self.parse_unary_expr();
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                line,
            };
        }
        left
    }

    fn parse_unary_expr(&mut self) -> Expr {
        let line = self.peek().line;
        if self.eat(TokenKind::Plus) {
            let expr = self.parse_postfix_expr();
            Expr::UnaryOp {
                op: UnaryOpKind::Plus,
                expr: Box::new(expr),
                line,
            }
        } else if self.eat(TokenKind::Minus) {
            let expr = self.parse_postfix_expr();
            Expr::UnaryOp {
                op: UnaryOpKind::Minus,
                expr: Box::new(expr),
                line,
            }
        } else {
            self.parse_postfix_expr()
        }
    }

    fn parse_postfix_expr(&mut self) -> Expr {
        let expr = self.parse_primary_expr().unwrap_or(Expr::Raw("unknown".to_string()));
        // PostgreSQL cast: expr::type
        while self.eat(TokenKind::DoubleColon) {
            let _ty = self.parse_ident().unwrap_or_default();
        }
        expr
    }

    fn parse_primary_expr(&mut self) -> Option<Expr> {
        let tok = self.peek().clone();
        match tok.kind {
            TokenKind::Null => {
                self.next_token();
                Some(Expr::Literal(Literal::Null))
            }
            TokenKind::IntLit(n) => {
                self.next_token();
                Some(Expr::Literal(Literal::Integer(n)))
            }
            TokenKind::FloatLit(f) => {
                self.next_token();
                Some(Expr::Literal(Literal::Float(f)))
            }
            TokenKind::StringLit(ref s) => {
                self.next_token();
                Some(Expr::Literal(Literal::String(s.clone())))
            }
            TokenKind::True => {
                self.next_token();
                Some(Expr::Literal(Literal::Boolean(true)))
            }
            TokenKind::False => {
                self.next_token();
                Some(Expr::Literal(Literal::Boolean(false)))
            }
            TokenKind::Case => {
                Some(self.parse_case_expr())
            }
            TokenKind::Nullif | TokenKind::Coalesce => {
                self.next_token();
                let name = match tok.kind {
                    TokenKind::Nullif => "nullif".to_string(),
                    TokenKind::Coalesce => "coalesce".to_string(),
                    _ => unreachable!(),
                };
                let args = self.parse_call_args();
                Some(Expr::FunctionCall {
                    name,
                    args,
                    line: tok.line,
                })
            }
            TokenKind::Cast => {
                self.next_token();
                if self.eat(TokenKind::LParen) {
                    let inner = self.parse_expr();
                    let _ = self.eat(TokenKind::As);
                    let _ = self.parse_ident();
                    let _ = self.eat(TokenKind::RParen);
                    Some(inner)
                } else {
                    None
                }
            }
            TokenKind::LParen => {
                self.next_token();
                if self.peek_kind() == TokenKind::Select || self.peek_kind() == TokenKind::With {
                    let subquery = self.parse_select();
                    let _ = self.eat(TokenKind::RParen);
                    Some(Expr::Subquery(Box::new(subquery)))
                } else {
                    let expr = self.parse_expr();
                    let _ = self.eat(TokenKind::RParen);
                    Some(expr)
                }
            }
            TokenKind::Ident(ref name) => {
                let id = name.clone();
                self.next_token();
                // Check if function call
                if self.peek_kind() == TokenKind::LParen {
                    let args = self.parse_call_args();
                    Some(Expr::FunctionCall {
                        name: id,
                        args,
                        line: tok.line,
                    })
                } else if self.eat(TokenKind::Dot) {
                    // table.column
                    let col = self.parse_ident().unwrap_or_default();
                    Some(Expr::ColumnRef {
                        table: Some(id),
                        column: col,
                        line: tok.line,
                    })
                } else {
                    Some(Expr::ColumnRef {
                        table: None,
                        column: id,
                        line: tok.line,
                    })
                }
            }
            TokenKind::Star => {
                self.next_token();
                Some(Expr::Raw("*".to_string()))
            }
            _ => None,
        }
    }

    fn parse_case_expr(&mut self) -> Expr {
        let line = self.peek().line;
        self.next_token(); // consume CASE

        let mut operand = None;
        if self.peek_kind() != TokenKind::When {
            operand = Some(Box::new(self.parse_expr()));
        }

        let mut when_clauses = Vec::new();
        while self.eat(TokenKind::When) {
            let cond = self.parse_expr();
            let _ = self.eat(TokenKind::Then);
            let res = self.parse_expr();
            when_clauses.push((cond, res));
        }

        let mut else_clause = None;
        if self.eat(TokenKind::Else) {
            else_clause = Some(Box::new(self.parse_expr()));
        }

        let _ = self.eat(TokenKind::End);

        Expr::Case {
            operand,
            when_clauses,
            else_clause,
            line,
        }
    }

    fn parse_call_args(&mut self) -> Vec<Expr> {
        let mut args = Vec::new();
        if self.eat(TokenKind::LParen) {
            while self.peek_kind() != TokenKind::RParen && self.peek_kind() != TokenKind::Eof {
                args.push(self.parse_expr());
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            let _ = self.eat(TokenKind::RParen);
        }
        args
    }

    fn parse_ident(&mut self) -> Result<String, String> {
        let tok = self.next_token();
        match tok.kind {
            TokenKind::Ident(s) => Ok(s),
            TokenKind::StringLit(s) => Ok(s),
            _ => Ok(format!("{:?}", tok.kind).to_ascii_lowercase()),
        }
    }

    fn parse_ident_list(&mut self) -> Vec<String> {
        let mut list = Vec::new();
        if self.eat(TokenKind::LParen) {
            while self.peek_kind() != TokenKind::RParen && self.peek_kind() != TokenKind::Eof {
                if let Ok(id) = self.parse_ident() {
                    list.push(id);
                }
                if !self.eat(TokenKind::Comma) {
                    break;
                }
            }
            let _ = self.eat(TokenKind::RParen);
        }
        list
    }
}

fn is_reserved_query_keyword(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "from"
            | "where"
            | "group"
            | "having"
            | "order"
            | "limit"
            | "union"
            | "join"
            | "inner"
            | "left"
            | "right"
            | "full"
            | "cross"
            | "on"
            | "using"
    )
}
