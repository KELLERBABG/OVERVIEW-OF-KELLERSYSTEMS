//! EVM bytecode parsing, opcode definitions, and instruction dispatch.
//!
//! Provides a complete model of the Ethereum Virtual Machine instruction set,
//! bytecode parsing into an instruction stream, and the symbolic executor
//! that interprets instructions over symbolic values.

pub mod opcodes;
pub mod parser;
pub mod executor;
pub mod gas;

pub use opcodes::*;
pub use parser::*;
pub use executor::*;