//! Integration tests for the symbolic execution engine.
//!
//! These tests verify the full pipeline: bytecode parsing → IR construction →
//! symbolic execution → analysis → reporting.

use symbolic_execution_core::*;

/// Test analyzing a simple contract with a single function.
/// Bytecode: PUSH1 0x42 PUSH1 0x00 MSTORE PUSH1 0x20 PUSH1 0x00 RETURN
/// This is the simplest possible contract that returns 0x42.
#[test]
fn test_analyse_simple_contract() {
    // Minimal bytecode that returns 0x42
    let bytecode = vec![
        0x60, 0x42, // PUSH1 0x42
        0x60, 0x00, // PUSH1 0x00
        0x52,       // MSTORE
        0x60, 0x20, // PUSH1 0x20
        0x60, 0x00, // PUSH1 0x00
        0xf3,       // RETURN
    ];

    let engine = AnalysisEngine::new();
    let report = engine.analyze_bytecode(&bytecode);

    assert!(!report.ir.bytecode.is_empty());
    assert_eq!(report.ir.bytecode.len(), 7);
}

/// Test parsing bytecode with jump destinations.
#[test]
fn test_parse_bytecode_with_jumps() {
    // JUMPDEST PUSH1 0x00 PUSH1 0x00 JUMPI (jumps to 0 if cond is nonzero)
    let bytecode = vec![
        0x5b,       // JUMPDEST
        0x60, 0x00, // PUSH1 0x00
        0x60, 0x00, // PUSH1 0x00 (condition)
        0x57,       // JUMPI
    ];

    let parsed = evm::parser::ParsedBytecode::parse(&bytecode).unwrap();
    assert_eq!(parsed.instructions.len(), 4);
    assert_eq!(parsed.jump_dests, vec![0]);
}

/// Test that the symbolic executor can process basic arithmetic.
#[test]
fn test_symbolic_executor_arithmetic() {
    // PUSH1 0x03 PUSH1 0x02 ADD STOP
    let bytecode = vec![0x60, 0x03, 0x60, 0x02, 0x01, 0x00];
    let parsed = evm::parser::ParsedBytecode::parse(&bytecode).unwrap();
    let mut executor = evm::executor::SymbolicExecutor::new(parsed);
    let mut state = ExecutionState::default();

    // Run until halted
    let mut steps = 0;
    while state.status == ExecutionStatus::Running && steps < 10 {
        let result = executor.step(&mut state).unwrap();
        state = result.state;
        steps += 1;
    }

    assert_eq!(state.status, ExecutionStatus::Halted);
    assert_eq!(steps, 4); // PUSH1, PUSH1, ADD, STOP
}

/// Test that CALLVALUE is properly handled symbolically.
#[test]
fn test_callvalue_symbolic() {
    // CALLVALUE STOP
    let bytecode = vec![0x34, 0x00];
    let parsed = evm::parser::ParsedBytecode::parse(&bytecode).unwrap();
    let mut executor = evm::executor::SymbolicExecutor::new(parsed);
    let mut state = ExecutionState::default();

    state.callvalue = SymbolicValue::fresh(types::SymVarId(0));

    let result = executor.step(&mut state).unwrap();
    state = result.state;

    assert_eq!(state.stack.len(), 1); // CALLVALUE pushed the value
}