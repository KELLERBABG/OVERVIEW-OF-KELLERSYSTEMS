//! Recursive descent C parser with error recovery and operator precedence.

use crate::c::ast::*;
use crate::c::lexer::{Token, TokenKind};
use std::collections::HashSet;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    typedefs: HashSet<String>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        let mut typedefs = HashSet::new();
        // Standard C typedefs
        typedefs.insert("size_t".into());
        typedefs.insert("ssize_t".into());
        typedefs.insert("int8_t".into());
        typedefs.insert("int16_t".into());
        typedefs.insert("int32_t".into());
        typedefs.insert("int64_t".into());
        typedefs.insert("uint8_t".into());
        typedefs.insert("uint16_t".into());
        typedefs.insert("uint32_t".into());
        typedefs.insert("uint64_t".into());
        typedefs.insert("intptr_t".into());
        typedefs.insert("uintptr_t".into());
        typedefs.insert("bool".into());
        typedefs.insert("FILE".into());
        typedefs.insert("div_t".into());
        typedefs.insert("ldiv_t".into());

        Self {
            tokens,
            pos: 0,
            typedefs,
        }
    }

    fn peek(&self) -> &TokenKind {
        self.tokens.get(self.pos).map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
    }

    fn peek_token(&self) -> &Token {
        static EOF_TOKEN: Token = Token {
            kind: TokenKind::Eof,
            line: 0,
            col: 0,
        };
        self.tokens.get(self.pos).unwrap_or(&EOF_TOKEN)
    }

    fn peek_next(&self) -> &TokenKind {
        self.tokens.get(self.pos + 1).map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
    }

    fn current_line(&self) -> usize {
        self.peek_token().line
    }

    fn advance(&mut self) -> Token {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            tok
        } else {
            Token {
                kind: TokenKind::Eof,
                line: self.current_line(),
                col: 0,
            }
        }
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.peek() == kind {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<Token, String> {
        if self.peek() == kind {
            Ok(self.advance())
        } else {
            Err(format!(
                "expected {:?}, got {:?} at line {}",
                kind,
                self.peek(),
                self.current_line()
            ))
        }
    }

    fn skip_attributes(&mut self) {
        while self.peek() == &TokenKind::Attribute {
            self.advance();
            if self.match_token(&TokenKind::LParen) {
                let mut depth = 1;
                while depth > 0 && self.peek() != &TokenKind::Eof {
                    match self.advance().kind {
                        TokenKind::LParen => depth += 1,
                        TokenKind::RParen => depth -= 1,
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn parse_translation_unit(&mut self) -> TranslationUnit {
        let mut tu = TranslationUnit::default();

        while self.peek() != &TokenKind::Eof {
            self.skip_attributes();
            if self.peek() == &TokenKind::Semicolon {
                self.advance();
                continue;
            }

            // Check for typedef
            if self.peek() == &TokenKind::Typedef {
                self.parse_typedef(&mut tu);
                continue;
            }

            // Check for struct / union / enum definition or declaration
            if (self.peek() == &TokenKind::Struct || self.peek() == &TokenKind::Union)
                && self.pos + 2 < self.tokens.len()
            {
                if let TokenKind::Ident(_) = &self.tokens[self.pos + 1].kind {
                    if self.tokens[self.pos + 2].kind == TokenKind::LBrace {
                        if let Some(st) = self.parse_struct_def() {
                            tu.structs.push(st);
                            continue;
                        }
                    }
                }
            }

            // Parse top-level declaration (function or global variable)
            let start_pos = self.pos;
            match self.parse_top_level_item() {
                Ok(TopLevelItem::Func(f)) => {
                    tu.functions.push(f);
                }
                Ok(TopLevelItem::Global(vars)) => {
                    tu.globals.extend(vars);
                }
                Ok(TopLevelItem::None) => {}
                Err(_) => {
                    // Recovery: skip until semicolon or closing brace
                    self.synchronize_top_level(start_pos);
                }
            }
        }

        tu
    }

    fn synchronize_top_level(&mut self, start_pos: usize) {
        if self.pos == start_pos {
            self.advance();
        }
        let mut brace_depth = 0;
        while self.peek() != &TokenKind::Eof {
            match self.peek() {
                TokenKind::LBrace => {
                    brace_depth += 1;
                    self.advance();
                }
                TokenKind::RBrace => {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            self.advance();
                            break;
                        }
                    }
                    self.advance();
                }
                TokenKind::Semicolon => {
                    if brace_depth == 0 {
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

    fn parse_typedef(&mut self, tu: &mut TranslationUnit) {
        self.advance(); // consume typedef
        self.skip_attributes();

        // Could be typedef struct { ... } Name;
        if self.peek() == &TokenKind::Struct && self.peek_next() == &TokenKind::LBrace {
            self.advance(); // struct
            let line = self.current_line();
            let fields = self.parse_struct_fields();
            if let TokenKind::Ident(name) = self.peek().clone() {
                self.advance();
                self.typedefs.insert(name.clone());
                tu.structs.push(StructDef {
                    name,
                    fields,
                    line,
                });
            }
            let _ = self.match_token(&TokenKind::Semicolon);
            return;
        }

        if let Ok(_ty) = self.parse_type() {
            if let TokenKind::Ident(name) = self.peek().clone() {
                self.advance();
                self.typedefs.insert(name);
            }
        }
        // Consume up to semicolon
        while self.peek() != &TokenKind::Semicolon && self.peek() != &TokenKind::Eof {
            self.advance();
        }
        let _ = self.match_token(&TokenKind::Semicolon);
    }

    fn parse_struct_def(&mut self) -> Option<StructDef> {
        let line = self.current_line();
        self.advance(); // struct or union
        let name = match self.advance().kind {
            TokenKind::Ident(n) => n,
            _ => return None,
        };

        if !self.match_token(&TokenKind::LBrace) {
            return None;
        }

        let fields = self.parse_struct_fields();
        self.skip_attributes();
        let _ = self.match_token(&TokenKind::Semicolon);

        Some(StructDef { name, fields, line })
    }

    fn parse_struct_fields(&mut self) -> Vec<VarDecl> {
        let mut fields = Vec::new();
        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            if let Ok(base_ty) = self.parse_type() {
                while self.peek() != &TokenKind::Semicolon && self.peek() != &TokenKind::Eof {
                    let mut field_ty = base_ty.clone();
                    while self.match_token(&TokenKind::Star) {
                        field_ty = Type::Pointer(Box::new(field_ty));
                    }
                    if let TokenKind::Ident(name) = self.peek().clone() {
                        let line = self.current_line();
                        self.advance();
                        if self.match_token(&TokenKind::LBracket) {
                            let size = if let TokenKind::IntLit(n) = self.peek() {
                                let sz = *n as usize;
                                self.advance();
                                Some(sz)
                            } else {
                                None
                            };
                            let _ = self.match_token(&TokenKind::RBracket);
                            field_ty = Type::Array(Box::new(field_ty), size);
                        }
                        fields.push(VarDecl {
                            name,
                            ty: field_ty,
                            init: None,
                            line,
                        });
                    }
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                let _ = self.match_token(&TokenKind::Semicolon);
            } else {
                self.advance();
            }
        }
        let _ = self.match_token(&TokenKind::RBrace);
        fields
    }

    fn parse_top_level_item(&mut self) -> Result<TopLevelItem, String> {
        let is_static = self.match_token(&TokenKind::Static);
        let _ = self.match_token(&TokenKind::Extern);
        let _ = self.match_token(&TokenKind::Inline);
        self.skip_attributes();

        let base_ty = self.parse_type()?;
        let mut ty = base_ty.clone();

        // Pointer stars on the name
        while self.match_token(&TokenKind::Star) {
            ty = Type::Pointer(Box::new(ty));
        }

        self.skip_attributes();

        let name = match self.advance().kind {
            TokenKind::Ident(n) => n,
            TokenKind::Semicolon => return Ok(TopLevelItem::None),
            other => return Err(format!("expected identifier at top level, got {:?}", other)),
        };

        // Check if function
        if self.match_token(&TokenKind::LParen) {
            let line = self.current_line();
            let params = self.parse_params()?;
            self.expect(&TokenKind::RParen)?;
            self.skip_attributes();

            // Function definition or declaration?
            if self.peek() == &TokenKind::LBrace {
                let body = self.parse_block()?;
                if let Stmt::Block(stmts, _) = body {
                    return Ok(TopLevelItem::Func(Function {
                        name,
                        ret_ty: ty,
                        params,
                        body: Some(stmts),
                        is_static,
                        line,
                    }));
                }
            } else if self.match_token(&TokenKind::Semicolon) {
                return Ok(TopLevelItem::Func(Function {
                    name,
                    ret_ty: ty,
                    params,
                    body: None,
                    is_static,
                    line,
                }));
            }
        }

        // Global variable
        let mut globals = Vec::new();
        let var_line = self.current_line();
        let mut var_ty = ty;

        if self.match_token(&TokenKind::LBracket) {
            let size = if let TokenKind::IntLit(n) = self.peek() {
                let sz = *n as usize;
                self.advance();
                Some(sz)
            } else {
                None
            };
            let _ = self.match_token(&TokenKind::RBracket);
            var_ty = Type::Array(Box::new(var_ty), size);
        }

        let init = if self.match_token(&TokenKind::Eq) {
            Some(self.parse_initializer()?)
        } else {
            None
        };

        globals.push(VarDecl {
            name,
            ty: var_ty,
            init,
            line: var_line,
        });

        while self.match_token(&TokenKind::Comma) {
            let mut g_ty = base_ty.clone();
            while self.match_token(&TokenKind::Star) {
                g_ty = Type::Pointer(Box::new(g_ty));
            }
            if let TokenKind::Ident(g_name) = self.advance().kind {
                let l = self.current_line();
                if self.match_token(&TokenKind::LBracket) {
                    let size = if let TokenKind::IntLit(n) = self.peek() {
                        let sz = *n as usize;
                        self.advance();
                        Some(sz)
                    } else {
                        None
                    };
                    let _ = self.match_token(&TokenKind::RBracket);
                    g_ty = Type::Array(Box::new(g_ty), size);
                }
                let g_init = if self.match_token(&TokenKind::Eq) {
                    Some(self.parse_initializer()?)
                } else {
                    None
                };
                globals.push(VarDecl {
                    name: g_name,
                    ty: g_ty,
                    init: g_init,
                    line: l,
                });
            }
        }

        let _ = self.match_token(&TokenKind::Semicolon);
        Ok(TopLevelItem::Global(globals))
    }

    fn parse_params(&mut self) -> Result<Vec<VarDecl>, String> {
        let mut params = Vec::new();
        if self.peek() == &TokenKind::RParen {
            return Ok(params);
        }

        if self.peek() == &TokenKind::Void && self.peek_next() == &TokenKind::RParen {
            self.advance(); // consume void
            return Ok(params);
        }

        loop {
            let _ = self.match_token(&TokenKind::Const);
            let _ = self.match_token(&TokenKind::Volatile);
            let _ = self.match_token(&TokenKind::Restrict);

            let mut ty = self.parse_type()?;
            while self.match_token(&TokenKind::Star) {
                ty = Type::Pointer(Box::new(ty));
            }

            let name = if let TokenKind::Ident(n) = self.peek().clone() {
                self.advance();
                n
            } else {
                format!("arg{}", params.len())
            };

            // Array notation in parameter `int arr[]` or `int arr[10]`
            if self.match_token(&TokenKind::LBracket) {
                let size = if let TokenKind::IntLit(n) = self.peek() {
                    let sz = *n as usize;
                    self.advance();
                    Some(sz)
                } else {
                    None
                };
                let _ = self.match_token(&TokenKind::RBracket);
                ty = Type::Array(Box::new(ty), size);
            }

            params.push(VarDecl {
                name,
                ty,
                init: None,
                line: self.current_line(),
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        Ok(params)
    }

    pub fn parse_type(&mut self) -> Result<Type, String> {
        let _ = self.match_token(&TokenKind::Const);
        let _ = self.match_token(&TokenKind::Volatile);

        let mut is_unsigned = false;
        if self.match_token(&TokenKind::Unsigned) {
            is_unsigned = true;
        } else {
            let _ = self.match_token(&TokenKind::Signed);
        }

        let base = match self.peek() {
            TokenKind::Void => {
                self.advance();
                Type::Base(BaseType::Void, false)
            }
            TokenKind::Bool => {
                self.advance();
                Type::Base(BaseType::Bool, false)
            }
            TokenKind::Char => {
                self.advance();
                Type::Base(BaseType::Char, is_unsigned)
            }
            TokenKind::Short => {
                self.advance();
                let _ = self.match_token(&TokenKind::Int);
                Type::Base(BaseType::Short, is_unsigned)
            }
            TokenKind::Int => {
                self.advance();
                Type::Base(BaseType::Int, is_unsigned)
            }
            TokenKind::Long => {
                self.advance();
                if self.match_token(&TokenKind::Long) {
                    let _ = self.match_token(&TokenKind::Int);
                    Type::Base(BaseType::LongLong, is_unsigned)
                } else {
                    let _ = self.match_token(&TokenKind::Int);
                    Type::Base(BaseType::Long, is_unsigned)
                }
            }
            TokenKind::Float => {
                self.advance();
                Type::Base(BaseType::Float, false)
            }
            TokenKind::Double => {
                self.advance();
                Type::Base(BaseType::Double, false)
            }
            TokenKind::Struct => {
                self.advance();
                if let TokenKind::Ident(n) = self.peek().clone() {
                    self.advance();
                    Type::Struct(n)
                } else {
                    Type::Struct("<anonymous>".into())
                }
            }
            TokenKind::Union => {
                self.advance();
                if let TokenKind::Ident(n) = self.peek().clone() {
                    self.advance();
                    Type::Union(n)
                } else {
                    Type::Union("<anonymous>".into())
                }
            }
            TokenKind::Enum => {
                self.advance();
                if let TokenKind::Ident(n) = self.peek().clone() {
                    self.advance();
                    Type::Enum(n)
                } else {
                    Type::Enum("<anonymous>".into())
                }
            }
            TokenKind::Ident(name) => {
                let is_type = self.typedefs.contains(name) || {
                    if let Some(next) = self.tokens.get(self.pos + 1).map(|t| &t.kind) {
                        matches!(next, TokenKind::Star | TokenKind::Ident(_))
                    } else {
                        false
                    }
                };
                if is_type {
                    let n = name.clone();
                    self.advance();
                    self.typedefs.insert(n.clone());
                    Type::Typedef(n)
                } else if is_unsigned {
                    Type::Base(BaseType::Int, true)
                } else {
                    return Err(format!("expected type, got {:?}", name));
                }
            }
            _ if is_unsigned => Type::Base(BaseType::Int, true),
            other => return Err(format!("expected type, got {:?}", other)),
        };

        let _ = self.match_token(&TokenKind::Const);
        let _ = self.match_token(&TokenKind::Volatile);

        Ok(base)
    }

    fn is_type_start(&self) -> bool {
        match self.peek() {
            TokenKind::Void
            | TokenKind::Char
            | TokenKind::Short
            | TokenKind::Int
            | TokenKind::Long
            | TokenKind::Float
            | TokenKind::Double
            | TokenKind::Signed
            | TokenKind::Unsigned
            | TokenKind::Bool
            | TokenKind::Struct
            | TokenKind::Union
            | TokenKind::Enum
            | TokenKind::Const
            | TokenKind::Volatile
            | TokenKind::Static => true,
            TokenKind::Ident(name) => {
                if self.typedefs.contains(name) {
                    true
                } else if let Some(next) = self.tokens.get(self.pos + 1).map(|t| &t.kind) {
                    matches!(next, TokenKind::Star | TokenKind::Ident(_))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn parse_stmt(&mut self) -> Result<Stmt, String> {
        self.skip_attributes();
        match self.peek() {
            TokenKind::LBrace => self.parse_block(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::Do => self.parse_do_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Switch => self.parse_switch(),
            TokenKind::Case => self.parse_case(),
            TokenKind::Default => self.parse_default(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Break => {
                let l = self.current_line();
                self.advance();
                let _ = self.match_token(&TokenKind::Semicolon);
                Ok(Stmt::Break(l))
            }
            TokenKind::Continue => {
                let l = self.current_line();
                self.advance();
                let _ = self.match_token(&TokenKind::Semicolon);
                Ok(Stmt::Continue(l))
            }
            TokenKind::Goto => {
                let l = self.current_line();
                self.advance();
                if let TokenKind::Ident(lbl) = self.advance().kind {
                    let _ = self.match_token(&TokenKind::Semicolon);
                    Ok(Stmt::Goto(lbl, l))
                } else {
                    Err("expected label after goto".into())
                }
            }
            TokenKind::Semicolon => {
                let l = self.current_line();
                self.advance();
                Ok(Stmt::Empty(l))
            }
            _ if self.is_type_start() => self.parse_decl_stmt(),
            _ => {
                // Check if labeled statement: `ident :`
                if let TokenKind::Ident(lbl) = self.peek().clone() {
                    if self.peek_next() == &TokenKind::Colon {
                        let l = self.current_line();
                        self.advance(); // ident
                        self.advance(); // :
                        return Ok(Stmt::Label(lbl, l));
                    }
                }
                self.parse_expr_stmt()
            }
        }
    }

    fn parse_decl_stmt(&mut self) -> Result<Stmt, String> {
        let line = self.current_line();
        let _ = self.match_token(&TokenKind::Static);
        let _ = self.match_token(&TokenKind::Const);
        let base_ty = self.parse_type()?;
        let mut decls = Vec::new();

        loop {
            let mut ty = base_ty.clone();
            while self.match_token(&TokenKind::Star) {
                ty = Type::Pointer(Box::new(ty));
            }
            let name = match self.advance().kind {
                TokenKind::Ident(n) => n,
                other => return Err(format!("expected variable name, got {:?}", other)),
            };

            // Array dimension
            if self.match_token(&TokenKind::LBracket) {
                let size = if let TokenKind::IntLit(n) = self.peek() {
                    let sz = *n as usize;
                    self.advance();
                    Some(sz)
                } else if self.peek() != &TokenKind::RBracket {
                    // Variable length array or expression size
                    let _ = self.parse_expr();
                    None
                } else {
                    None
                };
                let _ = self.match_token(&TokenKind::RBracket);
                ty = Type::Array(Box::new(ty), size);
            }

            let init = if self.match_token(&TokenKind::Eq) {
                Some(self.parse_initializer()?)
            } else {
                None
            };

            decls.push(VarDecl {
                name,
                ty,
                init,
                line,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        let _ = self.match_token(&TokenKind::Semicolon);
        Ok(Stmt::Decl(decls, line))
    }

    fn parse_initializer(&mut self) -> Result<Expr, String> {
        let line = self.current_line();
        if self.match_token(&TokenKind::LBrace) {
            let mut list = Vec::new();
            while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                list.push(self.parse_initializer()?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            let _ = self.match_token(&TokenKind::RBrace);
            Ok(Expr::InitList(list, line))
        } else {
            self.parse_assignment()
        }
    }

    fn parse_expr_stmt(&mut self) -> Result<Stmt, String> {
        let line = self.current_line();
        let expr = self.parse_expr()?;
        let _ = self.match_token(&TokenKind::Semicolon);
        Ok(Stmt::Expr(expr, line))
    }

    fn parse_block(&mut self) -> Result<Stmt, String> {
        let line = self.current_line();
        self.expect(&TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            match self.parse_stmt() {
                Ok(s) => stmts.push(s),
                Err(_) => {
                    // Skip to semicolon or brace
                    while self.peek() != &TokenKind::Semicolon
                        && self.peek() != &TokenKind::RBrace
                        && self.peek() != &TokenKind::Eof
                    {
                        self.advance();
                    }
                    let _ = self.match_token(&TokenKind::Semicolon);
                }
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::Block(stmts, line))
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        let line = self.current_line();
        self.expect(&TokenKind::If)?;
        self.expect(&TokenKind::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;

        let then_branch = Box::new(self.parse_stmt()?);
        let else_branch = if self.match_token(&TokenKind::Else) {
            Some(Box::new(self.parse_stmt()?))
        } else {
            None
        };

        Ok(Stmt::If {
            cond,
            then_branch,
            else_branch,
            line,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        let line = self.current_line();
        self.expect(&TokenKind::While)?;
        self.expect(&TokenKind::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        let body = Box::new(self.parse_stmt()?);
        Ok(Stmt::While { cond, body, line })
    }

    fn parse_do_while(&mut self) -> Result<Stmt, String> {
        let line = self.current_line();
        self.expect(&TokenKind::Do)?;
        let body = Box::new(self.parse_stmt()?);
        self.expect(&TokenKind::While)?;
        self.expect(&TokenKind::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        let _ = self.match_token(&TokenKind::Semicolon);
        Ok(Stmt::DoWhile { body, cond, line })
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        let line = self.current_line();
        self.expect(&TokenKind::For)?;
        self.expect(&TokenKind::LParen)?;

        let init = if self.peek() == &TokenKind::Semicolon {
            self.advance();
            None
        } else if self.is_type_start() {
            Some(Box::new(self.parse_decl_stmt()?))
        } else {
            let expr = self.parse_expr()?;
            self.expect(&TokenKind::Semicolon)?;
            Some(Box::new(Stmt::Expr(expr, line)))
        };

        let cond = if self.peek() == &TokenKind::Semicolon {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(&TokenKind::Semicolon)?;

        let step = if self.peek() == &TokenKind::RParen {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(&TokenKind::RParen)?;

        let body = Box::new(self.parse_stmt()?);
        Ok(Stmt::For {
            init,
            cond,
            step,
            body,
            line,
        })
    }

    fn parse_switch(&mut self) -> Result<Stmt, String> {
        let line = self.current_line();
        self.expect(&TokenKind::Switch)?;
        self.expect(&TokenKind::LParen)?;
        let expr = self.parse_expr()?;
        self.expect(&TokenKind::RParen)?;
        let body = Box::new(self.parse_stmt()?);
        Ok(Stmt::Switch { expr, body, line })
    }

    fn parse_case(&mut self) -> Result<Stmt, String> {
        let line = self.current_line();
        self.expect(&TokenKind::Case)?;
        let expr = self.parse_expr()?;
        self.expect(&TokenKind::Colon)?;
        Ok(Stmt::Case(expr, line))
    }

    fn parse_default(&mut self) -> Result<Stmt, String> {
        let line = self.current_line();
        self.expect(&TokenKind::Default)?;
        self.expect(&TokenKind::Colon)?;
        Ok(Stmt::Default(line))
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        let line = self.current_line();
        self.expect(&TokenKind::Return)?;
        let expr = if self.peek() == &TokenKind::Semicolon {
            None
        } else {
            Some(self.parse_expr()?)
        };
        let _ = self.match_token(&TokenKind::Semicolon);
        Ok(Stmt::Return(expr, line))
    }

    // Expression parsing
    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_assignment()?;
        if self.match_token(&TokenKind::Comma) {
            let line = self.current_line();
            let mut list = vec![expr];
            loop {
                list.push(self.parse_assignment()?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            expr = Expr::Comma(list, line);
        }
        Ok(expr)
    }

    pub fn parse_assignment(&mut self) -> Result<Expr, String> {
        let lhs = self.parse_ternary()?;

        let op = match self.peek() {
            TokenKind::Eq => Some(AssignOp::Assign),
            TokenKind::PlusEq => Some(AssignOp::AddAssign),
            TokenKind::MinusEq => Some(AssignOp::SubAssign),
            TokenKind::StarEq => Some(AssignOp::MulAssign),
            TokenKind::SlashEq => Some(AssignOp::DivAssign),
            TokenKind::PercentEq => Some(AssignOp::RemAssign),
            TokenKind::AmpEq => Some(AssignOp::BitAndAssign),
            TokenKind::PipeEq => Some(AssignOp::BitOrAssign),
            TokenKind::CaretEq => Some(AssignOp::BitXorAssign),
            TokenKind::ShlEq => Some(AssignOp::ShlAssign),
            TokenKind::ShrEq => Some(AssignOp::ShrAssign),
            _ => None,
        };

        if let Some(assign_op) = op {
            let line = self.current_line();
            self.advance();
            let rhs = self.parse_assignment()?;
            Ok(Expr::Assign {
                op: assign_op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                line,
            })
        } else {
            Ok(lhs)
        }
    }

    fn parse_ternary(&mut self) -> Result<Expr, String> {
        let cond = self.parse_logical_or()?;
        if self.match_token(&TokenKind::Question) {
            let line = self.current_line();
            let then_expr = self.parse_expr()?;
            self.expect(&TokenKind::Colon)?;
            let else_expr = self.parse_ternary()?;
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
        let mut lhs = self.parse_logical_and()?;
        while self.match_token(&TokenKind::PipePipe) {
            let line = self.current_line();
            let rhs = self.parse_logical_and()?;
            lhs = Expr::Binary {
                op: BinaryOp::Or,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                line,
            };
        }
        Ok(lhs)
    }

    fn parse_logical_and(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_bitwise_or()?;
        while self.match_token(&TokenKind::AmpAmp) {
            let line = self.current_line();
            let rhs = self.parse_bitwise_or()?;
            lhs = Expr::Binary {
                op: BinaryOp::And,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                line,
            };
        }
        Ok(lhs)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_bitwise_xor()?;
        while self.match_token(&TokenKind::Pipe) {
            let line = self.current_line();
            let rhs = self.parse_bitwise_xor()?;
            lhs = Expr::Binary {
                op: BinaryOp::BitOr,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                line,
            };
        }
        Ok(lhs)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_bitwise_and()?;
        while self.match_token(&TokenKind::Caret) {
            let line = self.current_line();
            let rhs = self.parse_bitwise_and()?;
            lhs = Expr::Binary {
                op: BinaryOp::BitXor,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                line,
            };
        }
        Ok(lhs)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_equality()?;
        while self.match_token(&TokenKind::Amp) {
            let line = self.current_line();
            let rhs = self.parse_equality()?;
            lhs = Expr::Binary {
                op: BinaryOp::BitAnd,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                line,
            };
        }
        Ok(lhs)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_relational()?;
        loop {
            let line = self.current_line();
            if self.match_token(&TokenKind::EqEq) {
                let rhs = self.parse_relational()?;
                lhs = Expr::Binary {
                    op: BinaryOp::Eq,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    line,
                };
            } else if self.match_token(&TokenKind::BangEq) {
                let rhs = self.parse_relational()?;
                lhs = Expr::Binary {
                    op: BinaryOp::Ne,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    line,
                };
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_relational(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_shift()?;
        loop {
            let line = self.current_line();
            if self.match_token(&TokenKind::Lt) {
                let rhs = self.parse_shift()?;
                lhs = Expr::Binary {
                    op: BinaryOp::Lt,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    line,
                };
            } else if self.match_token(&TokenKind::LtEq) {
                let rhs = self.parse_shift()?;
                lhs = Expr::Binary {
                    op: BinaryOp::Le,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    line,
                };
            } else if self.match_token(&TokenKind::Gt) {
                let rhs = self.parse_shift()?;
                lhs = Expr::Binary {
                    op: BinaryOp::Gt,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    line,
                };
            } else if self.match_token(&TokenKind::GtEq) {
                let rhs = self.parse_shift()?;
                lhs = Expr::Binary {
                    op: BinaryOp::Ge,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    line,
                };
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_additive()?;
        loop {
            let line = self.current_line();
            if self.match_token(&TokenKind::Shl) {
                let rhs = self.parse_additive()?;
                lhs = Expr::Binary {
                    op: BinaryOp::Shl,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    line,
                };
            } else if self.match_token(&TokenKind::Shr) {
                let rhs = self.parse_additive()?;
                lhs = Expr::Binary {
                    op: BinaryOp::Shr,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    line,
                };
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let line = self.current_line();
            if self.match_token(&TokenKind::Plus) {
                let rhs = self.parse_multiplicative()?;
                lhs = Expr::Binary {
                    op: BinaryOp::Add,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    line,
                };
            } else if self.match_token(&TokenKind::Minus) {
                let rhs = self.parse_multiplicative()?;
                lhs = Expr::Binary {
                    op: BinaryOp::Sub,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    line,
                };
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_unary()?;
        loop {
            let line = self.current_line();
            if self.match_token(&TokenKind::Star) {
                let rhs = self.parse_unary()?;
                lhs = Expr::Binary {
                    op: BinaryOp::Mul,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    line,
                };
            } else if self.match_token(&TokenKind::Slash) {
                let rhs = self.parse_unary()?;
                lhs = Expr::Binary {
                    op: BinaryOp::Div,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    line,
                };
            } else if self.match_token(&TokenKind::Percent) {
                let rhs = self.parse_unary()?;
                lhs = Expr::Binary {
                    op: BinaryOp::Rem,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    line,
                };
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        let line = self.current_line();

        if self.match_token(&TokenKind::PlusPlus) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::PreInc,
                expr: Box::new(expr),
                line,
            });
        }
        if self.match_token(&TokenKind::MinusMinus) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::PreDec,
                expr: Box::new(expr),
                line,
            });
        }
        if self.match_token(&TokenKind::Star) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Deref,
                expr: Box::new(expr),
                line,
            });
        }
        if self.match_token(&TokenKind::Amp) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::AddrOf,
                expr: Box::new(expr),
                line,
            });
        }
        if self.match_token(&TokenKind::Plus) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Plus,
                expr: Box::new(expr),
                line,
            });
        }
        if self.match_token(&TokenKind::Minus) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Minus,
                expr: Box::new(expr),
                line,
            });
        }
        if self.match_token(&TokenKind::Bang) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
                line,
            });
        }
        if self.match_token(&TokenKind::Tilde) {
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::BitNot,
                expr: Box::new(expr),
                line,
            });
        }
        if self.match_token(&TokenKind::Sizeof) {
            if self.match_token(&TokenKind::LParen) {
                if self.is_type_start() {
                    let mut ty = self.parse_type()?;
                    while self.match_token(&TokenKind::Star) {
                        ty = Type::Pointer(Box::new(ty));
                    }
                    self.expect(&TokenKind::RParen)?;
                    return Ok(Expr::SizeofType(ty, line));
                } else {
                    let expr = self.parse_expr()?;
                    self.expect(&TokenKind::RParen)?;
                    return Ok(Expr::Unary {
                        op: UnaryOp::Sizeof,
                        expr: Box::new(expr),
                        line,
                    });
                }
            } else {
                let expr = self.parse_unary()?;
                return Ok(Expr::Unary {
                    op: UnaryOp::Sizeof,
                    expr: Box::new(expr),
                    line,
                });
            }
        }

        // Cast: ( type ) expr
        if self.peek() == &TokenKind::LParen {
            let saved = self.pos;
            self.advance();
            if self.is_type_start() {
                if let Ok(mut ty) = self.parse_type() {
                    while self.match_token(&TokenKind::Star) {
                        ty = Type::Pointer(Box::new(ty));
                    }
                    if self.match_token(&TokenKind::RParen) {
                        // Cast confirmed!
                        let expr = self.parse_unary()?;
                        return Ok(Expr::Cast {
                            target_type: ty,
                            expr: Box::new(expr),
                            line,
                        });
                    }
                }
            }
            // Backtrack if not a cast
            self.pos = saved;
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            let line = self.current_line();
            if self.match_token(&TokenKind::LBracket) {
                let index = self.parse_expr()?;
                self.expect(&TokenKind::RBracket)?;
                expr = Expr::Index {
                    base: Box::new(expr),
                    index: Box::new(index),
                    line,
                };
            } else if self.match_token(&TokenKind::LParen) {
                let mut args = Vec::new();
                if self.peek() != &TokenKind::RParen {
                    loop {
                        args.push(self.parse_assignment()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::RParen)?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    line,
                };
            } else if self.match_token(&TokenKind::Dot) {
                if let TokenKind::Ident(field) = self.advance().kind {
                    expr = Expr::Member {
                        base: Box::new(expr),
                        field,
                        is_arrow: false,
                        line,
                    };
                } else {
                    return Err("expected field name after .".into());
                }
            } else if self.match_token(&TokenKind::Arrow) {
                if let TokenKind::Ident(field) = self.advance().kind {
                    expr = Expr::Member {
                        base: Box::new(expr),
                        field,
                        is_arrow: true,
                        line,
                    };
                } else {
                    return Err("expected field name after ->".into());
                }
            } else if self.match_token(&TokenKind::PlusPlus) {
                expr = Expr::Unary {
                    op: UnaryOp::PostInc,
                    expr: Box::new(expr),
                    line,
                };
            } else if self.match_token(&TokenKind::MinusMinus) {
                expr = Expr::Unary {
                    op: UnaryOp::PostDec,
                    expr: Box::new(expr),
                    line,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let line = self.current_line();
        match self.peek().clone() {
            TokenKind::IntLit(n) => {
                self.advance();
                Ok(Expr::IntLit(n, line))
            }
            TokenKind::FloatLit(f) => {
                self.advance();
                Ok(Expr::FloatLit(f, line))
            }
            TokenKind::CharLit(c) => {
                self.advance();
                Ok(Expr::CharLit(c, line))
            }
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(Expr::StringLit(s, line))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expr::NullLit(line))
            }
            TokenKind::Ident(id) => {
                self.advance();
                Ok(Expr::Ident(id, line))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            other => Err(format!(
                "unexpected token in primary expression: {:?} at line {}",
                other, line
            )),
        }
    }
}

enum TopLevelItem {
    Func(Function),
    Global(Vec<VarDecl>),
    None,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c::lexer::Lexer;

    #[test]
    fn test_parse_simple_func() {
        let code = r#"
            int add(int a, int b) {
                return a + b;
            }
        "#;
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let tu = parser.parse_translation_unit();
        assert_eq!(tu.functions.len(), 1);
        assert_eq!(tu.functions[0].name, "add");
        assert_eq!(tu.functions[0].params.len(), 2);
    }
}
