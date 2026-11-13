#![allow(dead_code)]

//! Abstract Syntax Tree (AST) definitions for TypeScript and modern JavaScript.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceLoc {
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for SourceLoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: String,
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl {
        kind: VarKind,
        decls: Vec<VarDeclarator>,
        loc: SourceLoc,
    },
    FunctionDecl(Function),
    ClassDecl(Class),
    InterfaceDecl(Interface),
    TypeAliasDecl(TypeAlias),
    EnumDecl(Enum),
    Expr(Expr, SourceLoc),
    If {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
        loc: SourceLoc,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
        loc: SourceLoc,
    },
    DoWhile {
        body: Box<Stmt>,
        cond: Expr,
        loc: SourceLoc,
    },
    For {
        init: Option<ForInit>,
        cond: Option<Expr>,
        update: Option<Expr>,
        body: Box<Stmt>,
        loc: SourceLoc,
    },
    ForOf {
        var: VarBinding,
        iterable: Expr,
        body: Box<Stmt>,
        loc: SourceLoc,
    },
    ForIn {
        var: VarBinding,
        object: Expr,
        body: Box<Stmt>,
        loc: SourceLoc,
    },
    Switch {
        discriminant: Expr,
        cases: Vec<SwitchCase>,
        loc: SourceLoc,
    },
    Return(Option<Expr>, SourceLoc),
    Throw(Expr, SourceLoc),
    Try {
        body: Block,
        catch_clause: Option<CatchClause>,
        finally_block: Option<Block>,
        loc: SourceLoc,
    },
    Break(Option<String>, SourceLoc),
    Continue(Option<String>, SourceLoc),
    Block(Block),
    Import(ImportDecl),
    Export(ExportDecl),
    Empty(SourceLoc),
}

impl Stmt {
    pub fn loc(&self) -> SourceLoc {
        match self {
            Stmt::VarDecl { loc, .. } => *loc,
            Stmt::FunctionDecl(f) => f.loc,
            Stmt::ClassDecl(c) => c.loc,
            Stmt::InterfaceDecl(i) => i.loc,
            Stmt::TypeAliasDecl(t) => t.loc,
            Stmt::EnumDecl(e) => e.loc,
            Stmt::Expr(_, loc) => *loc,
            Stmt::If { loc, .. } => *loc,
            Stmt::While { loc, .. } => *loc,
            Stmt::DoWhile { loc, .. } => *loc,
            Stmt::For { loc, .. } => *loc,
            Stmt::ForOf { loc, .. } => *loc,
            Stmt::ForIn { loc, .. } => *loc,
            Stmt::Switch { loc, .. } => *loc,
            Stmt::Return(_, loc) => *loc,
            Stmt::Throw(_, loc) => *loc,
            Stmt::Try { loc, .. } => *loc,
            Stmt::Break(_, loc) => *loc,
            Stmt::Continue(_, loc) => *loc,
            Stmt::Block(b) => b.loc,
            Stmt::Import(i) => i.loc,
            Stmt::Export(e) => e.loc,
            Stmt::Empty(loc) => *loc,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarKind {
    Let,
    Const,
    Var,
}

#[derive(Debug, Clone)]
pub struct VarDeclarator {
    pub pattern: BindingPattern,
    pub ty: Option<TypeAnnotation>,
    pub init: Option<Expr>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub enum BindingPattern {
    Ident(String),
    Array(Vec<Option<BindingPattern>>),
    Object(Vec<ObjectBindingProp>),
}

impl BindingPattern {
    pub fn as_ident(&self) -> Option<&str> {
        match self {
            BindingPattern::Ident(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn collect_idents(&self, out: &mut Vec<String>) {
        match self {
            BindingPattern::Ident(s) => out.push(s.clone()),
            BindingPattern::Array(elems) => {
                for elem in elems.iter().flatten() {
                    elem.collect_idents(out);
                }
            }
            BindingPattern::Object(props) => {
                for p in props {
                    if let Some(ref pat) = p.value {
                        pat.collect_idents(out);
                    } else {
                        out.push(p.key.clone());
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectBindingProp {
    pub key: String,
    pub value: Option<BindingPattern>,
    pub default_val: Option<Expr>,
}

#[derive(Debug, Clone)]
pub enum ForInit {
    VarDecl {
        kind: VarKind,
        decls: Vec<VarDeclarator>,
    },
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct VarBinding {
    pub kind: Option<VarKind>,
    pub pattern: BindingPattern,
    pub ty: Option<TypeAnnotation>,
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub test: Option<Expr>, // None = default
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct CatchClause {
    pub param: Option<BindingPattern>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub return_ty: Option<TypeAnnotation>,
    pub body: Option<Block>,
    pub is_async: bool,
    pub is_generator: bool,
    pub type_params: Vec<String>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub pattern: BindingPattern,
    pub ty: Option<TypeAnnotation>,
    pub default_value: Option<Expr>,
    pub is_optional: bool,
    pub is_rest: bool,
}

#[derive(Debug, Clone)]
pub struct Class {
    pub name: Option<String>,
    pub extends: Option<Expr>,
    pub implements: Vec<TypeAnnotation>,
    pub members: Vec<ClassMember>,
    pub is_abstract: bool,
    pub type_params: Vec<String>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub enum ClassMember {
    Constructor {
        params: Vec<Param>,
        body: Option<Block>,
        loc: SourceLoc,
    },
    Method {
        name: String,
        is_static: bool,
        is_async: bool,
        is_generator: bool,
        kind: MethodKind,
        params: Vec<Param>,
        return_ty: Option<TypeAnnotation>,
        body: Option<Block>,
        loc: SourceLoc,
    },
    Field {
        name: String,
        is_static: bool,
        ty: Option<TypeAnnotation>,
        init: Option<Expr>,
        loc: SourceLoc,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodKind {
    Normal,
    Getter,
    Setter,
}

#[derive(Debug, Clone)]
pub struct Interface {
    pub name: String,
    pub extends: Vec<TypeAnnotation>,
    pub type_params: Vec<String>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: String,
    pub type_params: Vec<String>,
    pub ty: TypeAnnotation,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub struct Enum {
    pub name: String,
    pub members: Vec<EnumMember>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub struct EnumMember {
    pub name: String,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub specifiers: Vec<String>,
    pub source: String,
    pub is_type_only: bool,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub struct ExportDecl {
    pub decl: Option<Box<Stmt>>,
    pub specifiers: Vec<String>,
    pub source: Option<String>,
    pub is_default: bool,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub enum TypeAnnotation {
    Basic(String),
    Union(Vec<TypeAnnotation>),
    Intersection(Vec<TypeAnnotation>),
    Array(Box<TypeAnnotation>),
    Generic(String, Vec<TypeAnnotation>),
    Function {
        params: Vec<TypeAnnotation>,
        return_ty: Box<TypeAnnotation>,
    },
    Tuple(Vec<TypeAnnotation>),
    Literal(String),
    Object,
    Any,
    Unknown,
    Void,
    Other(String),
}

impl TypeAnnotation {
    pub fn is_nullable_or_optional(&self) -> bool {
        match self {
            TypeAnnotation::Basic(name) => name == "undefined" || name == "null" || name == "any",
            TypeAnnotation::Union(types) => types.iter().any(|t| t.is_nullable_or_optional()),
            TypeAnnotation::Any => true,
            _ => false,
        }
    }

    pub fn is_function_type(&self) -> bool {
        match self {
            TypeAnnotation::Function { .. } => true,
            TypeAnnotation::Basic(name) => name == "Function",
            TypeAnnotation::Union(types) => types.iter().any(|t| t.is_function_type()),
            _ => false,
        }
    }
}

// ─── Expressions ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    // Literals
    Null(SourceLoc),
    Undefined(SourceLoc),
    Bool(bool, SourceLoc),
    Number(f64, String, SourceLoc),
    StringLit(String, SourceLoc),
    TemplateLit {
        quasis: Vec<String>,
        expressions: Vec<Expr>,
        loc: SourceLoc,
    },
    RegexLit(String, String, SourceLoc),

    // Identifiers & this/super
    Ident(String, SourceLoc),
    This(SourceLoc),
    Super(SourceLoc),

    // Aggregates
    ArrayLit(Vec<ArrayElem>, SourceLoc),
    ObjectLit(Vec<ObjectProp>, SourceLoc),

    // Functions
    ArrowFunction {
        params: Vec<Param>,
        return_ty: Option<TypeAnnotation>,
        body: ArrowBody,
        is_async: bool,
        loc: SourceLoc,
    },
    FunctionExpr(Function),

    // Member access & indexing
    Member {
        object: Box<Expr>,
        property: String,
        optional: bool,
        loc: SourceLoc,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        optional: bool,
        loc: SourceLoc,
    },

    // Calls & new
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        optional: bool,
        loc: SourceLoc,
    },
    New {
        callee: Box<Expr>,
        args: Vec<Expr>,
        loc: SourceLoc,
    },

    // Operators
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        loc: SourceLoc,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
        loc: SourceLoc,
    },
    Logical {
        op: LogicalOp,
        left: Box<Expr>,
        right: Box<Expr>,
        loc: SourceLoc,
    },
    Assign {
        op: AssignOp,
        left: Box<Expr>,
        right: Box<Expr>,
        loc: SourceLoc,
    },
    Update {
        op: UpdateOp,
        expr: Box<Expr>,
        prefix: bool,
        loc: SourceLoc,
    },
    Conditional {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        loc: SourceLoc,
    },
    TypeAssertion {
        expr: Box<Expr>,
        is_non_null: bool,
        ty: Option<TypeAnnotation>,
        loc: SourceLoc,
    },
    Sequence(Vec<Expr>, SourceLoc),
}

impl Expr {
    pub fn loc(&self) -> SourceLoc {
        match self {
            Expr::Null(loc)
            | Expr::Undefined(loc)
            | Expr::Bool(_, loc)
            | Expr::Number(_, _, loc)
            | Expr::StringLit(_, loc)
            | Expr::RegexLit(_, _, loc)
            | Expr::Ident(_, loc)
            | Expr::This(loc)
            | Expr::Super(loc)
            | Expr::ArrayLit(_, loc)
            | Expr::ObjectLit(_, loc)
            | Expr::FunctionExpr(Function { loc, .. })
            | Expr::Member { loc, .. }
            | Expr::Index { loc, .. }
            | Expr::Call { loc, .. }
            | Expr::New { loc, .. }
            | Expr::Unary { loc, .. }
            | Expr::Binary { loc, .. }
            | Expr::Logical { loc, .. }
            | Expr::Assign { loc, .. }
            | Expr::Update { loc, .. }
            | Expr::Conditional { loc, .. }
            | Expr::TypeAssertion { loc, .. }
            | Expr::Sequence(_, loc) => *loc,
            Expr::TemplateLit { loc, .. } => *loc,
            Expr::ArrowFunction { loc, .. } => *loc,
        }
    }

    pub fn is_literal_zero(&self) -> bool {
        match self {
            Expr::Number(val, _, _) => *val == 0.0,
            _ => false,
        }
    }

    pub fn is_literal_null_or_undefined(&self) -> bool {
        matches!(self, Expr::Null(_) | Expr::Undefined(_))
    }

    pub fn as_ident(&self) -> Option<&str> {
        match self {
            Expr::Ident(name, _) => Some(name.as_str()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ArrowBody {
    Expr(Box<Expr>),
    Block(Block),
}

#[derive(Debug, Clone)]
pub enum ArrayElem {
    Expr(Expr),
    Spread(Expr),
    Hole,
}

#[derive(Debug, Clone)]
pub enum ObjectProp {
    Property {
        key: Expr,
        value: Expr,
        shorthand: bool,
        computed: bool,
    },
    Spread(Expr),
    Method(Function),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
    BitNot,
    Typeof,
    Void,
    Delete,
    Await,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Exp,
    EqEq,
    EqEqEq,
    NotEq,
    NotEqEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    ShrUnsigned,
    Instanceof,
    In,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
    NullishCoalesce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    ExpAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
    ShrUnsignedAssign,
    AndAssign,
    OrAssign,
    NullishAssign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOp {
    Inc,
    Dec,
}
