//! C++ Abstract Syntax Tree types for the Keller-VIR frontend.
//!
//! Models the subset of C++ needed for class/vtable analysis and
//! STL container invariant checking.

/// A C++ translation unit (the root of the AST).
#[derive(Debug, Clone)]
pub struct TranslationUnit {
    /// All class/struct declarations in this TU.
    pub classes: Vec<ClassDecl>,
    /// Top-level function definitions.
    pub functions: Vec<FunctionDef>,
    /// Global variable declarations.
    pub variables: Vec<VariableDecl>,
    /// Included headers (for cross-TU analysis).
    pub includes: Vec<String>,
}

/// A class or struct declaration.
#[derive(Debug, Clone)]
pub struct ClassDecl {
    /// The class name.
    pub name: String,
    /// Base classes (for inheritance hierarchy).
    pub bases: Vec<BaseSpec>,
    /// Member methods.
    pub methods: Vec<MethodDecl>,
    /// Data members.
    pub members: Vec<MemberDecl>,
    /// Whether this is an abstract class (has at least one pure virtual).
    pub is_abstract: bool,
    /// Source location of the class keyword.
    pub location: super::SourceLocation,
}

impl ClassDecl {
    /// Compute whether this class is abstract based on its methods.
    pub fn compute_is_abstract(&mut self) {
        self.is_abstract = self.methods.iter().any(|m| m.is_pure_virtual);
    }
}

/// A base class specification in an inheritance list.
#[derive(Debug, Clone)]
pub struct BaseSpec {
    /// The base class name.
    pub name: String,
    /// Access specifier (public, protected, private).
    pub access: AccessSpecifier,
    /// Virtual inheritance flag.
    pub is_virtual: bool,
}

/// C++ access specifier.
#[derive(Debug, Clone, PartialEq)]
pub enum AccessSpecifier {
    Public,
    Protected,
    Private,
}

/// A method declaration within a class.
#[derive(Debug, Clone)]
pub struct MethodDecl {
    /// Method name.
    pub name: String,
    /// Return type (string representation).
    pub return_type: String,
    /// Parameter list.
    pub params: Vec<ParamDecl>,
    /// Whether this is a virtual method.
    pub is_virtual: bool,
    /// Whether this is a pure virtual (= 0).
    pub is_pure_virtual: bool,
    /// Whether this is a static method.
    pub is_static: bool,
    /// Whether this is a constructor or destructor.
    pub is_special: Option<SpecialKind>,
    /// The vtable slot index (computed during lowering).
    pub vtable_slot: Option<usize>,
    /// Source location.
    pub location: super::SourceLocation,
}

/// Special method kinds.
#[derive(Debug, Clone, PartialEq)]
pub enum SpecialKind {
    Constructor,
    Destructor,
}

/// A parameter declaration.
#[derive(Debug, Clone)]
pub struct ParamDecl {
    /// Parameter type (string representation).
    pub type_name: String,
    /// Parameter name (may be empty for unnamed params).
    pub name: Option<String>,
    /// Source location.
    pub location: super::SourceLocation,
}

/// A data member declaration.
#[derive(Debug, Clone)]
pub struct MemberDecl {
    /// Member type.
    pub type_name: String,
    /// Member name.
    pub name: String,
    /// Default initializer (if any).
    pub default_value: Option<String>,
    /// Source location.
    pub location: super::SourceLocation,
}

/// A top-level function definition.
#[derive(Debug, Clone)]
pub struct FunctionDef {
    /// Function name.
    pub name: String,
    /// Return type.
    pub return_type: String,
    /// Parameters.
    pub params: Vec<ParamDecl>,
    /// Body statements (simplified).
    pub body: Vec<Statement>,
    /// Source location of the function keyword/name.
    pub location: super::SourceLocation,
}

/// A global or local variable declaration.
#[derive(Debug, Clone)]
pub struct VariableDecl {
    /// Variable type.
    pub type_name: String,
    /// Variable name.
    pub name: String,
    /// Initializer (if any).
    pub initializer: Option<String>,
    /// Source location.
    pub location: super::SourceLocation,
}

/// A simplified statement node for function bodies.
#[derive(Debug, Clone)]
pub enum Statement {
    /// An expression statement.
    Expr(Expression),
    /// A block of statements.
    Block(Vec<Statement>),
    /// A for-loop (simplified).
    ForLoop {
        init: Option<Box<Statement>>,
        condition: Option<Box<Expression>>,
        increment: Option<Box<Statement>>,
        body: Box<Statement>,
    },
    /// A range-for loop.
    RangeFor {
        var_type: String,
        var_name: String,
        iterable_expr: Expression,
        body: Box<Statement>,
    },
    /// An if statement.
    If {
        condition: Box<Expression>,
        then_branch: Box<Statement>,
        else_branch: Option<Box<Statement>>,
    },
    /// A return statement.
    Return(Option<Box<Expression>>),
}

/// A simplified expression node.
#[derive(Debug, Clone)]
pub enum Expression {
    /// A literal value.
    Literal(String),
    /// A variable reference.
    VarRef(String),
    /// A function/method call.
    Call {
        callee: String,
        args: Vec<Expression>,
    },
    /// A member access (obj.member or obj->member).
    MemberAccess {
        base: Box<Expression>,
        member: String,
        is_arrow: bool,
    },
    /// An assignment.
    Assign {
        target: Box<Expression>,
        value: Box<Expression>,
    },
    /// A binary operation.
    BinaryOp {
        op: String,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
    },
}
