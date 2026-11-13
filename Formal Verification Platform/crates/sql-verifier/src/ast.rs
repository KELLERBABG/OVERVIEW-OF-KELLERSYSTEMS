//! SQL AST types for the verifier.

use std::fmt;

/// Top-level SQL statement kind.
#[derive(Debug, Clone)]
pub enum SqlStatement {
    Select(SelectStmt),
    Insert(InsertStmt),
    Update(UpdateStmt),
    Delete(DeleteStmt),
    Create(CreateStmt),
}

impl fmt::Display for SqlStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SqlStatement::Select(s) => write!(f, "SELECT ..."),
            SqlStatement::Insert(s) => write!(f, "INSERT INTO {}", s.table),
            SqlStatement::Update(s) => write!(f, "UPDATE {}", s.table),
            SqlStatement::Delete(s) => write!(f, "DELETE FROM {}", s.table),
            SqlStatement::Create(s) => write!(f, "CREATE {}", s.kind),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelectStmt {
    pub ctes: Vec<CteDef>,
    pub select_list: Vec<SelectItem>,
    pub from_clause: Option<FromClause>,
    pub where_clause: Option<Expression>,
    pub group_by: Vec<Expression>,
    pub having: Option<Expression>,
    pub order_by: Vec<OrderByItem>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub distinct: bool,
}

#[derive(Debug, Clone)]
pub struct CteDef {
    pub name: String,
    pub columns: Vec<String>,
    pub body: Box<SelectStmt>,
    pub recursive: bool,
}

#[derive(Debug, Clone)]
pub struct InsertStmt {
    pub table: String,
    pub columns: Vec<String>,
    pub values: Vec<Vec<Expression>>,
    pub select: Option<Box<SelectStmt>>,
}

#[derive(Debug, Clone)]
pub struct UpdateStmt {
    pub table: String,
    pub set_clauses: Vec<(String, Expression)>,
    pub where_clause: Option<Expression>,
}

#[derive(Debug, Clone)]
pub struct DeleteStmt {
    pub table: String,
    pub where_clause: Option<Expression>,
}

#[derive(Debug, Clone)]
pub struct CreateStmt {
    pub kind: String, // "TABLE", "INDEX", etc.
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    pub not_null: bool,
    pub primary_key: bool,
    pub default: Option<Expression>,
}

#[derive(Debug, Clone)]
pub enum DataType {
    Integer,
    BigInt,
    SmallInt,
    Varchar(Option<u64>),
    Text,
    Boolean,
    Float,
    Double,
    Decimal(Option<u64>, Option<u64>),
    Date,
    Timestamp,
    Blob,
    Array(Box<DataType>),
    Custom(String),
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataType::Integer => write!(f, "INTEGER"),
            DataType::BigInt => write!(f, "BIGINT"),
            DataType::SmallInt => write!(f, "SMALLINT"),
            DataType::Varchar(len) => match len {
                Some(l) => write!(f, "VARCHAR({})", l),
                None => write!(f, "VARCHAR"),
            },
            DataType::Text => write!(f, "TEXT"),
            DataType::Boolean => write!(f, "BOOLEAN"),
            DataType::Float => write!(f, "FLOAT"),
            DataType::Double => write!(f, "DOUBLE"),
            DataType::Decimal(p, s) => match (p, s) {
                (Some(p), Some(s)) => write!(f, "DECIMAL({}, {})", p, s),
                _ => write!(f, "DECIMAL"),
            },
            DataType::Date => write!(f, "DATE"),
            DataType::Timestamp => write!(f, "TIMESTAMP"),
            DataType::Blob => write!(f, "BLOB"),
            DataType::Array(inner) => write!(f, "{}[]", inner),
            DataType::Custom(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SelectItem {
    Column(ColumnRef),
    Expression(Expression, Option<String>), // expr, alias
    Star,
    Agg(AggFunction),
}

#[derive(Debug, Clone)]
pub struct ColumnRef {
    pub table: Option<String>,
    pub column: String,
}

impl fmt::Display for ColumnRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.table {
            Some(t) => write!(f, "{}.{}", t, self.column),
            None => write!(f, "{}", self.column),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FromClause {
    pub tables: Vec<TableRef>,
    pub joins: Vec<JoinClause>,
}

#[derive(Debug, Clone)]
pub enum TableRef {
    Simple(String, Option<String>), // table_name, alias
    Subquery(Box<SelectStmt>, String),
    Cte(String, Option<String>),
}

#[derive(Debug, Clone)]
pub struct JoinClause {
    pub kind: JoinKind,
    pub table: TableRef,
    pub on: Option<Expression>,
    pub using: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum JoinKind {
    Inner,
    LeftOuter,
    RightOuter,
    FullOuter,
    Cross,
}

#[derive(Debug, Clone)]
pub struct OrderByItem {
    pub expr: Expression,
    pub direction: OrderDirection,
}

#[derive(Debug, Clone)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Literal),
    Column(ColumnRef),
    BinaryOp(BinaryOpKind, Box<Expression>, Box<Expression>),
    UnaryOp(UnaryOpKind, Box<Expression>),
    FunctionCall(FunctionCall),
    InList(Box<Expression>, Vec<Expression>, bool), // expr, list, negated
    Between(Box<Expression>, Box<Expression>, Box<Expression>, bool), // expr, low, high, negated
    IsNull(Box<Expression>, bool), // expr, negated
    Case(CaseExpr),
    Cast(Box<Expression>, DataType),
    Subquery(Box<SelectStmt>),
    Parameter(usize), // positional parameter index for prepared statements
}

#[derive(Debug, Clone)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
    Blob(Vec<u8>),
}

#[derive(Debug, Clone)]
pub enum BinaryOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    Like,
    Concat,
}

#[derive(Debug, Clone)]
pub enum UnaryOpKind {
    Not,
    Neg,
    BitNot,
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub args: Vec<Expression>,
    pub distinct: bool,
}

#[derive(Debug, Clone)]
pub struct AggFunction {
    pub kind: AggKind,
    pub arg: Option<Box<Expression>>,
    pub distinct: bool,
}

#[derive(Debug, Clone)]
pub enum AggKind {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    Total, // SQLite-specific
}

impl fmt::Display for AggKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AggKind::Count => write!(f, "COUNT"),
            AggKind::Sum => write!(f, "SUM"),
            AggKind::Avg => write!(f, "AVG"),
            AggKind::Min => write!(f, "MIN"),
            AggKind::Max => write!(f, "MAX"),
            AggKind::Total => write!(f, "TOTAL"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CaseExpr {
    pub operand: Option<Box<Expression>>,
    pub whens: Vec<(Expression, Expression)>, // (when_expr, then_expr)
    pub else_expr: Option<Box<Expression>>,
}
