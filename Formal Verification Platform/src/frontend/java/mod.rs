//! Java JVM Bytecode Verifier frontend.

pub mod bytecode;
pub mod junit_gen;
pub mod verify;

pub use verify::verify_class;
