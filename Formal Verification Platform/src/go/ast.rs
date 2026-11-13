/// Go AST types for the Keller-VIR frontend.

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub imports: Vec<Import>,
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub receiver: Option<Param>,
    pub params: Vec<Param>,
    pub results: Vec<Param>,
    pub body: Block,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: Option<String>,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

// ─── Statements ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign(AssignStmt),
    Decl(DclStmt),
    IncDec(Expr, bool, usize), // expr++ (true) or expr-- (false)
    If(IfStmt),
    For(ForStmt),
    Range(RangeStmt),
    Switch(SwitchStmt),
    Return(ReturnStmt),
    Go(GoStmt),
    Send(SendStmt),
    Recv(RecvStmt),
    Close(CloseStmt),
    Select(SelectStmt),
    Defer(DeferStmt),
    Branch(BranchKind, Option<String>, usize),
    Block(Block),
    Expr(ExprStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BranchKind {
    Break,
    Continue,
    Goto,
    Fallthrough,
}

#[derive(Debug, Clone)]
pub struct AssignStmt {
    pub lhs: Vec<Expr>,
    pub rhs: Vec<Expr>,
    pub is_define: bool, // true if :=
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct DclStmt {
    pub names: Vec<String>,
    pub ty: Option<Type>,
    pub init: Option<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub init: Option<Box<Stmt>>,
    pub cond: Expr,
    pub then: Block,
    pub els: Option<Block>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub init: Option<Box<Stmt>>,
    pub cond: Option<Expr>,
    pub post: Option<Box<Stmt>>,
    pub body: Block,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct RangeStmt {
    pub key: Option<String>,
    pub val: Option<String>,
    pub iter: Expr,
    pub body: Block,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct SwitchStmt {
    pub init: Option<Box<Stmt>>,
    pub tag: Option<Box<Expr>>,
    pub cases: Vec<SwitchCase>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub exprs: Vec<Expr>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub values: Vec<Expr>,
    pub line: usize,
}

/// `go f(args...)` — spawns a goroutine.
#[derive(Debug, Clone)]
pub struct GoStmt {
    pub call: Expr,
    pub line: usize,
}

/// `ch <- expr`
#[derive(Debug, Clone)]
pub struct SendStmt {
    pub ch: Box<Expr>,
    pub val: Box<Expr>,
    pub line: usize,
}

/// `x := <-ch` or `x = <-ch`
#[derive(Debug, Clone)]
pub struct RecvStmt {
    pub target: Option<String>,
    pub ch: Box<Expr>,
    pub ok: bool,
    pub line: usize,
}

/// `close(ch)`
#[derive(Debug, Clone)]
pub struct CloseStmt {
    pub ch: Box<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct SelectStmt {
    pub cases: Vec<SelectCase>,
    pub default: Option<Block>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct SelectCase {
    pub comm: Option<Stmt>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct DeferStmt {
    pub call: Expr,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub expr: Expr,
    pub line: usize,
}

// ─── Expressions ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    Ident(Ident),
    Literal(Literal),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Call(CallExpr),
    Index(IndexExpr),
    Slice(SliceExpr),
    Field(FieldExpr),
    TypeAssert(TypeAssertExpr),
    CompositeLit(CompositeLit),
    KeyValue(Box<Expr>, Box<Expr>, usize),
    FuncLit(FuncLit),
    ChanMake(ChanMake),
    Paren(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct Ident {
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64, usize),
    Float(f64, usize),
    Str(String, usize),
    Bool(bool, usize),
    Nil(usize),
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub op: BinOp,
    pub lhs: Box<Expr>,
    pub rhs: Box<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Lt, Gt, Le, Ge, Eq, Ne,
    And, Or, BitAnd, BitOr, BitXor, BitClear,
    Shl, Shr,
    Arrow, // channel send in expression context
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub op: UnOp,
    pub operand: Box<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Neg, Not, Receive, Deref, Addr, BitNot,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub func: Box<Expr>,
    pub args: Vec<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct IndexExpr {
    pub base: Box<Expr>,
    pub index: Box<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct SliceExpr {
    pub base: Box<Expr>,
    pub low: Option<Box<Expr>>,
    pub high: Option<Box<Expr>>,
    pub max: Option<Box<Expr>>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct FieldExpr {
    pub base: Box<Expr>,
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct TypeAssertExpr {
    pub base: Box<Expr>,
    pub ty: Option<Type>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct CompositeLit {
    pub ty: Option<Type>,
    pub elems: Vec<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct FuncLit {
    pub params: Vec<Param>,
    pub results: Vec<Param>,
    pub body: Block,
    pub line: usize,
}

/// `make(chan T, cap)` or `make([]T, len, cap)`
#[derive(Debug, Clone)]
pub struct ChanMake {
    pub ty: Type,
    pub cap: Option<Box<Expr>>,
    pub line: usize,
}

// ─── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Basic(BasicType),
    Named(String),
    Pointer(Box<Type>),
    Slice(Box<Type>),
    Array(Option<usize>, Box<Type>),
    Map(Box<Type>, Box<Type>),
    Chan(ChanDir, Box<Type>),
    Struct(Vec<StructField>),
    Interface(Vec<String>),
    Func(Vec<Type>, Vec<Type>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BasicType {
    Int, Int8, Int16, Int32, Int64,
    Uint, Uint8, Uint16, Uint32, Uint64, Uintptr,
    Float32, Float64, Complex64, Complex128,
    Bool, String, Byte, Rune, Error, Any,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChanDir {
    Send, Recv, Both,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
}
