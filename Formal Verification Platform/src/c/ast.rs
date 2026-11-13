//! C Abstract Syntax Tree (AST) definitions.

#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseType {
    Void,
    Bool,
    Char,
    Short,
    Int,
    Long,
    LongLong,
    Float,
    Double,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Base(BaseType, bool), // (base, is_unsigned)
    Pointer(Box<Type>),
    Array(Box<Type>, Option<usize>),
    Struct(String),
    Union(String),
    Enum(String),
    Typedef(String),
    Function {
        ret: Box<Type>,
        params: Vec<Type>,
    },
    Unknown,
}

impl Type {
    pub fn is_pointer(&self) -> bool {
        matches!(self, Type::Pointer(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Type::Array(_, _))
    }

    pub fn is_signed(&self) -> bool {
        match self {
            Type::Base(BaseType::Int | BaseType::Short | BaseType::Long | BaseType::LongLong, false) => true,
            Type::Base(BaseType::Char, false) => true,
            _ => false,
        }
    }

    pub fn size_bytes(&self) -> usize {
        match self {
            Type::Base(BaseType::Char | BaseType::Bool, _) => 1,
            Type::Base(BaseType::Short, _) => 2,
            Type::Base(BaseType::Int | BaseType::Float, _) => 4,
            Type::Base(BaseType::Long | BaseType::Double, _) => 8,
            Type::Base(BaseType::LongLong, _) => 8,
            Type::Pointer(_) => 8,
            Type::Array(elem, Some(len)) => elem.size_bytes() * len,
            _ => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Deref,
    AddrOf,
    Plus,
    Minus,
    Not,
    BitNot,
    PreInc,
    PreDec,
    PostInc,
    PostDec,
    Sizeof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Shl,
    Shr,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    BitAnd,
    BitXor,
    BitOr,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    ShlAssign,
    ShrAssign,
    BitAndAssign,
    BitXorAssign,
    BitOrAssign,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Ident(String, usize),
    IntLit(i64, usize),
    FloatLit(f64, usize),
    CharLit(char, usize),
    StringLit(String, usize),
    BoolLit(bool, usize),
    NullLit(usize),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        line: usize,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        line: usize,
    },
    Assign {
        op: AssignOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        line: usize,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        line: usize,
    },
    Member {
        base: Box<Expr>,
        field: String,
        is_arrow: bool,
        line: usize,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        line: usize,
    },
    Cast {
        target_type: Type,
        expr: Box<Expr>,
        line: usize,
    },
    Ternary {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        line: usize,
    },
    Comma(Vec<Expr>, usize),
    InitList(Vec<Expr>, usize),
    SizeofType(Type, usize),
}

impl Expr {
    pub fn line(&self) -> usize {
        match self {
            Expr::Ident(_, l)
            | Expr::IntLit(_, l)
            | Expr::FloatLit(_, l)
            | Expr::CharLit(_, l)
            | Expr::StringLit(_, l)
            | Expr::BoolLit(_, l)
            | Expr::NullLit(l)
            | Expr::Unary { line: l, .. }
            | Expr::Binary { line: l, .. }
            | Expr::Assign { line: l, .. }
            | Expr::Index { line: l, .. }
            | Expr::Member { line: l, .. }
            | Expr::Call { line: l, .. }
            | Expr::Cast { line: l, .. }
            | Expr::Ternary { line: l, .. }
            | Expr::Comma(_, l)
            | Expr::InitList(_, l)
            | Expr::SizeofType(_, l) => *l,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub name: String,
    pub ty: Type,
    pub init: Option<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Decl(Vec<VarDecl>, usize),
    Expr(Expr, usize),
    Block(Vec<Stmt>, usize),
    If {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
        line: usize,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
        line: usize,
    },
    DoWhile {
        body: Box<Stmt>,
        cond: Expr,
        line: usize,
    },
    For {
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        step: Option<Expr>,
        body: Box<Stmt>,
        line: usize,
    },
    Switch {
        expr: Expr,
        body: Box<Stmt>,
        line: usize,
    },
    Case(Expr, usize),
    Default(usize),
    Return(Option<Expr>, usize),
    Break(usize),
    Continue(usize),
    Goto(String, usize),
    Label(String, usize),
    Empty(usize),
}

impl Stmt {
    pub fn line(&self) -> usize {
        match self {
            Stmt::Decl(_, l)
            | Stmt::Expr(_, l)
            | Stmt::Block(_, l)
            | Stmt::If { line: l, .. }
            | Stmt::While { line: l, .. }
            | Stmt::DoWhile { line: l, .. }
            | Stmt::For { line: l, .. }
            | Stmt::Switch { line: l, .. }
            | Stmt::Case(_, l)
            | Stmt::Default(l)
            | Stmt::Return(_, l)
            | Stmt::Break(l)
            | Stmt::Continue(l)
            | Stmt::Goto(_, l)
            | Stmt::Label(_, l)
            | Stmt::Empty(l) => *l,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub ret_ty: Type,
    pub params: Vec<VarDecl>,
    pub body: Option<Vec<Stmt>>,
    pub is_static: bool,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<VarDecl>,
    pub line: usize,
}

#[derive(Debug, Clone, Default)]
pub struct TranslationUnit {
    pub functions: Vec<Function>,
    pub globals: Vec<VarDecl>,
    pub structs: Vec<StructDef>,
}
