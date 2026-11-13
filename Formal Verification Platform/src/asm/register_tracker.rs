//! Register state clobber tracking.
//!
//! Maintains a liveness map across the micro-op sequence, identifying
//! which registers are written (clobbered) and which are live-in/live-out
//! at each program point. This feeds into the C-RMM analysis to determine
//! whether a buffer overflow could corrupt a live register holding a
//! critical value (e.g., return address in RBP or frame pointer).

use std::collections::{HashMap, HashSet};
use crate::asm::microop::{MicroOp, Operand};

/// A snapshot of register state at a program point.
#[derive(Debug, Clone)]
pub struct RegisterState {
    /// Map from register name to whether it is considered "live" (holding
    /// a value that will be read before being overwritten).
    pub live: HashMap<String, bool>,
    /// Registers written since the last snapshot.
    pub clobbered: Vec<String>,
}

impl RegisterState {
    pub fn new() -> Self {
        Self {
            live: HashMap::new(),
            clobbered: Vec::new(),
        }
    }

    /// Mark a register as live-in.
    pub fn mark_live(&mut self, reg: &str) {
        self.live.insert(reg.to_string(), true);
    }

    /// Mark a register as dead (no longer needed).
    pub fn mark_dead(&mut self, reg: &str) {
        self.live.insert(reg.to_string(), false);
    }

    /// Check if a register is currently live.
    pub fn is_live(&self, reg: &str) -> bool {
        self.live.get(reg).copied().unwrap_or(false)
    }
}

/// The register tracker processes a sequence of micro-ops and produces
/// clobber information for each op.
pub struct RegisterTracker {
    /// Current register liveness state.
    pub state: RegisterState,
    /// For each node_id, the set of registers clobbered by that op.
    pub clobbers: HashMap<u64, Vec<String>>,
    /// Registers that are live at function entry (assumed).
    pub entry_lives: HashSet<String>,
}

impl RegisterTracker {
    pub fn new() -> Self {
        Self {
            state: RegisterState::new(),
            clobbers: HashMap::new(),
            entry_lives: HashSet::new(),
        }
    }

    /// Initialize with assumed live registers at function entry.
    /// For x86_64 System V ABI: RDI, RSI, RDX, RCX, R8, R9 are argument regs.
    /// RBP is frame pointer (live if frame is used).
    pub fn init_x86_64_entry(&mut self) {
        for reg in &["rdi", "rsi", "rdx", "rcx", "r8", "r9", "rax"] {
            self.entry_lives.insert(reg.to_string());
            self.state.mark_live(reg);
        }
    }

    /// Initialize with assumed live registers at function entry.
    /// For AArch64 AAPCS64: X0-X7 are argument regs, X29 (FP), X30 (LR).
    pub fn init_aarch64_entry(&mut self) {
        for i in 0..8u32 {
            let reg = format!("x{}", i);
            self.entry_lives.insert(reg.clone());
            self.state.mark_live(&reg);
        }
        // Frame pointer and link register are always live at entry
        self.entry_lives.insert("x29".to_string());
        self.entry_lives.insert("x30".to_string());
        self.state.mark_live("x29");
        self.state.mark_live("x30");
    }

    /// Process a single micro-op, updating liveness and recording clobbers.
    pub fn process_op(&mut self, op: &MicroOp) {
        let mut clobbered = Vec::new();

        // Determine destination register (if any)
        if let Some(dest) = &op.dest {
            if let Operand::Reg(reg_name) = dest {
                // If the register was live, it's now clobbered
                if self.state.is_live(reg_name) {
                    clobbered.push(reg_name.clone());
                }
                // After write, the register is live (it holds a new value)
                self.state.mark_live(reg_name);
            }
        }

        // Source operands make registers live-in (they were read)
        for src in &op.sources {
            if let Operand::Reg(reg_name) = src {
                if !self.state.is_live(reg_name) {
                    self.state.mark_live(reg_name);
                }
            }
        }

        // Special handling: RET kills the return address (in LR / implicit stack)
        if op.kind == crate::asm::microop::OpKind::Ret {
            // After RET, the function's frame is gone.
            // All callee-saved registers become "live-out" but we mark
            // argument registers as dead.
            for reg in &["rdi", "rsi", "rdx", "rcx", "r8", "r9"] {
                self.state.mark_dead(reg);
            }
        }

        // Special handling: CALL clobbers LR (AArch64) or pushes return addr (x86_64)
        if op.kind == crate::asm::microop::OpKind::Call {
            if op.arch == crate::asm::microop::Arch::AArch64 {
                clobbered.push("x30".to_string());
                self.state.mark_live("x30");
            }
        }

        self.clobbers.insert(op.node_id, clobbered);
    }

    /// Process an entire sequence of micro-ops.
    pub fn process_sequence(&mut self, ops: &[MicroOp]) {
        for op in ops {
            self.process_op(op);
        }
    }

    /// Get the set of registers clobbered by a specific node.
    pub fn get_clobbers(&self, node_id: u64) -> &Vec<String> {
        &self.clobbers.get(&node_id).unwrap_or(&Vec::new())
    }

    /// Determine which registers are live-out at the end of processing.
    pub fn live_outs(&self) -> Vec<String> {
        self.state
            .live
            .iter()
            .filter(|(_, &v)| v)
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Check if a specific register is clobbered anywhere in the sequence.
    pub fn is_clobbered_anywhere(&self, reg: &str) -> bool {
        self.clobbers.values().any(|clobbers| clobbers.iter().any(|r| r == reg))
    }
}

impl Default for RegisterTracker {
    fn default() -> Self {
        Self::new()
    }
