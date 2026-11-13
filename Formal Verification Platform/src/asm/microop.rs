//! Keller-VIR Micro-Op IR
//!
//! Each micro-op is a node in the SSA hypergraph. The hypergraph edges
//! represent data dependencies between micro-ops. This module defines the
//! core types that the disassembler emits and the verifier consumes.

use std::fmt;

/// Target architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arch {
    X86_64,
    AArch64,
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arch::X86_64 => write!(f, "x86_64"),
            Arch::AArch64 => write!(f, "aarch64"),
        }
    }
}

/// Kind of operation encoded in a micro-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpKind {
    /// Integer addition (reg = reg + reg / reg + imm)
    Add,
    /// Integer subtraction
    Sub,
    /// Integer multiplication
    Mul,
    /// Integer division
    Div,
    /// Logical AND
    And,
    /// Logical OR
    Or,
    /// Logical XOR
    Xor,
    /// Bitwise NOT
    Not,
    /// Left shift
    Shl,
    /// Right shift (logical)
    Shr,
    /// Arithmetic right shift
    Sar,
    /// Move (register-to-register or immediate-to-register)
    Mov,
    /// Memory load
    Load,
    /// Memory store
    Store,
    /// Conditional branch
    BranchCond,
    /// Unconditional branch
    BranchUncond,
    /// Call
    Call,
    /// Return
    Ret,
    /// Push to stack
    Push,
    /// Pop from stack
    Pop,
    /// Compare (sets flags)
    Cmp,
    /// Test (AND + set flags)
    Test,
    /// Increment
    Inc,
    /// Decrement
    Dec,
    /// Negate
    Neg,
    /// No-op
    Nop,
    /// FENCE / barrier
    Fence,
    /// LEA (load effective address)
    Lea,
    /// IMUL (signed multiply)
    Imul,
    /// IDIV (signed divide)
    Idiv,
    /// CDQ/CDQE sign-extend for division
    Cdq,
    /// MOVZX / MOVSX (zero/sign extend move)
    Movzx,
    /// LEAVE (epilogue: mov rsp,rbp; pop rbp)
    Leave,
    /// ENTER (prologue variant)
    Enter,
    /// Other / unknown
    Other,
}

impl fmt::Display for OpKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            OpKind::Add => "add",
            OpKind::Sub => "sub",
            OpKind::Mul => "mul",
            OpKind::Div => "div",
            OpKind::And => "and",
            OpKind::Or => "or",
            OpKind::Xor => "xor",
            OpKind::Not => "not",
            OpKind::Shl => "shl",
            OpKind::Shr => "shr",
            OpKind::Sar => "sar",
            OpKind::Mov => "mov",
            OpKind::Load => "load",
            OpKind::Store => "store",
            OpKind::BranchCond => "branch_cond",
            OpKind::BranchUncond => "branch_uncond",
            OpKind::Call => "call",
            OpKind::Ret => "ret",
            OpKind::Push => "push",
            OpKind::Pop => "pop",
            OpKind::Cmp => "cmp",
            OpKind::Test => "test",
            OpKind::Inc => "inc",
            OpKind::Dec => "dec",
            OpKind::Neg => "neg",
            OpKind::Nop => "nop",
            OpKind::Fence => "fence",
            OpKind::Lea => "lea",
            OpKind::Imul => "imul",
            OpKind::Idiv => "idiv",
            OpKind::Cdq => "cdq",
            OpKind::Movzx => "movzx",
            OpKind::Leave => "leave",
            OpKind::Enter => "enter",
            OpKind::Other => "other",
        };
        write!(f, "{}", name)
    }
}

/// An operand in a micro-op.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operand {
    /// A register (name is architecture-specific, e.g., "rax", "x0").
    Reg(String),
    /// An immediate value.
    Imm(i64),
    /// A memory reference: base + index*scale + displacement.
    Mem {
        base: String,
        index: Option<String>,
        scale: i32,
        disp: i64,
    },
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Reg(name) => write!(f, "{}", name),
            Operand::Imm(v) => write!(f, "0x{:x}", v & 0xFFFFFFFFFFFFFFFF),
            Operand::Mem { base, index, scale, disp } => {
                write!(f, "[{}]", base)?;
                if let Some(idx) = index {
                    write!(f, "+{}*{}", idx, scale)?;
                }
                if disp != 0 {
                    write!(f, "{}{}", if disp > 0 { '+' } else { '-' }, disp.abs())?;
                }
                Ok(())
            }
        }
    }
}

/// A single micro-op node in the Keller-VIR SSA hypergraph.
#[derive(Debug, Clone)]
pub struct MicroOp {
    /// Unique node identifier within the hypergraph.
    pub node_id: u64,
    /// The architecture this op belongs to.
    pub arch: Arch,
    /// The operation kind.
    pub kind: OpKind,
    /// Destination operand (if any).
    pub dest: Option<Operand>,
    /// Source operands.
    pub sources: Vec<Operand>,
    /// Byte offset in the original binary where this instruction starts.
    pub offset: usize,
    /// Size of the encoded instruction in bytes.
    pub size: usize,
    /// Raw machine code bytes.
    pub raw_bytes: Vec<u8>,
    /// Flags modified by this op (x86_64: CF,PF,AF,ZF,SF,OF; AArch64: NZCV).
    pub flags_modified: u32,
    /// Flags read by this op.
    pub flags_read: u32,
    /// Whether this op is a control-flow transfer.
    pub is_cft: bool,
}

impl MicroOp {
    /// Returns true if this micro-op writes to the stack pointer.
    pub fn modifies_sp(&self) -> bool {
        match self.kind {
            OpKind::Push | OpKind::Pop | OpKind::Add | OpKind::Sub => {
                self.dest.as_ref()
                    .map(|d| matches_sp(d))
                    .unwrap_or(false)
                || self.sources.iter().any(matches_sp)
            }
            _ => false,
        }
    }

    /// Returns true if this micro-op writes to the return address slot.
    pub fn writes_return_address(&self) -> bool {
        matches!(self.kind, OpKind::Call | OpKind::Ret)
    }
}

fn matches_sp(op: &Operand) -> bool {
    match op {
        Operand::Reg(name) => {
            name == "rsp" || name == "sp" || name == "xsp" || name == "r13" // r13 sometimes used as SP in AArch64
        }
        _ => false,
    }
}

/// Flag bit constants (x86_64 EFLAGS subset).
pub const FLAG_CF: u32 = 1 << 0;
pub const FLAG_PF: u32 = 1 << 1;
pub const FLAG_AF: u32 = 1 << 2;
pub const FLAG_ZF: u32 = 1 << 3;
pub const FLAG_SF: u32 = 1 << 4;
pub const FLAG_OF: u32 = 1 << 5;

/// Flag bit constants (AArch64 NZCV).
pub const FLAG_N: u32 = 1 << 0;
pub const FLAG_Z: u32 = 1 << 1;
pub const FLAG_C: u32 = 1 << 2;
pub const FLAG_V: u32 = 1 << 3;
