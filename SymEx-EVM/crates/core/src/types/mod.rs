//! Core domain types for the symbolic execution engine.
//!
//! This module defines the foundational types used throughout the engine:
//! addresses, storage keys, symbolic variables, concrete and symbolic values,
//! execution status, and error types.

use std::collections::BTreeMap;
use std::fmt;

// ---------------------------------------------------------------------------
// Re-exports
// ---------------------------------------------------------------------------

pub use value::*;
pub use error::*;
pub use address::*;
pub use state::*;

// ---------------------------------------------------------------------------
// Addresses & Identifiers
// ---------------------------------------------------------------------------

mod address {
    use std::fmt;

    /// A 20-byte Ethereum address.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Address([u8; 20]);

    impl Address {
        pub const fn new(bytes: [u8; 20]) -> Self {
            Self(bytes)
        }

        pub const fn zero() -> Self {
            Self([0u8; 20])
        }

        pub fn as_bytes(&self) -> &[u8; 20] {
            &self.0
        }
    }

    impl From<[u8; 20]> for Address {
        fn from(bytes: [u8; 20]) -> Self {
            Self(bytes)
        }
    }

    impl fmt::LowerHex for Address {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for byte in &self.0 {
                write!(f, "{byte:02x}")?;
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Values
// ---------------------------------------------------------------------------

mod value {
    use std::fmt;
    use std::num::ParseIntError;

    /// A 256-bit unsigned integer (EVM word).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    pub struct U256([u64; 4]);

    impl U256 {
        pub const ZERO: Self = Self([0, 0, 0, 0]);
        pub const ONE: Self = Self([1, 0, 0, 0]);

        pub fn from_limbs(limbs: [u64; 4]) -> Self {
            Self(limbs)
        }

        pub fn limbs(&self) -> &[u64; 4] {
            &self.0
        }
    }

    impl From<u64> for U256 {
        fn from(v: u64) -> Self {
            Self([v, 0, 0, 0])
        }
    }

    impl fmt::LowerHex for U256 {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            // Print as big-endian hex padded to 64 nibbles.
            let be_bytes = self.to_be_bytes();
            for byte in &be_bytes {
                write!(f, "{byte:02x}")?;
            }
            Ok(())
        }
    }

    impl U256 {
        /// Convert to big-endian byte array.
        pub fn to_be_bytes(&self) -> [u8; 32] {
            let mut buf = [0u8; 32];
            for i in 0..4 {
                buf[24 - i * 8..32 - i * 8].copy_from_slice(&self.0[i].to_be_bytes());
            }
            buf
        }

        /// Convert from big-endian byte array.
        pub fn from_be_bytes(bytes: &[u8; 32]) -> Self {
            let mut limbs = [0u64; 4];
            for i in 0..4 {
                let start = 24 - i * 8;
                let end = 32 - i * 8;
                let arr: [u8; 8] = bytes[start..end].try_into().unwrap();
                limbs[i] = u64::from_be_bytes(arr);
            }
            Self(limbs)
        }
    }

    /// A symbolic variable identifier.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct SymVarId(pub u64);

    impl fmt::Display for SymVarId {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "sym_{}", self.0)
        }
    }

    /// A value in the symbolic execution engine.
    ///
    /// A value can be a concrete unsigned integer, a symbolic variable
    /// (representing unknown input), or an expression tree built from
    /// operations on symbolic values.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    #[allow(clippy::derive_ord_xor_partial_ord)]
    pub enum SymbolicValue {
        /// A concrete 256-bit unsigned integer.
        Concrete(U256),
        /// A symbolic variable representing unknown input (e.g. calldata).
        Symbolic(SymVarId),
        /// An abstract expression over symbolic values.
        Expression(Box<SymExpr>),
    }

    /// Supported abstract expression types.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum SymExpr {
        Add(SymbolicValue, SymbolicValue),
        Sub(SymbolicValue, SymbolicValue),
        Mul(SymbolicValue, SymbolicValue),
        Div(SymbolicValue, SymbolicValue),
        Mod(SymbolicValue, SymbolicValue),
        And(SymbolicValue, SymbolicValue),
        Or(SymbolicValue, SymbolicValue),
        Xor(SymbolicValue, SymbolicValue),
        Not(SymbolicValue),
        Shl(SymbolicValue, SymbolicValue),
        Shr(SymbolicValue, SymbolicValue),
        /// A byte-size operation (e.g. BYTE opcode).
        Byte(u8, SymbolicValue),
        /// Keccak-256 hash of a symbolic byte sequence.
        Keccak256(SymbolicValue),
        /// Comparison: equal to.
        Eq(SymbolicValue, SymbolicValue),
        /// Comparison: less than (unsigned).
        Lt(SymbolicValue, SymbolicValue),
        /// Comparison: greater than (unsigned).
        Gt(SymbolicValue, SymbolicValue),
        /// Select from a symbolic storage mapping.
        Sload(SymbolicValue),
        /// Call data load at offset.
        CalldataLoad(SymbolicValue),
    }

    impl SymbolicValue {
        /// Create a new concrete value from a u64.
        pub fn concrete(val: u64) -> Self {
            SymbolicValue::Concrete(U256::from(val))
        }

        /// Create a fresh symbolic variable.
        pub fn fresh(id: SymVarId) -> Self {
            SymbolicValue::Symbolic(id)
        }
    }
}

// ---------------------------------------------------------------------------
// Execution State
// ---------------------------------------------------------------------------

mod state {
    use super::value::SymbolicValue;
    use std::collections::BTreeMap;

    /// The full symbolic EVM state.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ExecutionState {
        /// Symbolic stack (max 1024 elements).
        pub stack: Vec<SymbolicValue>,
        /// Symbolic memory (byte-addressable, unbounded).
        pub memory: BTreeMap<usize, SymbolicValue>,
        /// Symbolic storage (key → value).
        pub storage: Vec<(SymbolicValue, SymbolicValue)>,
        /// Program counter.
        pub pc: usize,
        /// Call data (symbolic byte sequence).
        pub calldata: Vec<SymbolicValue>,
        /// Call value in wei.
        pub callvalue: SymbolicValue,
        /// Return data from last subcall.
        pub returndata: Vec<SymbolicValue>,
        /// Gas remaining.
        pub gas: u64,
        /// Whether execution has halted.
        pub status: ExecutionStatus,
        /// Depth of call stack.
        pub call_depth: usize,
    }

    impl Default for ExecutionState {
        fn default() -> Self {
            Self {
                stack: Vec::new(),
                memory: BTreeMap::new(),
                storage: Vec::new(),
                pc: 0,
                calldata: Vec::new(),
                callvalue: SymbolicValue::Concrete(0u64.into()),
                returndata: Vec::new(),
                gas: u64::MAX,
                status: ExecutionStatus::Running,
                call_depth: 0,
            }
        }
    }

    /// Execution status of the EVM.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ExecutionStatus {
        /// Execution is still in progress.
        Running,
        /// Execution halted successfully (STOP, RETURN, SELFDESTRUCT).
        Halted,
        /// Execution reverted (REVERT).
        Reverted,
        /// Execution ran out of gas.
        OutOfGas,
        /// Execution encountered an invalid instruction.
        InvalidInstruction,
        /// Execution hit an explicit assertion failure.
        AssertionFailure,
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

mod error {
    use std::fmt;

    /// Errors that can occur during symbolic execution.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ExecutionError {
        /// Stack underflow (pop from empty stack).
        StackUnderflow,
        /// Stack overflow (push beyond 1024 elements).
        StackOverflow,
        /// Invalid opcode encountered.
        InvalidOpcode(u8),
        /// Out of gas.
        OutOfGas,
        /// Invalid jump destination.
        InvalidJumpDestination(u16),
        /// Memory out of bounds.
        MemoryOutOfBounds(usize),
        /// SMT solver error.
        SolverError(String),
        /// Internal engine error.
        Internal(String),
        /// Unsupported feature.
        Unsupported(String),
    }

    impl fmt::Display for ExecutionError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ExecutionError::StackUnderflow => write!(f, "stack underflow"),
                ExecutionError::StackOverflow => write!(f, "stack overflow (max 1024)"),
                ExecutionError::InvalidOpcode(op) => write!(f, "invalid opcode: 0x{op:02x}"),
                ExecutionError::OutOfGas => write!(f, "out of gas"),
                ExecutionError::InvalidJumpDestination(dest) => {
                    write!(f, "invalid jump destination: {dest}")
                }
                ExecutionError::MemoryOutOfBounds(addr) => {
                    write!(f, "memory access out of bounds at {addr}")
                }
                ExecutionError::SolverError(msg) => write!(f, "SMT solver error: {msg}"),
                ExecutionError::Internal(msg) => write!(f, "internal engine error: {msg}"),
                ExecutionError::Unsupported(feature) => {
                    write!(f, "unsupported feature: {feature}")
                }
            }
        }
    }

    impl std::error::Error for ExecutionError {}
}