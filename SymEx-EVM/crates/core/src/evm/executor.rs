//! Symbolic EVM executor — interprets instructions over symbolic values,
//! building path constraints and updating the symbolic state.

use crate::evm::parser::ParsedBytecode;
use crate::types::{
    ExecutionError, ExecutionState, ExecutionStatus, SymExpr, SymbolicValue, U256,
};

/// Result of executing a single instruction.
#[derive(Debug, Clone)]
pub struct StepResult {
    /// The updated execution state.
    pub state: ExecutionState,
    /// Path constraints collected (for JUMPI branches).
    pub branch_constraints: Option<BranchConstraints>,
}

/// Constraints emitted when encountering a conditional jump.
#[derive(Debug, Clone)]
pub struct BranchConstraints {
    /// The taken-branch constraint (condition holds).
    pub taken: SymbolicValue,
    /// The not-taken-branch constraint (condition fails).
    pub not_taken: SymbolicValue,
    /// Offset of the JUMPI instruction.
    pub pc: usize,
}

/// The symbolic EVM executor.
pub struct SymbolicExecutor {
    /// Parsed bytecode being executed.
    bytecode: ParsedBytecode,
    /// Fresh symbolic variable counter.
    sym_counter: u64,
}

impl SymbolicExecutor {
    /// Create a new executor for the given bytecode.
    pub fn new(bytecode: ParsedBytecode) -> Self {
        Self {
            bytecode,
            sym_counter: 0,
        }
    }

    /// Allocate a fresh symbolic variable identifier.
    pub fn fresh_sym(&mut self) -> SymbolicValue {
        let id = self.sym_counter;
        self.sym_counter += 1;
        SymbolicValue::fresh(crate::types::SymVarId(id))
    }

    /// Get a reference to the parsed bytecode.
    pub fn parsed_bytecode(&self) -> &ParsedBytecode {
        &self.bytecode
    }

    /// Execute a single instruction at the current program counter.
    /// Returns the step result, or an error if execution fails.
    pub fn step(&mut self, state: &mut ExecutionState) -> Result<StepResult, ExecutionError> {
        let instruction = self
            .bytecode
            .instruction_at(state.pc)
            .ok_or(ExecutionError::Internal(format!(
                "no instruction at pc {}",
                state.pc
            )))?;

        // --- Gas deduction ---
        let gas_cost = instruction.instruction.gas_cost;
        if state.gas < gas_cost {
            state.status = ExecutionStatus::OutOfGas;
            return Ok(StepResult {
                state: state.clone(),
                branch_constraints: None,
            });
        }
        state.gas -= gas_cost;

        match instruction.instruction.opcode {
            // ---- STOP ----
            0x00 => {
                state.status = ExecutionStatus::Halted;
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }

            // ---- Arithmetic ----
            0x01 => self.binary_op(state, |a, b| SymExpr::Add(a, b)),
            0x02 => self.binary_op(state, |a, b| SymExpr::Mul(a, b)),
            0x03 => self.binary_op(state, |a, b| SymExpr::Sub(a, b)),
            0x04 => self.binary_op(state, |a, b| SymExpr::Div(a, b)),
            0x06 => self.binary_op(state, |a, b| SymExpr::Mod(a, b)),

            // ---- Comparison ----
            0x10 => self.binary_op(state, |a, b| SymExpr::Lt(a, b)),
            0x11 => self.binary_op(state, |a, b| SymExpr::Gt(a, b)),
            0x14 => self.binary_op(state, |a, b| SymExpr::Eq(a, b)),
            0x15 => {
                let a = state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;
                let zero = SymbolicValue::Concrete(U256::ZERO);
                state.stack.push(SymExpr::Eq(a, zero).into());
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }
            0x16 => self.binary_op(state, |a, b| SymExpr::And(a, b)),
            0x17 => self.binary_op(state, |a, b| SymExpr::Or(a, b)),
            0x18 => self.binary_op(state, |a, b| SymExpr::Xor(a, b)),
            0x19 => {
                let a = state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;
                state.stack.push(SymExpr::Not(a).into());
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }

            // ---- Environment ----
            0x34 => {
                // CALLVALUE
                state.stack.push(state.callvalue.clone());
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }
            0x35 => {
                // CALLDATALOAD
                let offset = state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;
                state.stack.push(SymExpr::CalldataLoad(offset).into());
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }
            0x36 => {
                // CALLDATASIZE
                let size = SymbolicValue::Concrete(U256::from(state.calldata.len() as u64));
                state.stack.push(size);
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }

            // ---- Memory ----
            0x51 => {
                // MLOAD
                let addr = state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;
                let value = match &addr {
                    SymbolicValue::Concrete(offset) => {
                        let offset_usize = offset.to_be_bytes();
                        let off = u64::from_be_bytes([
                            offset_usize[24],
                            offset_usize[25],
                            offset_usize[26],
                            offset_usize[27],
                            offset_usize[28],
                            offset_usize[29],
                            offset_usize[30],
                            offset_usize[31],
                        ]) as usize;
                        state
                            .memory
                            .get(&off)
                            .cloned()
                            .unwrap_or(SymbolicValue::Concrete(U256::ZERO))
                    }
                    _ => SymbolicValue::Concrete(U256::ZERO),
                };
                state.stack.push(value);
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }
            0x52 => {
                // MSTORE
                let addr = state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;
                let value = state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;
                if let SymbolicValue::Concrete(offset) = &addr {
                    let offset_usize = offset.to_be_bytes();
                    let off = u64::from_be_bytes([
                        offset_usize[24],
                        offset_usize[25],
                        offset_usize[26],
                        offset_usize[27],
                        offset_usize[28],
                        offset_usize[29],
                        offset_usize[30],
                        offset_usize[31],
                    ]) as usize;
                    state.memory.insert(off, value);
                }
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }

            // ---- Storage ----
            0x54 => {
                // SLOAD
                let key = state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;
                let value = state
                    .storage
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| v.clone())
                    .unwrap_or(SymbolicValue::Concrete(U256::ZERO));
                state.stack.push(value);
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }
            0x55 => {
                // SSTORE
                let key = state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;
                let value = state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;
                // Check if key already exists, update it; otherwise push
                if let Some(pos) = state.storage.iter().position(|(k, _)| *k == key) {
                    state.storage[pos] = (key, value);
                } else {
                    state.storage.push((key, value));
                }
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }

            // ---- JUMP / JUMPI ----
            0x56 => {
                // JUMP
                let target = state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;
                let offset = self.resolve_jump_target(&target)?;
                if !self.bytecode.is_valid_jumpdest(offset) {
                    return Err(ExecutionError::InvalidJumpDestination(offset as u16));
                }
                state.pc = offset;
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }
            0x57 => {
                // JUMPI
                let target = state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;
                let condition = state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;

                let offset = self.resolve_jump_target(&target)?;

                // Generate branch constraints
                let taken = condition.clone();
                let zero = SymbolicValue::Concrete(U256::ZERO);
                let not_taken = SymExpr::Eq(condition, zero).into();

                if self.bytecode.is_valid_jumpdest(offset) {
                    state.pc = offset;
                } else {
                    // Fall through (not-taken)
                    state.pc += 1;
                }

                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: Some(BranchConstraints {
                        taken,
                        not_taken,
                        pc: instruction.offset,
                    }),
                })
            }

            // ---- Push operations ----
            0x60..=0x7f => {
                if let Some(data) = &instruction.push_data {
                    let mut buf = [0u8; 32];
                    buf[32 - data.len()..].copy_from_slice(data);
                    let val = U256::from_be_bytes(&buf);
                    state.stack.push(SymbolicValue::Concrete(val));
                }
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }

            // ---- Duplication ----
            0x80..=0x8f => {
                let depth = (instruction.instruction.opcode - 0x7f) as usize;
                let idx = state.stack.len().checked_sub(depth).ok_or(ExecutionError::StackUnderflow)?;
                let val = state.stack[idx].clone();
                if state.stack.len() >= 1024 {
                    return Err(ExecutionError::StackOverflow);
                }
                state.stack.push(val);
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }

            // ---- Swap ----
            0x90..=0x9f => {
                let depth = (instruction.instruction.opcode - 0x8f) as usize;
                let len = state.stack.len();
                let top = len - 1;
                let idx = len.checked_sub(depth).ok_or(ExecutionError::StackUnderflow)?;
                state.stack.swap(top, idx);
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }

            // ---- Pop ----
            0x50 => {
                state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }

            // ---- Return / Revert ----
            0xf3 | 0xfd => {
                state.status = ExecutionStatus::Halted;
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }

            // ---- INVALID ----
            0xfe => {
                state.status = ExecutionStatus::InvalidInstruction;
                Ok(StepResult {
                    state: state.clone(),
                    branch_constraints: None,
                })
            }

            // ---- Unsupported ----
            _ => Err(ExecutionError::Unsupported(format!(
                "opcode 0x{:02x} ({})",
                instruction.instruction.opcode, instruction.instruction.name
            ))),
        }
    }

    // --- Helpers ---

    /// Execute a binary operation on the top two stack items.
    fn binary_op(
        &mut self,
        state: &mut ExecutionState,
        op: impl FnOnce(SymbolicValue, SymbolicValue) -> SymExpr,
    ) -> Result<StepResult, ExecutionError> {
        let b = state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;
        let a = state.stack.pop().ok_or(ExecutionError::StackUnderflow)?;
        state.stack.push(op(a, b).into());
        Ok(StepResult {
            state: state.clone(),
            branch_constraints: None,
        })
    }

    /// Resolve a jump target from a symbolic value.
    fn resolve_jump_target(&self, target: &SymbolicValue) -> Result<usize, ExecutionError> {
        match target {
            SymbolicValue::Concrete(addr) => {
                let bytes = addr.to_be_bytes();
                let off = u64::from_be_bytes([
                    bytes[24], bytes[25], bytes[26], bytes[27],
                    bytes[28], bytes[29], bytes[30], bytes[31],
                ]) as usize;
                Ok(off)
            }
            _ => Err(ExecutionError::Unsupported(
                "non-concrete jump target".to_string(),
            )),
        }
    }
}

impl From<SymExpr> for SymbolicValue {
    fn from(expr: SymExpr) -> Self {
        SymbolicValue::Expression(Box::new(expr))
    }
}