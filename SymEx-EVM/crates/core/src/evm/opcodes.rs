//! EVM opcode definitions with metadata (name, gas cost, stack inputs/outputs).
//!
//! This file covers all EVM opcodes from the Frontier through Shanghai hard forks.

use std::fmt;

/// A single EVM instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instruction {
    /// The raw opcode byte.
    pub opcode: u8,
    /// Mnemonic name.
    pub name: &'static str,
    /// Number of stack items this instruction pops.
    pub pops: u8,
    /// Number of stack items this instruction pushes.
    pub pushes: u8,
    /// Base gas cost.
    pub gas_cost: u64,
    /// Whether this instruction terminates the execution frame.
    pub is_terminal: bool,
    /// Whether this instruction is a JUMPDEST (valid jump target).
    pub is_jumpdest: bool,
}

impl Instruction {
    /// Look up an instruction by opcode byte.
    pub fn from_opcode(opcode: u8) -> Option<&'static Instruction> {
        INSTRUCTION_TABLE.get(opcode as usize)
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02x} {}", self.opcode, self.name)
    }
}

// ---------------------------------------------------------------------------
// Instruction table (256 entries, indexed by opcode)
// ---------------------------------------------------------------------------

/// Static instruction table, indexed by opcode byte value.
/// Entries for undefined opcodes are the `STOP` sentinel with name "INVALID".
pub static INSTRUCTION_TABLE: [Instruction; 256] = make_table();

const fn make_table() -> [Instruction; 256] {
    let mut table = [Instruction {
        opcode: 0x00,
        name: "STOP",
        pops: 0,
        pushes: 0,
        gas_cost: 0,
        is_terminal: true,
        is_jumpdest: false,
    }; 256];

    // Mark all non-zero entries as invalid by default; we'll fill real opcodes.
    let mut i = 1u8;
    loop {
        table[i as usize] = Instruction {
            opcode: i,
            name: "INVALID",
            pops: 0,
            pushes: 0,
            gas_cost: 0,
            is_terminal: true,
            is_jumpdest: false,
        };
        if i == 255 {
            break;
        }
        i += 1;
    }

    // ---- Arithmetic ----
    table[0x01] = instr(0x01, "ADD", 2, 1, 3);
    table[0x02] = instr(0x02, "MUL", 2, 1, 5);
    table[0x03] = instr(0x03, "SUB", 2, 1, 3);
    table[0x04] = instr(0x04, "DIV", 2, 1, 5);
    table[0x05] = instr(0x05, "SDIV", 2, 1, 5);
    table[0x06] = instr(0x06, "MOD", 2, 1, 5);
    table[0x07] = instr(0x07, "SMOD", 2, 1, 5);
    table[0x08] = instr(0x08, "ADDMOD", 3, 1, 8);
    table[0x09] = instr(0x09, "MULMOD", 3, 1, 8);
    table[0x0a] = instr(0x0a, "EXP", 2, 1, 10);
    table[0x0b] = instr(0x0b, "SIGNEXTEND", 2, 1, 5);

    // ---- Comparison & Bitwise ----
    table[0x10] = instr(0x10, "LT", 2, 1, 3);
    table[0x11] = instr(0x11, "GT", 2, 1, 3);
    table[0x12] = instr(0x12, "SLT", 2, 1, 3);
    table[0x13] = instr(0x13, "SGT", 2, 1, 3);
    table[0x14] = instr(0x14, "EQ", 2, 1, 3);
    table[0x15] = instr(0x15, "ISZERO", 1, 1, 3);
    table[0x16] = instr(0x16, "AND", 2, 1, 3);
    table[0x17] = instr(0x17, "OR", 2, 1, 3);
    table[0x18] = instr(0x18, "XOR", 2, 1, 3);
    table[0x19] = instr(0x19, "NOT", 1, 1, 3);
    table[0x1a] = instr(0x1a, "BYTE", 2, 1, 3);
    table[0x1b] = instr(0x1b, "SHL", 2, 1, 3);
    table[0x1c] = instr(0x1c, "SHR", 2, 1, 3);
    table[0x1d] = instr(0x1d, "SAR", 2, 1, 3);

    // ---- Keccak256 ----
    table[0x20] = instr(0x20, "KECCAK256", 2, 1, 30);

    // ---- Environment ----
    table[0x30] = instr(0x30, "ADDRESS", 0, 1, 2);
    table[0x31] = instr(0x31, "BALANCE", 1, 1, 100);
    table[0x32] = instr(0x32, "ORIGIN", 0, 1, 2);
    table[0x33] = instr(0x33, "CALLER", 0, 1, 2);
    table[0x34] = instr(0x34, "CALLVALUE", 0, 1, 2);
    table[0x35] = instr(0x35, "CALLDATALOAD", 1, 1, 3);
    table[0x36] = instr(0x36, "CALLDATASIZE", 0, 1, 2);
    table[0x37] = instr(0x37, "CALLDATACOPY", 3, 0, 3);
    table[0x38] = instr(0x38, "CODESIZE", 0, 1, 2);
    table[0x39] = instr(0x39, "CODECOPY", 3, 0, 3);
    table[0x3a] = instr(0x3a, "GASPRICE", 0, 1, 2);
    table[0x3b] = instr(0x3b, "EXTCODESIZE", 1, 1, 100);
    table[0x3c] = instr(0x3c, "EXTCODECOPY", 4, 0, 100);
    table[0x3d] = instr(0x3d, "RETURNDATASIZE", 0, 1, 2);
    table[0x3e] = instr(0x3e, "RETURNDATACOPY", 3, 0, 3);
    table[0x3f] = instr(0x3f, "EXTCODEHASH", 1, 1, 100);

    // ---- Block Information ----
    table[0x40] = instr(0x40, "BLOCKHASH", 1, 1, 20);
    table[0x41] = instr(0x41, "COINBASE", 0, 1, 2);
    table[0x42] = instr(0x42, "TIMESTAMP", 0, 1, 2);
    table[0x43] = instr(0x43, "NUMBER", 0, 1, 2);
    table[0x44] = instr(0x44, "PREVRANDAO", 0, 1, 2);
    table[0x45] = instr(0x45, "GASLIMIT", 0, 1, 2);
    table[0x46] = instr(0x46, "CHAINID", 0, 1, 2);
    table[0x47] = instr(0x47, "SELFBALANCE", 0, 1, 5);
    table[0x48] = instr(0x48, "BASEFEE", 0, 1, 2);

    // ---- Memory & Stack ----
    table[0x50] = instr(0x50, "POP", 1, 0, 2);
    table[0x51] = instr(0x51, "MLOAD", 1, 1, 3);
    table[0x52] = instr(0x52, "MSTORE", 2, 0, 3);
    table[0x53] = instr(0x53, "MSTORE8", 2, 0, 3);
    table[0x54] = instr(0x54, "SLOAD", 1, 1, 100);
    table[0x55] = instr(0x55, "SSTORE", 2, 0, 100);
    table[0x56] = instr(0x56, "JUMP", 1, 0, 8);
    table[0x57] = instr(0x57, "JUMPI", 2, 0, 10);
    table[0x58] = instr(0x58, "PC", 0, 1, 2);
    table[0x59] = instr(0x59, "MSIZE", 0, 1, 2);
    table[0x5a] = instr(0x5a, "GAS", 0, 1, 2);
    table[0x5b] = instr_jd(0x5b, "JUMPDEST", 0, 0, 1);

    // ---- Push Operations ----
    table[0x60] = instr(0x60, "PUSH1", 0, 1, 3);
    table[0x61] = instr(0x61, "PUSH2", 0, 1, 3);
    table[0x62] = instr(0x62, "PUSH3", 0, 1, 3);
    table[0x63] = instr(0x63, "PUSH4", 0, 1, 3);
    table[0x64] = instr(0x64, "PUSH5", 0, 1, 3);
    table[0x65] = instr(0x65, "PUSH6", 0, 1, 3);
    table[0x66] = instr(0x66, "PUSH7", 0, 1, 3);
    table[0x67] = instr(0x67, "PUSH8", 0, 1, 3);
    table[0x68] = instr(0x68, "PUSH9", 0, 1, 3);
    table[0x69] = instr(0x69, "PUSH10", 0, 1, 3);
    table[0x6a] = instr(0x6a, "PUSH11", 0, 1, 3);
    table[0x6b] = instr(0x6b, "PUSH12", 0, 1, 3);
    table[0x6c] = instr(0x6c, "PUSH13", 0, 1, 3);
    table[0x6d] = instr(0x6d, "PUSH14", 0, 1, 3);
    table[0x6e] = instr(0x6e, "PUSH15", 0, 1, 3);
    table[0x6f] = instr(0x6f, "PUSH16", 0, 1, 3);
    table[0x70] = instr(0x70, "PUSH17", 0, 1, 3);
    table[0x71] = instr(0x71, "PUSH18", 0, 1, 3);
    table[0x72] = instr(0x72, "PUSH19", 0, 1, 3);
    table[0x73] = instr(0x73, "PUSH20", 0, 1, 3);
    table[0x74] = instr(0x74, "PUSH21", 0, 1, 3);
    table[0x75] = instr(0x75, "PUSH22", 0, 1, 3);
    table[0x76] = instr(0x76, "PUSH23", 0, 1, 3);
    table[0x77] = instr(0x77, "PUSH24", 0, 1, 3);
    table[0x78] = instr(0x78, "PUSH25", 0, 1, 3);
    table[0x79] = instr(0x79, "PUSH26", 0, 1, 3);
    table[0x7a] = instr(0x7a, "PUSH27", 0, 1, 3);
    table[0x7b] = instr(0x7b, "PUSH28", 0, 1, 3);
    table[0x7c] = instr(0x7c, "PUSH29", 0, 1, 3);
    table[0x7d] = instr(0x7d, "PUSH30", 0, 1, 3);
    table[0x7e] = instr(0x7e, "PUSH31", 0, 1, 3);
    table[0x7f] = instr(0x7f, "PUSH32", 0, 1, 3);

    // ---- Dup Operations ----
    table[0x80] = instr(0x80, "DUP1", 1, 2, 3);
    table[0x81] = instr(0x81, "DUP2", 2, 3, 3);
    table[0x82] = instr(0x82, "DUP3", 3, 4, 3);
    table[0x83] = instr(0x83, "DUP4", 4, 5, 3);
    table[0x84] = instr(0x84, "DUP5", 5, 6, 3);
    table[0x85] = instr(0x85, "DUP6", 6, 7, 3);
    table[0x86] = instr(0x86, "DUP7", 7, 8, 3);
    table[0x87] = instr(0x87, "DUP8", 8, 9, 3);
    table[0x88] = instr(0x88, "DUP9", 9, 10, 3);
    table[0x89] = instr(0x89, "DUP10", 10, 11, 3);
    table[0x8a] = instr(0x8a, "DUP11", 11, 12, 3);
    table[0x8b] = instr(0x8b, "DUP12", 12, 13, 3);
    table[0x8c] = instr(0x8c, "DUP13", 13, 14, 3);
    table[0x8d] = instr(0x8d, "DUP14", 14, 15, 3);
    table[0x8e] = instr(0x8e, "DUP15", 15, 16, 3);
    table[0x8f] = instr(0x8f, "DUP16", 16, 17, 3);

    // ---- Swap Operations ----
    table[0x90] = instr(0x90, "SWAP1", 2, 2, 3);
    table[0x91] = instr(0x91, "SWAP2", 3, 3, 3);
    table[0x92] = instr(0x92, "SWAP3", 4, 4, 3);
    table[0x93] = instr(0x93, "SWAP4", 5, 5, 3);
    table[0x94] = instr(0x94, "SWAP5", 6, 6, 3);
    table[0x95] = instr(0x95, "SWAP6", 7, 7, 3);
    table[0x96] = instr(0x96, "SWAP7", 8, 8, 3);
    table[0x97] = instr(0x97, "SWAP8", 9, 9, 3);
    table[0x98] = instr(0x98, "SWAP9", 10, 10, 3);
    table[0x99] = instr(0x99, "SWAP10", 11, 11, 3);
    table[0x9a] = instr(0x9a, "SWAP11", 12, 12, 3);
    table[0x9b] = instr(0x9b, "SWAP12", 13, 13, 3);
    table[0x9c] = instr(0x9c, "SWAP13", 14, 14, 3);
    table[0x9d] = instr(0x9d, "SWAP14", 15, 15, 3);
    table[0x9e] = instr(0x9e, "SWAP15", 16, 16, 3);
    table[0x9f] = instr(0x9f, "SWAP16", 17, 17, 3);

    // ---- Logging ----
    table[0xa0] = instr(0xa0, "LOG0", 2, 0, 100);
    table[0xa1] = instr(0xa1, "LOG1", 3, 0, 100);
    table[0xa2] = instr(0xa2, "LOG2", 4, 0, 100);
    table[0xa3] = instr(0xa3, "LOG3", 5, 0, 100);
    table[0xa4] = instr(0xa4, "LOG4", 6, 0, 100);

    // ---- Calls ----
    table[0xf0] = instr(0xf0, "CREATE", 3, 1, 32000);
    table[0xf1] = instr(0xf1, "CALL", 7, 1, 100);
    table[0xf2] = instr(0xf2, "CALLCODE", 7, 1, 100);
    table[0xf3] = instr_term(0xf3, "RETURN", 2, 0, 0);
    table[0xf4] = instr(0xf4, "DELEGATECALL", 6, 1, 100);
    table[0xf5] = instr(0xf5, "CREATE2", 4, 1, 32000);
    table[0xfa] = instr(0xfa, "STATICCALL", 6, 1, 100);
    table[0xfd] = instr_term(0xfd, "REVERT", 2, 0, 0);
    table[0xfe] = instr(0xfe, "INVALID", 0, 0, 0);
    table[0xff] = instr_term(0xff, "SELFDESTRUCT", 1, 0, 5000);

    table
}

/// Helper to build a non-terminal instruction.
const fn instr(opcode: u8, name: &'static str, pops: u8, pushes: u8, gas_cost: u64) -> Instruction {
    Instruction {
        opcode,
        name,
        pops,
        pushes,
        gas_cost,
        is_terminal: false,
        is_jumpdest: false,
    }
}

/// Helper to build a terminal instruction (STOP, RETURN, REVERT, SELFDESTRUCT).
const fn instr_term(
    opcode: u8,
    name: &'static str,
    pops: u8,
    pushes: u8,
    gas_cost: u64,
) -> Instruction {
    Instruction {
        opcode,
        name,
        pops,
        pushes,
        gas_cost,
        is_terminal: true,
        is_jumpdest: false,
    }
}

/// Helper to build JUMPDEST.
const fn instr_jd(
    opcode: u8,
    name: &'static str,
    pops: u8,
    pushes: u8,
    gas_cost: u64,
) -> Instruction {
    Instruction {
        opcode,
        name,
        pops,
        pushes,
        gas_cost,
        is_terminal: false,
        is_jumpdest: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_opcodes() {
        assert_eq!(Instruction::from_opcode(0x01).unwrap().name, "ADD");
        assert_eq!(Instruction::from_opcode(0x55).unwrap().name, "SSTORE");
        assert_eq!(Instruction::from_opcode(0xff).unwrap().name, "SELFDESTRUCT");
    }

    #[test]
    fn test_invalid_opcode() {
        let inst = Instruction::from_opcode(0x0c).unwrap();
        assert_eq!(inst.name, "INVALID");
        assert!(inst.is_terminal);
    }

    #[test]
    fn test_jumpdest() {
        assert!(Instruction::from_opcode(0x5b).unwrap().is_jumpdest);
        assert!(!Instruction::from_opcode(0x01).unwrap().is_jumpdest);
    }
}