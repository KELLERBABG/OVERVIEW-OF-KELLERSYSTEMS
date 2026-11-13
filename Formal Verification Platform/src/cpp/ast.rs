//! C++ AST nodes for the subset relevant to formal verification.

#![allow(dead_code)]

use std::collections::HashMap;

/// A C++ class or struct declaration.
#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name: String,
    pub base_classes: Vec<String>,
    pub methods: Vec<MethodDecl>,
    pub members: Vec<MemberDecl>,
    pub line: usize,
}

impl ClassDecl {
    pub fn pure_virtual_methods(&self) -> Vec<&MethodDecl> {
        self.methods.iter().filter(|m| m.is_pure_virtual).collect()
    }

    pub fn virtual_methods(&self) -> Vec<&MethodDecl> {
        self.methods.iter().filter(|m| m.is_virtual).collect()
    }
}

/// A method declaration within a class.
#[derive(Debug, Clone)]
pub struct MethodDecl {
    pub name: String,
    pub params: Vec<String>,
    pub return_type: String,
    pub is_virtual: bool,
    pub is_pure_virtual: bool,
    pub is_const: bool,
    pub body_lines: Vec<(usize, String)>,
    pub line: usize,
}

/// A standalone or member function declaration.
#[derive(Debug, Clone)]
pub struct FunctionDecl {
    pub name: String,
    pub class_name: Option<String>,
    pub return_type: String,
    pub params: Vec<ParamDecl>,
    pub line: usize,
    pub body_lines: Vec<(usize, String)>,
    pub is_constructor: bool,
    pub is_destructor: bool,
}

/// A function parameter.
#[derive(Debug, Clone)]
pub struct ParamDecl {
    pub name: String,
    pub type_name: String,
}

/// A member variable declaration.
#[derive(Debug, Clone)]
pub struct MemberDecl {
    pub name: String,
    pub type_name: String,
    pub initializer: Option<String>,
    pub line: usize,
}

/// An STL container usage site.
#[derive(Debug, Clone)]
pub struct ContainerUse {
    pub var_name: String,
    pub container_kind: ContainerKind,
    pub element_type: String,
    pub operations: Vec<ContainerOp>,
    pub initial_size: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerKind {
    Vector,
    String,
    Deque,
    List,
}

/// A single operation on a container that may affect iterator validity.
#[derive(Debug, Clone)]
pub struct ContainerOp {
    pub op_kind: OpKind,
    pub line: usize,
    pub arg_exprs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpKind {
    PushBack,
    PopBack,
    Insert,
    Erase,
    Resize,
    Clear,
    EmplaceBack,
    Assign,
    /// Iterator-reading operation (safe if no invalidation precedes).
    Iterate,
    /// Direct index access.
    Index,
}

impl OpKind {
    /// Returns true if this operation invalidates iterators/references.
    pub fn invalidates_iterators(&self) -> bool {
        matches!(
            self,
            OpKind::PushBack
                | OpKind::PopBack
                | OpKind::Insert
                | OpKind::Erase
                | OpKind::Resize
                | OpKind::Clear
                | OpKind::EmplaceBack
                | OpKind::Assign
        )
    }
}

impl std::fmt::Display for OpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpKind::PushBack => write!(f, "push_back"),
            OpKind::PopBack => write!(f, "pop_back"),
            OpKind::Insert => write!(f, "insert"),
            OpKind::Erase => write!(f, "erase"),
            OpKind::Resize => write!(f, "resize"),
            OpKind::Clear => write!(f, "clear"),
            OpKind::EmplaceBack => write!(f, "emplace_back"),
            OpKind::Assign => write!(f, "assign"),
            OpKind::Iterate => write!(f, "iterate"),
            OpKind::Index => write!(f, "index"),
        }
    }
}

/// A virtual call site in the source.
#[derive(Debug, Clone)]
pub struct VirtualCall {
    pub receiver: String,
    pub method_name: String,
    pub line: usize,
    /// Whether the receiver's static type has a pure virtual override
    /// guarantee (i.e., it is not an abstract class).
    pub receiver_is_abstract: bool,
}

/// The full parsed C++ translation unit.
#[derive(Debug, Clone)]
pub struct TranslationUnit {
    pub classes: Vec<ClassDecl>,
    pub functions: Vec<FunctionDecl>,
    pub containers: Vec<ContainerUse>,
    pub virtual_calls: Vec<VirtualCall>,
    /// Maps class name -> vtable slot index for each virtual method.
    pub vtables: HashMap<String, Vec<String>>,
}

impl TranslationUnit {
    pub fn new() -> Self {
        Self {
            classes: Vec::new(),
            functions: Vec::new(),
            containers: Vec::new(),
            virtual_calls: Vec::new(),
            vtables: HashMap::new(),
        }
    }
}

impl Default for TranslationUnit {
    fn default() -> Self {
        Self::new()
    }
}
