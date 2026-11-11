# SymEx-EVM — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Rust`
> **Summary:** A 2-crate Rust symbolic execution engine for EVM bytecode (`crates/core` and `crates/cli`) integrating Z3 SMT solving, abstract interpretation, concolic fuzzing, opcode parsing, gas accounting, and reentrancy/overflow vulnerability pattern matching.

## Task Queue

- [ ] **TASK-01: Implement cross-function reentrancy detection with state-write after call constraint matching**
  - **Target Files:** `crates/core/src/vulnerabilities/mod.rs`, `crates/core/src/evm/executor.rs`
  - **Context Files:** `crates/core/src/path_constraints/mod.rs`, `crates/core/src/z3_backend/mod.rs`
  - **Verify:** `cargo test -p symbolic_execution_core -p symbolic-execution-cli --quiet`
  - **Spec:** Enhance `crates/core/src/vulnerabilities/mod.rs` and `executor.rs` to detect the classic reentrancy pattern (SWC-107).

1. Flag a critical vulnerability if an execution path contains an external `CALL` instruction that forwards value or control followed by an `SSTORE` updating balance or state variables without a preceding non-reentrant mutex guard.
2. Use the Z3 backend to solve for concrete calldata that triggers the reentrancy loop and generates a reproducible witness.
3. Add integration tests in `tests/integration_test.rs` analyzing vulnerable vs. guarded reentrancy test bytecodes.

- [ ] **TASK-02: Add quadratic memory expansion gas accounting and unbounded loop DoS solver**
  - **Target Files:** `crates/core/src/evm/gas.rs`, `crates/core/src/analysis/mod.rs`
  - **Context Files:** `crates/core/src/evm/opcodes.rs`
  - **Verify:** `cargo test -p symbolic_execution_core --quiet`
  - **Spec:** Update `crates/core/src/evm/gas.rs` to compute exact EVM gas schedules including memory expansion cost $C_{\text{mem}}(a) = 3 \cdot a + \lfloor \frac{a^2}{512} \rfloor$.

1. In `analysis/mod.rs`, detect loop backward branches whose iteration bounds are unconstrained or controlled by user-supplied array lengths without pagination, asserting `GasExhaustionDoS` if required gas $> 30\text{M}$ block limit.
2. Add unit tests for gas calculation and unbounded loop detection.
