//! Stack pointer alignment and integrity tracking.
//!
//! Tracks the stack pointer (RSP / SP) through the micro-op sequence,
//! verifying that:
//! 1. The stack is always 16-byte aligned at call sites (x86_64 ABI requirement).
//! 2. The stack does not underflow (pop without matching push).
//! 3. The stack frame size is consistent (no unbalanced push/pop).
//! 4. At function entry, RSP is 16-byte aligned (caller guarantee).

use std::collections::HashMap;
use crate::asm::microop::{Arch, MicroOp, OpKind, Operand};

/// A single alignment/integrity violation detected by the stack tracker.
#[derive(Debug, Clone)]
pub struct StackViolation {
    /// The node_id of the micro-op where the violation was detected.
    pub node_id: u64,
    /// Byte offset in the binary.
    pub offset: usize,
    /// Description of the violation.
    pub description: String,
    /// Severity level.
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViolationSeverity {
    /// Informational - may be fine in context.
    Info,
    /// Warning - likely a bug but could be intentional.
    Warning,
    /// Error - definite ABI violation or memory corruption risk.
    Error,
}

/// Tracks stack pointer state through the micro-op sequence.
pub struct StackTracker {
    /// Current stack offset relative to function entry (positive = deeper).
    pub current_sp_offset: i64,
    /// The initial SP alignment assumption (16 for x86_64, 16 for AArch64).
    pub initial_alignment: u32,
    /// Detected violations.
    pub violations: Vec<StackViolation>,
    /// Maximum stack depth reached (in bytes).
    pub max_depth: i64,
    /// Minimum stack offset (should be 0 or negative if we track upward).
    pub min_offset: i64,
    /// Per-node SP offset for post-hoc analysis.
    pub sp_at_node: HashMap<u64, i64>,
}

impl StackTracker {
    pub fn new(arch: Arch) -> Self {
        let alignment = match arch {
            Arch::X86_64 => 16,
            Arch::AArch64 => 16,
        };
        Self {
            current_sp_offset: 0,
            initial_alignment: alignment,
            violations: Vec::new(),
            max_depth: 0,
            min_offset: 0,
            sp_at_node: HashMap::new(),
        }
    }

    /// Process a single micro-op, updating SP tracking.
    pub fn process_op(&mut self, op: &MicroOp) {
        let delta = self.sp_delta(op);
        self.current_sp_offset += delta;

        if self.current_sp_offset > self.max_depth {
            self.max_depth = self.current_sp_offset;
        }
        if self.current_sp_offset < self.min_offset {
            self.min_offset = self.current_sp_offset;
        }

        // Check for underflow (SP going above entry point)
        if self.current_sp_offset < 0 {
            self.violations.push(StackViolation {
                node_id: op.node_id,
                offset: op.offset,
                description: format!(
                    "Stack underflow: SP is {} bytes above function entry at offset 0x{:x}",
                    -self.current_sp_offset, op.offset
                ),
                severity: ViolationSeverity::Error,
            });
        }

        // Check alignment at CALL sites (x86_64 requires 16-byte aligned SP before CALL)
        if op.kind == OpKind::Call && self.initial_alignment == 16 {
            // At the point of CALL, SP must be aligned to 16 bytes.
            // The CALL itself pushes 8 bytes (return address), so BEFORE the call,
            // SP should be at a 16-byte boundary.
            let sp_before_call = self.current_sp_offset;
            if sp_before_call % 16 != 0 {
                self.violations.push(StackViolation {
                    node_id: op.node_id,
                    offset: op.offset,
                    description: format!(
                        "Stack misalignment at CALL: SP offset {} is not 16-byte aligned (offset 0x{:x})",
                        sp_before_call, op.offset
                    ),
                    severity: ViolationSeverity::Error,
                });
            }
        }

        // Check alignment at RET (SP should be back to entry alignment)
        if op.kind == OpKind::Ret {
            if self.current_sp_offset != 0 {
                self.violations.push(StackViolation {
                    node_id: op.node_id,
                    offset: op.offset,
                    description: format!(
                        "Unbalanced stack at RET: SP offset is {} (expected 0) at offset 0x{:x}",
                        self.current_sp_offset, op.offset
                    ),
                    severity: ViolationSeverity::Error,
                });
            }
        }

        self.sp_at_node.insert(op.node_id, self.current_sp_offset);
    }

    /// Process an entire sequence.
    pub fn process_sequence(&mut self, ops: &[MicroOp]) {
        for op in ops {
            self.process_op(op);
        }
    }

    /// Compute the SP delta for a micro-op.
    fn sp_delta(&self, op: &MicroOp) -> i64 {
        match op.kind {
            OpKind::Push => -8, // x86_64: push decrements SP by 8
            OpKind::Pop => 8,   // x86_64: pop increments SP by 8
            OpKind::Call => -8, // CALL pushes return address (implicit)
            OpKind::Ret => 8,   // RET pops return address (implicit)
            OpKind::Add | OpKind::Sub => {
                // Check if SP is the destination or source with an immediate
                self.sp_arith_delta(op)
            }
            OpKind::Leave => {
                // LEAVE = mov rsp, rbp; pop rbp => net effect depends on frame size.
                // We can't know the exact value here; return 0 and rely on
                // the explicit push/pop tracking.
                0
            }
            _ => 0,
        }
    }

    /// For ADD/SUB involving SP, try to determine the immediate delta.
    fn sp_arith_delta(&self, op: &MicroOp) -> i64 {
        // Check if dest is SP and source is an immediate
        if let Some(Operand::Reg(ref name)) = op.dest {
            if name == "rsp" || name == "sp" {
                for src in &op.sources {
                    if let Operand::Imm(val) = src {
                        return *val;
                    }
                }
            }
        }
        // Check if source is SP and dest is an immediate (SUB imm, rsp pattern)
        if let Some(Operand::Reg(ref name)) = op.dest {
            if name == "rsp" || name == "sp" {
                for src in &op.sources {
                    if let Operand::Imm(val) = src {
                        if op.kind == OpKind::Sub {
                            return -*val;
                        }
                    }
                }
            }
        }
        0
    }

    /// Verify that the total stack usage is balanced (net zero at RET).
    pub fn verify_balance(&self) -> bool {
        self.current_sp_offset == 0
    }

    /// Get the maximum stack frame size in bytes.
    pub fn frame_size(&self) -> i64 {
        self.max_depth
    }

    /// Check if any violation is at Error severity.
    pub fn has_errors(&self) -> bool {
        self.violations.iter().any(|v| v.severity == ViolationSeverity::Error)
    }
}
