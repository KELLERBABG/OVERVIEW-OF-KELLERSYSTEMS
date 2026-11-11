//! EVM bytecode parser — converts raw bytecode into a stream of instructions
//! with resolved push data and valid jump destinations.

use crate::evm::opcodes::{Instruction, INSTRUCTION_TABLE};
use crate::types::ExecutionError;

/// A parsed instruction with its associated immediate data (push bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedInstruction {
    /// The instruction definition.
    pub instruction: &'static Instruction,
    /// The byte offset of this instruction in the bytecode.
    pub offset: usize,
    /// Immediate data for PUSH instructions (big-endian, right-padded to 32 bytes).
    pub push_data: Option<Vec<u8>>,
}

/// Parsed bytecode with pre-computed jump destinations.
#[derive(Debug, Clone)]
pub struct ParsedBytecode {
    /// Raw bytecode bytes.
    pub bytecode: Vec<u8>,
    /// Parsed instruction sequence.
    pub instructions: Vec<ParsedInstruction>,
    /// Set of valid JUMPDEST offsets.
    pub jump_dests: Vec<usize>,
}

impl ParsedBytecode {
    /// Parse raw bytecode into an instruction stream.
    pub fn parse(bytecode: &[u8]) -> Result<Self, ExecutionError> {
        let mut instructions = Vec::new();
        let mut jump_dests = Vec::new();
        let mut offset = 0;

        while offset < bytecode.len() {
            let opcode = bytecode[offset];
            let instr = Instruction::from_opcode(opcode)
                .ok_or(ExecutionError::InvalidOpcode(opcode))?;

            let push_data = if (0x60..=0x7f).contains(&opcode) {
                // PUSH1..PUSH32 — read immediate bytes
                let n = (opcode - 0x5f) as usize; // PUSH1=1, PUSH32=32
                let start = offset + 1;
                let end = (start + n).min(bytecode.len());
                let data = bytecode[start..end].to_vec();
                Some(data)
            } else {
                None
            };

            let parsed = ParsedInstruction {
                instruction: instr,
                offset,
                push_data,
            };

            if instr.is_jumpdest {
                jump_dests.push(offset);
            }

            instructions.push(parsed);

            // Advance: 1 byte opcode + push data length
            offset += 1 + match instr.opcode {
                0x60..=0x7f => (instr.opcode - 0x5f) as usize,
                _ => 0,
            };
        }

        Ok(Self {
            bytecode: bytecode.to_vec(),
            instructions,
            jump_dests,
        })
    }

    /// Check if a given offset is a valid JUMPDEST.
    pub fn is_valid_jumpdest(&self, offset: usize) -> bool {
        self.jump_dests.binary_search(&offset).is_ok()
    }

    /// Get the instruction at a given offset (must be instruction-aligned).
    pub fn instruction_at(&self, offset: usize) -> Option<&ParsedInstruction> {
        self.instructions.iter().find(|i| i.offset == offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let parsed = ParsedBytecode::parse(&[]).unwrap();
        assert!(parsed.instructions.is_empty());
        assert!(parsed.jump_dests.is_empty());
    }

    #[test]
    fn test_parse_push_and_stop() {
        // PUSH1 0x42 STOP
        let bytecode = vec![0x60, 0x42, 0x00];
        let parsed = ParsedBytecode::parse(&bytecode).unwrap();
        assert_eq!(parsed.instructions.len(), 2);
        assert_eq!(parsed.instructions[0].instruction.name, "PUSH1");
        assert_eq!(parsed.instructions[0].push_data, Some(vec![0x42]));
        assert_eq!(parsed.instructions[1].instruction.name, "STOP");
    }

    #[test]
    fn test_jumpdest_detection() {
        // JUMPDEST PUSH1 0x00 JUMPDEST
        let bytecode = vec![0x5b, 0x60, 0x00, 0x5b];
        let parsed = ParsedBytecode::parse(&bytecode).unwrap();
        assert_eq!(parsed.jump_dests, vec![0, 3]);
    }
}