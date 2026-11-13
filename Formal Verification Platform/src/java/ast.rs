#![allow(dead_code, unused_variables, unused_imports, private_interfaces)]

//! Java AST definitions for the Keller-VIR Java frontend.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLoc {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct CompilationUnit {
    pub package: Option<String>,
    pub imports: Vec<String>,
    pub types: Vec<TypeDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind {
    Class,
    Interface,
    Enum,
    Record,
    Annotation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaType {
    pub name: String,
    pub is_primitive: bool,
    pub array_dims: usize,
}

impl JavaType {
    pub fn new(name: &str) -> Self {
        let is_prim = matches!(
            name,
            "byte" | "short" | "int" | "long" | "float" | "double" | "char" | "boolean" | "void"
        );
        Self {
            name: name.to_string(),
            is_primitive: is_prim,
            array_dims: 0,
        }
    }

    pub fn array(mut self, dims: usize) -> Self {
        self.array_dims += dims;
        self
    }

    pub fn is_reference(&self) -> bool {
        self.array_dims > 0 || !self.is_primitive
    }
}

#[derive(Debug, Clone)]
pub struct TypeDecl {
    pub name: String,
    pub kind: TypeKind,
    pub modifiers: Vec<String>,
    pub type_params: Vec<String>,
    pub extends: Option<JavaType>,
    pub implements: Vec<JavaType>,
    pub fields: Vec<FieldDecl>,
    pub methods: Vec<MethodDecl>,
    pub constructors: Vec<ConstructorDecl>,
    pub nested_types: Vec<TypeDecl>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct FieldDecl {
    pub name: String,
    pub ty: JavaType,
    pub modifiers: Vec<String>,
    pub init: Option<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: JavaType,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct MethodDecl {
    pub name: String,
    pub modifiers: Vec<String>,
    pub return_ty: Option<JavaType>,
    pub params: Vec<Param>,
    pub body: Option<Block>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct ConstructorDecl {
    pub name: String,
    pub modifiers: Vec<String>,
    pub params: Vec<Param>,
    pub body: Option<Block>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Block(Block),
    VarDecl {
        ty: JavaType,
        vars: Vec<(String, Option<Expr>)>,
        line: usize,
    },
    Expr(Expr, usize),
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
        init: Option<Vec<Stmt>>,
        cond: Option<Expr>,
        update: Option<Vec<Expr>>,
        body: Box<Stmt>,
        line: usize,
    },
    ForEach {
        var_name: String,
        var_ty: JavaType,
        iterable: Expr,
        body: Box<Stmt>,
        line: usize,
    },
    Switch {
        selector: Expr,
        cases: Vec<SwitchCase>,
        line: usize,
    },
    Return(Option<Expr>, usize),
    Throw(Expr, usize),
    Try {
        resources: Vec<Stmt>,
        body: Block,
        catches: Vec<CatchClause>,
        finally_body: Option<Block>,
        line: usize,
    },
    Assert {
        cond: Expr,
        msg: Option<Expr>,
        line: usize,
    },
    Break(Option<String>, usize),
    Continue(Option<String>, usize),
    Synchronized {
        lock: Expr,
        body: Block,
        line: usize,
    },
    Empty(usize),
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub labels: Vec<Expr>,
    pub stmts: Vec<Stmt>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct CatchClause {
    pub param_name: String,
    pub param_types: Vec<String>,
    pub body: Block,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Null(usize),
    Bool(bool, usize),
    Int(i64, usize),
    Float(f64, usize),
    StringLit(String, usize),
    CharLit(char, usize),
    ClassLiteral(String, usize),

    Ident(String, usize),
    This(usize),
    Super(usize),

    FieldAccess {
        target: Box<Expr>,
        field: String,
        line: usize,
    },
    MethodCall {
        target: Option<Box<Expr>>,
        method: String,
        args: Vec<Expr>,
        line: usize,
    },
    ArrayAccess {
        array: Box<Expr>,
        index: Box<Expr>,
        line: usize,
    },
    NewObject {
        ty: JavaType,
        args: Vec<Expr>,
        line: usize,
    },
    NewArray {
        element_ty: JavaType,
        dims: Vec<Expr>,
        initializer: Option<Vec<Expr>>,
        line: usize,
    },
    Cast {
        target_ty: JavaType,
        expr: Box<Expr>,
        line: usize,
    },
    InstanceOf {
        expr: Box<Expr>,
        check_ty: JavaType,
        pattern_var: Option<String>,
        line: usize,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        line: usize,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
        line: usize,
    },
    Ternary {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        line: usize,
    },
    Assign {
        op: AssignOp,
        target: Box<Expr>,
        value: Box<Expr>,
        line: usize,
    },
    Lambda {
        params: Vec<String>,
        body_expr: Option<Box<Expr>>,
        body_block: Option<Block>,
        line: usize,
    },
    MethodRef {
        target: Box<Expr>,
        method: String,
        line: usize,
    },
}

impl Expr {
    pub fn line(&self) -> usize {
        match self {
            Expr::Null(l)
            | Expr::Bool(_, l)
            | Expr::Int(_, l)
            | Expr::Float(_, l)
            | Expr::StringLit(_, l)
            | Expr::CharLit(_, l)
            | Expr::ClassLiteral(_, l)
            | Expr::Ident(_, l)
            | Expr::This(l)
            | Expr::Super(l)
            | Expr::FieldAccess { line: l, .. }
            | Expr::MethodCall { line: l, .. }
            | Expr::ArrayAccess { line: l, .. }
            | Expr::NewObject { line: l, .. }
            | Expr::NewArray { line: l, .. }
            | Expr::Cast { line: l, .. }
            | Expr::InstanceOf { line: l, .. }
            | Expr::Unary { line: l, .. }
            | Expr::Binary { line: l, .. }
            | Expr::Ternary { line: l, .. }
            | Expr::Assign { line: l, .. }
            | Expr::Lambda { line: l, .. }
            | Expr::MethodRef { line: l, .. } => *l,
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
    Ushr,
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
    UshrAssign,
}
