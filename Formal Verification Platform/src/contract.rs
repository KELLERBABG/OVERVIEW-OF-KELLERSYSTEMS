use rustc_middle::mir::{Body, Local, Operand, Place, VarDebugInfoContents};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;
use std::collections::HashMap;
use z3::ast::{Ast, Bool, Int};
use z3::Context;

use crate::constant_prop::ConstMap;
use crate::path_state::PathState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractExpr {
    Int(i128),
    Bool(bool),
    Ident(String),
    Unary(UnOp, Box<ContractExpr>),
    Binary(BinOp, Box<ContractExpr>, Box<ContractExpr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Implies,
}

#[derive(Debug, Clone, Default)]
pub struct FnContracts {
    pub requires: Vec<(String, ContractExpr)>,
    pub ensures: Vec<(String, ContractExpr)>,
}

// -----------------------------------------------------------------------------
// Lexer & Parser
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Int(i128),
    Bool(bool),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    AndAnd,
    OrOr,
    Implies, // ==> or ->
    Not,
    LParen,
    RParen,
    Comma,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        if c == '(' {
            tokens.push(Token::LParen);
            i += 1;
        } else if c == ')' {
            tokens.push(Token::RParen);
            i += 1;
        } else if c == ',' {
            tokens.push(Token::Comma);
            i += 1;
        } else if c == '+' {
            tokens.push(Token::Plus);
            i += 1;
        } else if c == '-' {
            if i + 1 < chars.len() && chars[i + 1] == '>' {
                tokens.push(Token::Implies);
                i += 2;
            } else {
                tokens.push(Token::Minus);
                i += 1;
            }
        } else if c == '*' {
            tokens.push(Token::Star);
            i += 1;
        } else if c == '/' {
            tokens.push(Token::Slash);
            i += 1;
        } else if c == '%' {
            tokens.push(Token::Percent);
            i += 1;
        } else if c == '=' {
            if i + 2 < chars.len() && chars[i + 1] == '=' && chars[i + 2] == '>' {
                tokens.push(Token::Implies);
                i += 3;
            } else if i + 1 < chars.len() && chars[i + 1] == '=' {
                tokens.push(Token::EqEq);
                i += 2;
            } else if i + 1 < chars.len() && chars[i + 1] == '>' {
                tokens.push(Token::Implies);
                i += 2;
            } else {
                return Err(format!("Unexpected character '=' at pos {i}"));
            }
        } else if c == '!' {
            if i + 1 < chars.len() && chars[i + 1] == '=' {
                tokens.push(Token::NotEq);
                i += 2;
            } else {
                tokens.push(Token::Not);
                i += 1;
            }
        } else if c == '<' {
            if i + 1 < chars.len() && chars[i + 1] == '=' {
                tokens.push(Token::LtEq);
                i += 2;
            } else {
                tokens.push(Token::Lt);
                i += 1;
            }
        } else if c == '>' {
            if i + 1 < chars.len() && chars[i + 1] == '=' {
                tokens.push(Token::GtEq);
                i += 2;
            } else {
                tokens.push(Token::Gt);
                i += 1;
            }
        } else if c == '&' {
            if i + 1 < chars.len() && chars[i + 1] == '&' {
                tokens.push(Token::AndAnd);
                i += 2;
            } else {
                return Err(format!("Expected '&&' at pos {i}"));
            }
        } else if c == '|' {
            if i + 1 < chars.len() && chars[i + 1] == '|' {
                tokens.push(Token::OrOr);
                i += 2;
            } else {
                return Err(format!("Expected '||' at pos {i}"));
            }
        } else if c.is_digit(10) {
            let start = i;
            let is_hex =
                c == '0' && i + 1 < chars.len() && (chars[i + 1] == 'x' || chars[i + 1] == 'X');
            let is_bin =
                c == '0' && i + 1 < chars.len() && (chars[i + 1] == 'b' || chars[i + 1] == 'B');
            if is_hex || is_bin {
                i += 2;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().filter(|&&ch| ch != '_').collect();
                let val = if is_hex {
                    i128::from_str_radix(&s[2..], 16).map_err(|e| e.to_string())?
                } else {
                    i128::from_str_radix(&s[2..], 2).map_err(|e| e.to_string())?
                };
                tokens.push(Token::Int(val));
            } else {
                while i < chars.len() && (chars[i].is_digit(10) || chars[i] == '_') {
                    i += 1;
                }
                let s: String = chars[start..i].iter().filter(|&&ch| ch != '_').collect();
                let val = s.parse::<i128>().map_err(|e| e.to_string())?;
                tokens.push(Token::Int(val));
            }
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            if s == "true" {
                tokens.push(Token::Bool(true));
            } else if s == "false" {
                tokens.push(Token::Bool(false));
            } else {
                tokens.push(Token::Ident(s));
            }
        } else {
            return Err(format!("Unexpected character '{c}' at pos {i}"));
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        match self.next() {
            Some(t) if t == expected => Ok(()),
            Some(t) => Err(format!("Expected {expected:?}, found {t:?}")),
            None => Err(format!("Expected {expected:?}, found EOF")),
        }
    }

    // Top-level: parses one or more comma-separated expressions
    fn parse_list(&mut self) -> Result<Vec<ContractExpr>, String> {
        let mut list = Vec::new();
        while self.peek().is_some() {
            list.push(self.parse_implies()?);
            if self.peek() == Some(&Token::Comma) {
                self.next();
            } else {
                break;
            }
        }
        Ok(list)
    }

    fn parse_implies(&mut self) -> Result<ContractExpr, String> {
        let left = self.parse_or()?;
        if self.peek() == Some(&Token::Implies) {
            self.next();
            let right = self.parse_implies()?;
            Ok(ContractExpr::Binary(
                BinOp::Implies,
                Box::new(left),
                Box::new(right),
            ))
        } else {
            Ok(left)
        }
    }

    fn parse_or(&mut self) -> Result<ContractExpr, String> {
        let mut left = self.parse_and()?;
        while self.peek() == Some(&Token::OrOr) {
            self.next();
            let right = self.parse_and()?;
            left = ContractExpr::Binary(BinOp::Or, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<ContractExpr, String> {
        let mut left = self.parse_comparison()?;
        while self.peek() == Some(&Token::AndAnd) {
            self.next();
            let right = self.parse_comparison()?;
            left = ContractExpr::Binary(BinOp::And, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<ContractExpr, String> {
        let left = self.parse_arith()?;
        let op = match self.peek() {
            Some(Token::EqEq) => BinOp::Eq,
            Some(Token::NotEq) => BinOp::Ne,
            Some(Token::Lt) => BinOp::Lt,
            Some(Token::LtEq) => BinOp::Le,
            Some(Token::Gt) => BinOp::Gt,
            Some(Token::GtEq) => BinOp::Ge,
            _ => return Ok(left),
        };
        self.next();
        let right = self.parse_arith()?;
        Ok(ContractExpr::Binary(op, Box::new(left), Box::new(right)))
    }

    fn parse_arith(&mut self) -> Result<ContractExpr, String> {
        let mut left = self.parse_term()?;
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.next();
            let right = self.parse_term()?;
            left = ContractExpr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<ContractExpr, String> {
        let mut left = self.parse_unary()?;
        while let Some(tok) = self.peek() {
            let op = match tok {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Rem,
                _ => break,
            };
            self.next();
            let right = self.parse_unary()?;
            left = ContractExpr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<ContractExpr, String> {
        match self.peek() {
            Some(Token::Not) => {
                self.next();
                let inner = self.parse_unary()?;
                Ok(ContractExpr::Unary(UnOp::Not, Box::new(inner)))
            }
            Some(Token::Minus) => {
                self.next();
                let inner = self.parse_unary()?;
                Ok(ContractExpr::Unary(UnOp::Neg, Box::new(inner)))
            }
            _ => self.parse_atom(),
        }
    }

    fn parse_atom(&mut self) -> Result<ContractExpr, String> {
        match self.next() {
            Some(Token::Int(v)) => Ok(ContractExpr::Int(v)),
            Some(Token::Bool(b)) => Ok(ContractExpr::Bool(b)),
            Some(Token::Ident(name)) => Ok(ContractExpr::Ident(name)),
            Some(Token::LParen) => {
                let expr = self.parse_implies()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Some(tok) => Err(format!("Unexpected token in expression: {tok:?}")),
            None => Err("Unexpected end of expression".to_string()),
        }
    }
}

pub fn parse_contract_exprs(input: &str) -> Result<Vec<ContractExpr>, String> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse_list()
}

// -----------------------------------------------------------------------------
// AST Evaluation into Z3
// -----------------------------------------------------------------------------

pub enum Z3Val<'ctx> {
    Int(Int<'ctx>),
    Bool(Bool<'ctx>),
}

pub trait VarResolver<'ctx> {
    fn resolve(&mut self, name: &str) -> Option<Z3Val<'ctx>>;
}

impl<'ctx> ContractExpr {
    pub fn eval<R: VarResolver<'ctx>>(
        &self,
        ctx: &'ctx Context,
        resolver: &mut R,
    ) -> Result<Z3Val<'ctx>, String> {
        match self {
            ContractExpr::Int(v) => Ok(Z3Val::Int(Int::from_str(ctx, &v.to_string()).unwrap())),
            ContractExpr::Bool(b) => Ok(Z3Val::Bool(Bool::from_bool(ctx, *b))),
            ContractExpr::Ident(name) => resolver
                .resolve(name)
                .ok_or_else(|| format!("Unknown identifier in contract: '{name}'")),
            ContractExpr::Unary(op, inner) => {
                let val = inner.eval(ctx, resolver)?;
                match op {
                    UnOp::Not => match val {
                        Z3Val::Bool(b) => Ok(Z3Val::Bool(b.not())),
                        Z3Val::Int(_) => Err("Cannot apply '!' to integer".to_string()),
                    },
                    UnOp::Neg => match val {
                        Z3Val::Int(i) => Ok(Z3Val::Int(-i)),
                        Z3Val::Bool(_) => Err("Cannot apply unary '-' to boolean".to_string()),
                    },
                }
            }
            ContractExpr::Binary(op, left, right) => {
                let l_val = left.eval(ctx, resolver)?;
                let r_val = right.eval(ctx, resolver)?;
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                        let (Z3Val::Int(l), Z3Val::Int(r)) = (l_val, r_val) else {
                            return Err(format!("Arithmetic {op:?} requires integer operands"));
                        };
                        let res = match op {
                            BinOp::Add => l + r,
                            BinOp::Sub => l - r,
                            BinOp::Mul => l * r,
                            BinOp::Div => l / r,
                            BinOp::Rem => l.rem(&r),
                            _ => unreachable!(),
                        };
                        Ok(Z3Val::Int(res))
                    }
                    BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        let (Z3Val::Int(l), Z3Val::Int(r)) = (l_val, r_val) else {
                            return Err(format!("Comparison {op:?} requires integer operands"));
                        };
                        let res = match op {
                            BinOp::Lt => l.lt(&r),
                            BinOp::Le => l.le(&r),
                            BinOp::Gt => l.gt(&r),
                            BinOp::Ge => l.ge(&r),
                            _ => unreachable!(),
                        };
                        Ok(Z3Val::Bool(res))
                    }
                    BinOp::Eq | BinOp::Ne => match (l_val, r_val) {
                        (Z3Val::Int(l), Z3Val::Int(r)) => {
                            let eq = l._eq(&r);
                            Ok(Z3Val::Bool(if *op == BinOp::Eq { eq } else { eq.not() }))
                        }
                        (Z3Val::Bool(l), Z3Val::Bool(r)) => {
                            let eq = l._eq(&r);
                            Ok(Z3Val::Bool(if *op == BinOp::Eq { eq } else { eq.not() }))
                        }
                        _ => Err("Equality requires operands of the same type".to_string()),
                    },
                    BinOp::And | BinOp::Or | BinOp::Implies => {
                        let (Z3Val::Bool(l), Z3Val::Bool(r)) = (l_val, r_val) else {
                            return Err(format!("Logical {op:?} requires boolean operands"));
                        };
                        let res = match op {
                            BinOp::And => Bool::and(ctx, &[&l, &r]),
                            BinOp::Or => Bool::or(ctx, &[&l, &r]),
                            BinOp::Implies => Bool::or(ctx, &[&l.not(), &r]),
                            _ => unreachable!(),
                        };
                        Ok(Z3Val::Bool(res))
                    }
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Resolvers for Callee and Caller
// -----------------------------------------------------------------------------

/// Resolver used when verifying a function's own body.
pub struct BodyVarResolver<'a, 'ctx, 'tcx> {
    pub ctx: &'ctx Context,
    pub mir: &'a Body<'tcx>,
    pub const_map: &'a ConstMap,
    pub tcx: TyCtxt<'tcx>,
    pub state: &'a mut PathState<'ctx>,
    pub name_to_local: &'a HashMap<String, Local>,
}

impl<'a, 'ctx, 'tcx> VarResolver<'ctx> for BodyVarResolver<'a, 'ctx, 'tcx> {
    fn resolve(&mut self, name: &str) -> Option<Z3Val<'ctx>> {
        let &local = self.name_to_local.get(name)?;
        let place = Place::from(local);
        let operand = Operand::Copy(place);
        let int_val =
            self.state
                .int_operand(self.ctx, self.mir, self.const_map, self.tcx, &operand)?;
        // Check if local is boolean type
        let local_ty = self.mir.local_decls[local].ty;
        if local_ty.is_bool() {
            let zero = Int::from_i64(self.ctx, 0);
            Some(Z3Val::Bool(int_val._eq(&zero).not()))
        } else {
            Some(Z3Val::Int(int_val))
        }
    }
}

/// Resolver used at a caller's call site to evaluate callee contracts.
pub struct CallSiteVarResolver<'a, 'ctx, 'tcx> {
    pub ctx: &'ctx Context,
    pub caller_mir: &'a Body<'tcx>,
    pub const_map: &'a ConstMap,
    pub tcx: TyCtxt<'tcx>,
    pub state: &'a mut PathState<'ctx>,
    /// Map from parameter name (e.g. "x") or return name ("result") to caller operand or place
    pub param_to_arg: &'a HashMap<String, Operand<'tcx>>,
    pub return_place: Option<Place<'tcx>>,
}

impl<'a, 'ctx, 'tcx> VarResolver<'ctx> for CallSiteVarResolver<'a, 'ctx, 'tcx> {
    fn resolve(&mut self, name: &str) -> Option<Z3Val<'ctx>> {
        if (name == "result" || name == "return" || name == "_0") && self.return_place.is_some() {
            let place = self.return_place.as_ref().unwrap();
            let operand = Operand::Copy(*place);
            let int_val = self.state.int_operand(
                self.ctx,
                self.caller_mir,
                self.const_map,
                self.tcx,
                &operand,
            )?;
            return Some(Z3Val::Int(int_val));
        }

        let operand = self.param_to_arg.get(name)?;
        let int_val =
            self.state
                .int_operand(self.ctx, self.caller_mir, self.const_map, self.tcx, operand)?;
        let ty = operand.ty(self.caller_mir, self.tcx);
        if ty.is_bool() {
            let zero = Int::from_i64(self.ctx, 0);
            Some(Z3Val::Bool(int_val._eq(&zero).not()))
        } else {
            Some(Z3Val::Int(int_val))
        }
    }
}

// -----------------------------------------------------------------------------
// Contract Extraction from MIR / Rustc Attributes
// -----------------------------------------------------------------------------

pub fn build_param_mapping(mir: &Body<'_>) -> HashMap<String, Local> {
    let mut map = HashMap::new();
    // Default names
    map.insert("result".to_string(), Local::from_usize(0));
    map.insert("return".to_string(), Local::from_usize(0));
    map.insert("_0".to_string(), Local::from_usize(0));

    for i in 1..=mir.arg_count {
        let local = Local::from_usize(i);
        map.insert(format!("_{i}"), local);
        map.insert(format!("arg{}", i - 1), local);
    }

    // Add source parameter names from var_debug_info
    for var in &mir.var_debug_info {
        if let VarDebugInfoContents::Place(place) = &var.value {
            if place.projection.is_empty() {
                let name = var.name.as_str().to_string();
                map.insert(name, place.local);
            }
        }
    }
    map
}

pub fn build_callsite_mapping<'tcx>(
    tcx: TyCtxt<'tcx>,
    callee_def_id: DefId,
    args: &[rustc_span::Spanned<Operand<'tcx>>],
) -> HashMap<String, Operand<'tcx>> {
    let mut map = HashMap::new();
    for (i, arg) in args.iter().enumerate() {
        map.insert(format!("arg{i}"), arg.node.clone());
        map.insert(format!("_{}", i + 1), arg.node.clone());
    }
    if tcx.is_mir_available(callee_def_id) {
        let callee_mir = tcx.optimized_mir(callee_def_id);
        let param_map = build_param_mapping(&callee_mir);
        for (name, local) in param_map {
            let idx = local.as_usize();
            if idx >= 1 && idx <= args.len() {
                map.insert(name, args[idx - 1].node.clone());
            }
        }
    }
    map
}

#[allow(deprecated)]
pub fn extract_fn_contracts(tcx: TyCtxt<'_>, def_id: DefId) -> FnContracts {
    let mut contracts = FnContracts::default();
    let attrs = tcx.get_all_attrs(def_id);

    for attr in attrs {
        // Check for doc comments
        if let Some(doc_sym) = attr.doc_str() {
            let doc_text = doc_sym.as_str();
            for line in doc_text.lines() {
                parse_contract_from_line(line, &mut contracts);
            }
            continue;
        }

        // In rustc_hir, `Attribute` can be `Attribute::Parsed(..)`. Calling `.span()` on it panics!
        // We only care about `requires` and `ensures` attributes.
        match attr {
            rustc_hir::Attribute::Unparsed(ref item) => {
                let path_str = item
                    .path
                    .segments
                    .iter()
                    .map(|seg| seg.as_str())
                    .collect::<Vec<_>>()
                    .join("::");

                if path_str == "requires"
                    || path_str == "ensures"
                    || path_str == "keller::requires"
                    || path_str == "keller::ensures"
                    || path_str.ends_with("::requires")
                    || path_str.ends_with("::ensures")
                {
                    let snippet = tcx
                        .sess
                        .source_map()
                        .span_to_snippet(attr.span())
                        .unwrap_or_default();
                    parse_contract_from_attr_str(&snippet, &mut contracts);
                }
            }
            rustc_hir::Attribute::Parsed(_) => {}
        }
    }

    contracts
}

fn parse_contract_from_line(line: &str, contracts: &mut FnContracts) {
    let trimmed = line.trim();
    let stripped = trimmed.trim_start_matches(|c: char| c == '/' || c == '*' || c.is_whitespace());

    // Look for #[requires(...)] or #[ensures(...)]
    if stripped.contains("#[requires(") || stripped.contains("#[keller::requires(") {
        if let Some(inner) = extract_parenthesized_content(stripped, "requires(") {
            if let Ok(exprs) = parse_contract_exprs(inner) {
                for expr in exprs {
                    contracts.requires.push((inner.to_string(), expr));
                }
            }
        }
    } else if stripped.contains("#[ensures(") || stripped.contains("#[keller::ensures(") {
        if let Some(inner) = extract_parenthesized_content(stripped, "ensures(") {
            if let Ok(exprs) = parse_contract_exprs(inner) {
                for expr in exprs {
                    contracts.ensures.push((inner.to_string(), expr));
                }
            }
        }
    } else {
        // Look for doc markdown: "requires: <expr>" or "ensures: <expr>"
        let lower = stripped.to_lowercase();
        if let Some(rest) = lower
            .strip_prefix("requires:")
            .or_else(|| lower.strip_prefix("# requires:"))
        {
            let raw_expr = &stripped[stripped.len() - rest.len()..];
            if let Ok(exprs) = parse_contract_exprs(raw_expr) {
                for expr in exprs {
                    contracts.requires.push((raw_expr.to_string(), expr));
                }
            }
        } else if let Some(rest) = lower
            .strip_prefix("ensures:")
            .or_else(|| lower.strip_prefix("# ensures:"))
        {
            let raw_expr = &stripped[stripped.len() - rest.len()..];
            if let Ok(exprs) = parse_contract_exprs(raw_expr) {
                for expr in exprs {
                    contracts.ensures.push((raw_expr.to_string(), expr));
                }
            }
        }
    }
}

fn parse_contract_from_attr_str(attr_str: &str, contracts: &mut FnContracts) {
    if attr_str.contains("requires(") {
        if let Some(inner) = extract_parenthesized_content(attr_str, "requires(") {
            if let Ok(exprs) = parse_contract_exprs(inner) {
                for expr in exprs {
                    contracts.requires.push((inner.to_string(), expr));
                }
            }
        }
    }
    if attr_str.contains("ensures(") {
        if let Some(inner) = extract_parenthesized_content(attr_str, "ensures(") {
            if let Ok(exprs) = parse_contract_exprs(inner) {
                for expr in exprs {
                    contracts.ensures.push((inner.to_string(), expr));
                }
            }
        }
    }
}

fn extract_parenthesized_content<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    let start_idx = text.find(marker)? + marker.len();
    let mut depth = 1;
    let bytes = text[start_idx..].as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'(' {
            depth += 1;
        } else if b == b')' {
            depth -= 1;
            if depth == 0 {
                return Some(&text[start_idx..start_idx + i]);
            }
        }
    }
    None
}

// -----------------------------------------------------------------------------
// Unit Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use z3::Config;

    #[test]
    fn test_parse_contract_expr() {
        let exprs = parse_contract_exprs("x < 100, y >= 0 && y <= 50").unwrap();
        assert_eq!(exprs.len(), 2);

        let exprs2 = parse_contract_exprs("result == x + 1").unwrap();
        assert_eq!(exprs2.len(), 1);

        let exprs3 = parse_contract_exprs("x > 0 ==> result > 10").unwrap();
        assert_eq!(exprs3.len(), 1);
    }

    struct MockResolver<'ctx> {
        vals: HashMap<String, Z3Val<'ctx>>,
    }

    impl<'ctx> VarResolver<'ctx> for MockResolver<'ctx> {
        fn resolve(&mut self, name: &str) -> Option<Z3Val<'ctx>> {
            self.vals.remove(name)
        }
    }

    #[test]
    fn test_eval_contract_expr() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let exprs = parse_contract_exprs("x + 10 < 100").unwrap();
        assert_eq!(exprs.len(), 1);

        let mut resolver = MockResolver {
            vals: {
                let mut m = HashMap::new();
                m.insert("x".to_string(), Z3Val::Int(Int::from_i64(&ctx, 50)));
                m
            },
        };

        let res = exprs[0].eval(&ctx, &mut resolver).unwrap();
        match res {
            Z3Val::Bool(b) => {
                let solver = z3::Solver::new(&ctx);
                solver.assert(&b);
                assert_eq!(solver.check(), z3::SatResult::Sat);
            }
            _ => panic!("Expected boolean result"),
        }
    }

    #[test]
    fn test_extract_contract_from_doc() {
        let mut contracts = FnContracts::default();
        parse_contract_from_line("/// #[requires(x < 100)]", &mut contracts);
        parse_contract_from_line("/// #[ensures(result > 0)]", &mut contracts);
        parse_contract_from_line("/// Requires: a + b <= 255", &mut contracts);

        assert_eq!(contracts.requires.len(), 2);
        assert_eq!(contracts.ensures.len(), 1);
    }

    #[test]
    fn test_solver_precondition_check() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let exprs = parse_contract_exprs("x < 100").unwrap();

        // Safe case: caller passes 50 -> !(50 < 100) is UNSAT (proved safe)
        {
            let mut resolver = MockResolver {
                vals: [("x".to_string(), Z3Val::Int(Int::from_i64(&ctx, 50)))]
                    .into_iter()
                    .collect(),
            };
            let evaluated = exprs[0].eval(&ctx, &mut resolver);
            if let Ok(Z3Val::Bool(cond)) = evaluated {
                let solver = z3::Solver::new(&ctx);
                solver.assert(&cond.not());
                assert_eq!(solver.check(), z3::SatResult::Unsat);
            } else {
                panic!("Expected bool");
            }
        }

        // Failing case: caller passes 150 -> !(150 < 100) is SAT (counterexample found)
        {
            let mut resolver = MockResolver {
                vals: [("x".to_string(), Z3Val::Int(Int::from_i64(&ctx, 150)))]
                    .into_iter()
                    .collect(),
            };
            let evaluated = exprs[0].eval(&ctx, &mut resolver);
            if let Ok(Z3Val::Bool(cond)) = evaluated {
                let solver = z3::Solver::new(&ctx);
                solver.assert(&cond.not());
                assert_eq!(solver.check(), z3::SatResult::Sat);
            } else {
                panic!("Expected bool");
            }
        }
    }

    #[test]
    fn test_solver_postcondition_propagation() {
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let exprs = parse_contract_exprs("result > 0").unwrap();

        // If callee ensures result > 0, then caller dividing by result cannot div-by-zero:
        // Solver checks: (result > 0) && (result == 0) -> UNSAT
        let res_var = Int::new_const(&ctx, "res");
        let mut resolver = MockResolver {
            vals: [("result".to_string(), Z3Val::Int(res_var.clone()))]
                .into_iter()
                .collect(),
        };
        let evaluated = exprs[0].eval(&ctx, &mut resolver);
        if let Ok(Z3Val::Bool(postcond)) = evaluated {
            let solver = z3::Solver::new(&ctx);
            solver.assert(&postcond);
            solver.assert(&res_var._eq(&Int::from_i64(&ctx, 0)));
            assert_eq!(solver.check(), z3::SatResult::Unsat);
        } else {
            panic!("Expected bool");
        }
    }
}
