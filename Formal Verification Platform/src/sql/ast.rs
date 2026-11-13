//! AST types for the Keller-VIR SQL frontend.

#[derive(Debug, Clone)]
pub enum Statement {
    CreateTable(CreateTableStmt),
    AlterTable(AlterTableStmt),
    DropTable(Vec<String>),
    Insert(InsertStmt),
    Update(UpdateStmt),
    Delete(DeleteStmt),
    Select(Box<SelectStmt>),
    CreateFunction(RoutineDef),
    CreateProcedure(RoutineDef),
    CreateTrigger(TriggerDef),
    CreateView(ViewDef),
    Other { kind: String, line: usize },
}

#[derive(Debug, Clone)]
pub struct CreateTableStmt {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub constraints: Vec<TableConstraint>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: String,
    pub not_null: bool,
    pub has_default: bool,
    pub default_expr: Option<Expr>,
    pub is_primary_key: bool,
    pub is_auto_increment: bool,
    pub check_expr: Option<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum TableConstraint {
    PrimaryKey(Vec<String>),
    ForeignKey {
        columns: Vec<String>,
        ref_table: String,
        ref_columns: Vec<String>,
    },
    Unique(Vec<String>),
    Check(Expr),
}

#[derive(Debug, Clone)]
pub struct AlterTableStmt {
    pub table_name: String,
    pub actions: Vec<AlterAction>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum AlterAction {
    AddColumn(ColumnDef),
    DropColumn(String),
    AddConstraint(TableConstraint),
    DropConstraint(String),
    ModifyColumn(ColumnDef),
    Other(String),
}

#[derive(Debug, Clone)]
pub struct InsertStmt {
    pub table_name: String,
    pub columns: Vec<String>,
    pub values: Vec<Vec<Expr>>,
    pub select_query: Option<Box<SelectStmt>>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct UpdateStmt {
    pub table_name: String,
    pub assignments: Vec<(String, Expr)>,
    pub where_clause: Option<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct DeleteStmt {
    pub table_name: String,
    pub where_clause: Option<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectStmt {
    pub with_cte: Option<WithClause>,
    pub distinct: bool,
    pub projections: Vec<SelectItem>,
    pub from: Vec<TableRef>,
    pub joins: Vec<JoinClause>,
    pub where_clause: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub union_all: Option<Box<SelectStmt>>,
    pub union: Option<Box<SelectStmt>>,
    pub order_by: Vec<OrderByExpr>,
    pub limit: Option<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WithClause {
    pub recursive: bool,
    pub ctes: Vec<CteDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CteDef {
    pub name: String,
    pub columns: Vec<String>,
    pub query: SelectStmt,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectItem {
    pub expr: Expr,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableRef {
    pub name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JoinClause {
    pub join_type: String,
    pub table: TableRef,
    pub on_clause: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderByExpr {
    pub expr: Expr,
    pub asc: bool,
}

#[derive(Debug, Clone)]
pub struct RoutineDef {
    pub name: String,
    pub params: Vec<String>,
    pub return_type: Option<String>,
    pub body: Vec<Statement>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct TriggerDef {
    pub name: String,
    pub table_name: String,
    pub timing: String,
    pub event: String,
    pub body: Vec<Statement>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct ViewDef {
    pub name: String,
    pub query: SelectStmt,
    pub line: usize,
}

// ─── Expressions ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),
    ColumnRef {
        table: Option<String>,
        column: String,
        line: usize,
    },
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOpKind,
        right: Box<Expr>,
        line: usize,
    },
    UnaryOp {
        op: UnaryOpKind,
        expr: Box<Expr>,
        line: usize,
    },
    FunctionCall {
        name: String,
        args: Vec<Expr>,
        line: usize,
    },
    Case {
        operand: Option<Box<Expr>>,
        when_clauses: Vec<(Expr, Expr)>,
        else_clause: Option<Box<Expr>>,
        line: usize,
    },
    IsNull {
        expr: Box<Expr>,
        is_not: bool,
        line: usize,
    },
    InList {
        expr: Box<Expr>,
        list: Vec<Expr>,
        is_not: bool,
        line: usize,
    },
    InSubquery {
        expr: Box<Expr>,
        subquery: Box<SelectStmt>,
        is_not: bool,
        line: usize,
    },
    Between {
        expr: Box<Expr>,
        low: Box<Expr>,
        high: Box<Expr>,
        is_not: bool,
        line: usize,
    },
    Subquery(Box<SelectStmt>),
    List(Vec<Expr>),
    Raw(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Null,
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    Concat,
    Like,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOpKind {
    Plus,
    Minus,
    Not,
}

impl Expr {
    pub fn is_literal_null(&self) -> bool {
        matches!(self, Expr::Literal(Literal::Null))
    }

    pub fn is_literal_zero(&self) -> bool {
        match self {
            Expr::Literal(Literal::Integer(0)) => true,
            Expr::Literal(Literal::Float(f)) if *f == 0.0 => true,
            _ => false,
        }
    }

    pub fn is_literal_nonzero(&self) -> bool {
        match self {
            Expr::Literal(Literal::Integer(n)) => *n != 0,
            Expr::Literal(Literal::Float(f)) => *f != 0.0,
            _ => false,
        }
    }

    /// Check if this expression is wrapped in `NULLIF(col, 0)`.
    pub fn is_nullif_guard(&self) -> bool {
        if let Expr::FunctionCall { name, args, .. } = self {
            if name.eq_ignore_ascii_case("nullif") && args.len() >= 2 {
                if args[1].is_literal_zero() {
                    return true;
                }
            }
        }
        false
    }
}
