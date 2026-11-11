//! Intermediate Representation (IR) for smart contract bytecode.
//!
//! The IR sits between raw EVM bytecode and the symbolic execution engine.
//! It provides a higher-level view: basic blocks, control flow graphs,
//! function-level entry points, and simplified instruction sequences.
//!
//! This enables more efficient analysis than operating directly on
//! raw bytecode.

use std::collections::{BTreeMap, BTreeSet};

/// A basic block — a linear sequence of instructions with a single entry
/// and single exit (branch or terminal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    /// Unique block identifier.
    pub id: BlockId,
    /// Starting byte offset in the original bytecode.
    pub start_offset: usize,
    /// Ending byte offset (exclusive).
    pub end_offset: usize,
    /// Successor block IDs (jump targets or fall-through).
    pub successors: Vec<BlockId>,
    /// Whether this block is a function entry point (PUSH4 + EQ + JUMPI pattern).
    pub is_function_entry: bool,
}

/// A block identifier.
pub type BlockId = usize;

/// Control flow graph for a contract.
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    /// All basic blocks in the contract, keyed by ID.
    pub blocks: BTreeMap<BlockId, BasicBlock>,
    /// The entry block ID (usually block 0).
    pub entry_block: BlockId,
    /// Reachable blocks from the entry.
    pub reachable: BTreeSet<BlockId>,
    /// Block IDs that contain terminal instructions (STOP, RETURN, REVERT, SELFDESTRUCT).
    pub terminal_blocks: BTreeSet<BlockId>,
}

impl ControlFlowGraph {
    /// Build a CFG from parsed bytecode offsets.
    ///
    /// In a real implementation this would analyze JUMP/JUMPI destinations
    /// to partition the bytecode into basic blocks. For the scaffold, this
    /// creates a simplified linear CFG.
    pub fn from_jump_dests(
        code_size: usize,
        jump_dests: &[usize],
    ) -> Self {
        let mut blocks = BTreeMap::new();
        let mut sorted = jump_dests.to_vec();
        sorted.sort();

        // Always add the code_size as a sentinel if not empty, or just use it
        if code_size > 0 {
            sorted.push(code_size);
        }

        let mut prev = 0usize;
        let mut id_counter = 0usize;

        for &offset in &sorted {
            if offset == prev {
                continue;
            }
            let block = BasicBlock {
                id: id_counter,
                start_offset: prev,
                end_offset: offset,
                successors: Vec::new(),
                is_function_entry: false,
            };
            blocks.insert(id_counter, block);
            id_counter += 1;
            prev = offset;
        }

        // Ensure at least one block exists (even for empty bytecode)
        if blocks.is_empty() {
            blocks.insert(0, BasicBlock {
                id: 0,
                start_offset: 0,
                end_offset: 0,
                successors: Vec::new(),
                is_function_entry: false,
            });
        }

        // Link blocks sequentially (simplified)
        let ids: Vec<_> = blocks.keys().copied().collect();
        for i in 0..ids.len() {
            if let Some(&next_id) = ids.get(i + 1) {
                if let Some(b) = blocks.get_mut(&ids[i]) {
                    b.successors.push(next_id);
                }
            }
        }

        let reachable: BTreeSet<_> = blocks.keys().copied().collect();

        ControlFlowGraph {
            blocks,
            entry_block: 0,
            reachable,
            terminal_blocks: BTreeSet::new(),
        }
    }
}

/// The contract-level intermediate representation.
#[derive(Debug, Clone)]
pub struct ContractIR {
    /// The raw bytecode.
    pub bytecode: Vec<u8>,
    /// The control flow graph.
    pub cfg: ControlFlowGraph,
    /// Function signatures detected (first 4 bytes of Keccak-256 of function sig).
    pub function_selectors: Vec<[u8; 4]>,
    /// Contract metadata (source mapping, compiler version, etc.).
    pub metadata: ContractMetadata,
}

/// Metadata associated with a contract.
#[derive(Debug, Clone, Default)]
pub struct ContractMetadata {
    /// Detected Solidity compiler version.
    pub compiler_version: Option<String>,
    /// Whether the contract has a payable fallback.
    pub has_payable_fallback: bool,
    /// Whether the contract has a receive function.
    pub has_receive: bool,
}

/// Build the IR from parsed bytecode.
pub fn build_ir(bytecode: &[u8]) -> ContractIR {
    let cfg = ControlFlowGraph::from_jump_dests(bytecode.len(), &[]);

    // Detect function selectors from PUSH4 patterns (0x63 + 4 bytes)
    let mut selectors = Vec::new();
    let len = bytecode.len();
    if len >= 5 {
        for i in 0..=len - 5 {
            if bytecode[i] == 0x63 {
                // PUSH4
                let mut sel = [0u8; 4];
                sel.copy_from_slice(&bytecode[i + 1..i + 5]);
                selectors.push(sel);
            }
        }
    }
    selectors.sort();
    selectors.dedup();

    ContractIR {
        bytecode: bytecode.to_vec(),
        cfg,
        function_selectors: selectors,
        metadata: ContractMetadata::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ir_empty() {
        let ir = build_ir(&[]);
        assert!(ir.function_selectors.is_empty());
        // For empty bytecode, CFG has 1 block (the sentinel)
        assert_eq!(ir.cfg.blocks.len(), 1);
    }

    #[test]
    fn test_selector_detection() {
        // PUSH4 + selector bytes
        let mut bytecode = Vec::new();
        bytecode.extend_from_slice(&[0x63, 0xa9, 0x05, 0x9c, 0xbb]); // transfer selector
        let ir = build_ir(&bytecode);
        assert_eq!(ir.function_selectors, vec![[0xa9, 0x05, 0x9c, 0xbb]]);
    }
}