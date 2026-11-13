//! C# AST definitions for Keller Verify C# frontend.

use std::fmt;

/// Source location (1-based line, 1-based column).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourceLoc {
    pub line: usize,
    pub col: usize,
}

impl SourceLoc {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

impl fmt::Display for SourceLoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// C# type representation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CSharpType {
    pub name: String,
    pub is_primitive: bool,
    pub is_nullable: bool,
    pub is_span: bool,
    pub array_dims: usize,
    pub generic_args: Vec<CSharpType>,
}

impl CSharpType {
    pub fn new(name: &str) -> Self {
        let clean = name.trim();
        let is_prim = matches!(
            clean,
            "int" | "long" | "short" | "byte" | "sbyte" | "uint" | "ulong" | "ushort"
                | "float" | "double" | "decimal" | "bool" | "char" | "void"
                | "nint" | "nuint"
        );
        let is_span = clean == "Span" || clean == "ReadOnlySpan";
        Self {
            name: clean.to_string(),
            is_primitive: is_prim,
            is_nullable: false,
            is_span,
            array_dims: 0,
            generic_args: Vec::new(),
        }
    }

    pub fn nullable(mut self) -> Self {
        self.is_nullable = true;
        self
    }

    pub fn array(mut self, dims: usize) -> Self {
        self.array_dims += dims;
        self
    }

    pub fn span(elem: CSharpType, _readonly: bool) -> Self {
        Self {
            name: "Span".to_string(),
            is_primitive: false,
            is_nullable: false,
            is_span: true,
            array_dims: 0,
            generic_args: vec![elem],
        }
    }

    pub fn is_reference(&self) -> bool {
        self.array_dims > 0
            || self.name == "string"
            || self.name == "object"
            || self.name == "dynamic"
            || (!self.is_primitive && !self.is_span)
    }

    pub fn is_numeric_primitive(&self) -> bool {
        matches!(
            self.name.as_str(),
            "byte" | "sbyte" | "short" | "ushort" | "int" | "uint" | "long" | "ulong"
                | "float" | "double" | "decimal" | "nint" | "nuint" | "char"
        )
    }
}

impl fmt::Display for CSharpType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.generic_args.is_empty() {
            write!(f, "<")?;
            for (i, arg) in self.generic_args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", arg)?;
            }
            write!(f, ">")?;
        }
        for _ in 0..self.array_dims {
            write!(f, "[]")?;
        }
        if self.is_nullable {
            write!(f, "?")?;
        }
        Ok(())
    }
}

/// A compilation unit (file).
#[derive(Debug, Clone)]
pub struct CompilationUnit {
    pub namespace: Option<String>,
    pub usings: Vec<String>,
    pub types: Vec<TypeDecl>,
}

pub type Program = CompilationUnit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind {
    Class,
    Struct,
    Interface,
    Record,
    Enum,
}

#[derive(Debug, Clone)]
pub struct TypeDecl {
    pub name: String,
    pub kind: TypeKind,
    pub modifiers: Vec<String>,
    pub type_params: Vec<String>,
    pub base_types: Vec<CSharpType>,
    pub methods: Vec<MethodDecl>,
    pub constructors: Vec<ConstructorDecl>,
    pub properties: Vec<PropertyDecl>,
    pub fields: Vec<FieldDecl>,
    pub nested_types: Vec<TypeDecl>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub struct MethodDecl {
    pub name: String,
    pub modifiers: Vec<String>,
    pub return_ty: Option<CSharpType>,
    pub params: Vec<Param>,
    pub body: Option<Block>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub struct PropertyDecl {
    pub name: String,
    pub modifiers: Vec<String>,
    pub ty: CSharpType,
    pub getter: Option<Block>,
    pub setter: Option<Block>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub struct ConstructorDecl {
    pub name: String,
    pub modifiers: Vec<String>,
    pub params: Vec<Param>,
    pub body: Option<Block>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: String,
    pub modifiers: Vec<String>,
    pub ty: CSharpType,
    pub init: Option<Expr>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: CSharpType,
    pub modifiers: Vec<String>,
    pub default_val: Option<Expr>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Block(Block),
    VarDecl {
        ty: CSharpType,
        vars: Vec<(String, Option<Expr>)>,
        loc: SourceLoc,
    },
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
        init: Option<Vec<Stmt>>,
        cond: Option<Expr>,
        update: Option<Vec<Expr>>,
        body: Box<Stmt>,
        loc: SourceLoc,
    },
    ForEach {
        var_name: String,
        var_ty: CSharpType,
        iterable: Expr,
        body: Box<Stmt>,
        loc: SourceLoc,
    },
    Switch {
        selector: Expr,
        cases: Vec<SwitchCase>,
        loc: SourceLoc,
    },
    Return(Option<Expr>, SourceLoc),
    Throw(Expr, SourceLoc),
    Try {
        body: Block,
        catches: Vec<CatchClause>,
        finally_body: Option<Block>,
        loc: SourceLoc,
    },
    Using {
        init: Option<Box<Stmt>>,
        expr: Option<Expr>,
        body: Box<Stmt>,
        loc: SourceLoc,
    },
    Lock {
        lock_expr: Expr,
        body: Block,
        loc: SourceLoc,
    },
    Break(SourceLoc),
    Continue(SourceLoc),
    Empty(SourceLoc),
    LocalFunction(MethodDecl),
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub labels: Vec<Expr>,
    pub stmts: Vec<Stmt>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub struct CatchClause {
    pub param_name: Option<String>,
    pub exception_ty: Option<CSharpType>,
    pub body: Block,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Null(SourceLoc),
    Bool(bool, SourceLoc),
    Int(i64, SourceLoc),
    Float(f64, SourceLoc),
    StringLit(String, SourceLoc),
    CharLit(char, SourceLoc),
    Ident(String, SourceLoc),
    This(SourceLoc),
    Base(SourceLoc),
    Default(Option<CSharpType>, SourceLoc),
    MemberAccess {
        target: Box<Expr>,
        member: String,
        is_null_conditional: bool,
        loc: SourceLoc,
    },
    MethodCall {
        target: Option<Box<Expr>>,
        method: String,
        args: Vec<Expr>,
        is_null_conditional: bool,
        loc: SourceLoc,
    },
    ElementAccess {
        target: Box<Expr>,
        index: Box<Expr>,
        is_null_conditional: bool,
        loc: SourceLoc,
    },
    NewObject {
        ty: CSharpType,
        args: Vec<Expr>,
        loc: SourceLoc,
    },
    NewArray {
        element_ty: CSharpType,
        dims: Vec<Expr>,
        initializer: Option<Vec<Expr>>,
        loc: SourceLoc,
    },
    Cast {
        target_ty: CSharpType,
        expr: Box<Expr>,
        loc: SourceLoc,
    },
    IsPattern {
        expr: Box<Expr>,
        pattern_ty: CSharpType,
        pattern_var: Option<String>,
        is_negated: bool,
        loc: SourceLoc,
    },
    AsCast {
        expr: Box<Expr>,
        target_ty: CSharpType,
        loc: SourceLoc,
    },
    NullCoalescing {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        loc: SourceLoc,
    },
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
    Ternary {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        loc: SourceLoc,
    },
    Assign {
        op: AssignOp,
        target: Box<Expr>,
        value: Box<Expr>,
        loc: SourceLoc,
    },
    Lambda {
        params: Vec<String>,
        body_expr: Option<Box<Expr>>,
        body_block: Option<Block>,
        loc: SourceLoc,
    },
    Tuple(Vec<Expr>, SourceLoc),
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        loc: SourceLoc,
    },
    ThrowExpr(Box<Expr>, SourceLoc),
}

impl Expr {
    pub fn loc(&self) -> SourceLoc {
        match self {
            Expr::Null(l)
            | Expr::Bool(_, l)
            | Expr::Int(_, l)
            | Expr::Float(_, l)
            | Expr::StringLit(_, l)
            | Expr::CharLit(_, l)
            | Expr::Ident(_, l)
            | Expr::This(l)
            | Expr::Base(l)
            | Expr::Default(_, l)
            | Expr::MemberAccess { loc: l, .. }
            | Expr::MethodCall { loc: l, .. }
            | Expr::ElementAccess { loc: l, .. }
            | Expr::NewObject { loc: l, .. }
            | Expr::NewArray { loc: l, .. }
            | Expr::Cast { loc: l, .. }
            | Expr::IsPattern { loc: l, .. }
            | Expr::AsCast { loc: l, .. }
            | Expr::NullCoalescing { loc: l, .. }
            | Expr::Unary { loc: l, .. }
            | Expr::Binary { loc: l, .. }
            | Expr::Ternary { loc: l, .. }
            | Expr::Assign { loc: l, .. }
            | Expr::Lambda { loc: l, .. }
            | Expr::Tuple(_, l)
            | Expr::Range { loc: l, .. }
            | Expr::ThrowExpr(_, l) => *l,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
    BitNot,
    PreInc,
    PreDec,
    PostInc,
    PostDec,
    NullForgiving,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
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
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    ShlAssign,
    ShrAssign,
    NullCoalescingAssign,
}
