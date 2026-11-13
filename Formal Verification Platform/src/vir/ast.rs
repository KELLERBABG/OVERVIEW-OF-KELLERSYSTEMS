//! Abstract Syntax Tree and Types for Keller-VIR.
//!
//! Formal grammar from Section 1.1 and C-RMM from Section 1.2.

use crate::vir::effect::EffectSet;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VirType {
    Bool,
    Int(u32),     // Bitwidth (e.g. 8, 16, 32, 64, 128)
    Uint(u32),    // Bitwidth
    Float(u32),   // 32, 64
    Pointer(Box<VirType>),
    Array(Box<VirType>, usize),
    Tuple(Vec<VirType>),
    Custom(String),
}

/// Capability-Based Region Memory Model (C-RMM) affine quad:
/// Ptr = <RegionID, BaseOffset, DynamicOffset, Extent>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtrCapability {
    pub region_id: u64,
    pub base_offset: i64,
    pub dynamic_offset_var: String,
    pub extent: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticKind {
    Overflow,
    DivisionByZero,
    OutOfBounds,
    NullPointer,
    MemorySafety,
    UserAssertion,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VirBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VirUnaryOp {
    Neg,
    Not,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VirExpr {
    Var(String),
    ConstInt(i128, u32), // value, bitwidth (0 = unbounded mathematical integer)
    ConstFloat(String),  // string representation of float/real
    ConstBool(bool),
    Binary(VirBinaryOp, Box<VirExpr>, Box<VirExpr>),
    Unary(VirUnaryOp, Box<VirExpr>),
    Ite(Box<VirExpr>, Box<VirExpr>, Box<VirExpr>),
    Load {
        mem_token: String,
        address: Box<VirExpr>,
        ty: VirType,
    },
    Call {
        target: String,
        args: Vec<VirExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VirStatement {
    Assign(String, VirExpr),
    Store {
        mem_token: String,
        address: VirExpr,
        value: VirExpr,
    },
    Assert(VirExpr, DiagnosticKind),
    Acquire(String),
    Release(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VirTerminator {
    Goto(String),
    Branch {
        guard: VirExpr,
        then_block: String,
        else_block: String,
    },
    Return(Option<VirExpr>),
    Unreachable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirBasicBlock {
    pub label: String,
    pub statements: Vec<VirStatement>,
    pub terminator: VirTerminator,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirParam {
    pub name: String,
    pub ty: VirType,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirFunction {
    pub name: String,
    pub params: Vec<VirParam>,
    pub return_type: VirType,
    pub effects: EffectSet,
    pub blocks: Vec<VirBasicBlock>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VirProgram {
    pub functions: Vec<VirFunction>,
}
