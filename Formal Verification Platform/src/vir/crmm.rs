//! C-RMM (C Reduced Memory Model) program representation.
//!
//! Captures the essential memory operations of C programs for formal
//! verification: allocation, deallocation, pointer arithmetic, and
//! array access.

use std::fmt;

/// Kind of variable in C-RMM.
#[derive(Debug, Clone, PartialEq)]
pub enum CrmmVarKind {
    Scalar,
    Pointer,
}

/// A C-RMM operation.
#[derive(Debug, Clone)]
pub enum CrmmOp {
    /// Heap allocation: target = malloc(size)
    Alloc { target: String, size: usize },
    /// Deallocation: free(target)
    Free { target: String },
    /// Dereference: *target
    Deref { target: String },
    /// Pointer arithmetic: target += delta (in element units)
    PtrArith { target: String, delta: i64 },
    /// Array access: target[index]
    ArrayAccess { target: String, index: usize },
    /// Assignment: target = value
    Assign { target: String, source: String },
}

impl fmt::Display for CrmmOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CrmmOp::Alloc { target, size } => write!(f, "{} = malloc({})", target, size),
            CrmmOp::Free { target } => write!(f, "free({})", target),
            CrmmOp::Deref { target } => write!(f, "*{}", target),
            CrmmOp::PtrArith { target, delta } => write!(f, "{} += {}", target, delta),
            CrmmOp::ArrayAccess { target, index } => write!(f, "{}[{}]", target, index),
            CrmmOp::Assign { target, source } => write!(f, "{} = {}", target, source),
        }
    }
}

/// A function in the C-RMM program.
#[derive(Debug, Clone)]
pub struct CrmmFunction {
    pub name: String,
    pub params: Vec<CrmmParam>,
    pub locals: Vec<CrmmLocal>,
    pub ops: Vec<CrmmOp>,
}

/// A parameter in C-RMM.
#[derive(Debug, Clone)]
pub struct CrmmParam {
    pub name: String,
    pub kind: CrmmVarKind,
}

/// A local variable in C-RMM.
#[derive(Debug, Clone)]
pub struct CrmmLocal {
    pub name: String,
    pub kind: CrmmVarKind,
}

/// The full C-RMM program.
#[derive(Debug, Clone)]
pub struct CrmmProgram {
    pub globals: Vec<CrmmGlobal>,
    pub functions: Vec<CrmmFunction>,
}

/// A global variable in C-RMM.
#[derive(Debug, Clone)]
pub struct CrmmGlobal {
    pub name: String,
    pub kind: CrmmVarKind,
}

impl CrmmProgram {
    pub fn new() -> Self {
        Self {
            globals: Vec::new(),
            functions: Vec::new(),
        }
    }

    pub fn add_global(&mut self, name: &str, kind: CrmmVarKind) {
        self.globals.push(CrmmGlobal { name: name.to_string(), kind });
    }

    pub fn add_function(&mut self, name: &str) -> usize {
        let id = self.functions.len();
        self.functions.push(CrmmFunction {
            name: name.to_string(),
            params: Vec::new(),
            locals: Vec::new(),
            ops: Vec::new(),
        });
        id
    }

    pub fn add_param(&mut self, func_id: usize, name: &str, kind: CrmmVarKind) {
        self.functions[func_id].params.push(CrmmParam { name: name.to_string(), kind });
    }

    pub fn add_local(&mut self, func_id: usize, name: &str, kind: CrmmVarKind) {
        self.functions[func_id].locals.push(CrmmLocal { name: name.to_string(), kind });
    }

    pub fn emit(&mut self, func_id: usize, op: CrmmOp) {
        self.functions[func_id].ops.push(op);
    }
}

impl Default for CrmmProgram {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crmm_program_construction() {
        let mut prog = CrmmProgram::new();
        prog.add_global("counter", CrmmVarKind::Scalar);
        let fid = prog.add_function("main");
        prog.add_param(fid, "argc", CrmmVarKind::Scalar);
        prog.add_local(fid, "p", CrmmVarKind::Pointer);
        prog.emit(fid, CrmmOp::Alloc { target: "p".into(), size: 100 });
        prog.emit(fid, CrmmOp::Free { target: "p".into() });

        assert_eq!(prog.globals.len(), 1);
        assert_eq!(prog.functions.len(), 1);
        assert_eq!(prog.functions[0].ops.len(), 2);
    }
}
