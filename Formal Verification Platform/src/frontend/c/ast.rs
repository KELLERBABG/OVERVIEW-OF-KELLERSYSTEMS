//! C Frontend Abstract Syntax Tree.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CType {
    Void,
    Int,
    Char,
    Pointer(Box<CType>),
    Array(Box<CType>, usize),
    Named(String),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub return_type: CType,
    pub params: Vec<(String, CType)>,
    pub body_offset: usize,
}

#[derive(Debug, Clone)]
pub struct TranslationUnit {
    pub filename: String,
    pub functions: Vec<Function>,
}
