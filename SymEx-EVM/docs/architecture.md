# Architecture

## Overview

The Symbolic Execution Engine is a formal verification tool for smart contracts. It parses EVM bytecode, symbolically executes it along all possible paths, collects path constraints, and uses SMT solving to detect vulnerabilities and verify invariants.

## Pipeline Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                         CLI (symex)                                              │
│             clap-based command-line interface (analyze, scan, gas)               │
└───────────────────────────────────┬─────────────────────────────────────────────┘
                                    │
┌───────────────────────────────────▼─────────────────────────────────────────────┐
│                               Analysis Engine                                    │
│                    Orchestrates the full analysis pipeline                       │
└──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬────┘
       │      │      │      │      │      │      │      │      │      │      │
       ▼      ▼      ▼      ▼      ▼      ▼      ▼      ▼      ▼      ▼      ▼
   ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐
   │  IR  │ │AbsInt│ │ Path │ │State │ │ SMT  │ │ Taint│ │ Gas  │ │Conc  │ │ Hash │ │ Z3   │
   │      │ │      │ │ ░░░  │ │ ░░░  │ │ ░░░  │ │ ░░░  │ │ ░░░  │ │olic  │ │ Table│ │Nativ │
   └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘ └──────┘
       │                                    │         │
       ▼                                    ▼         ▼
   ┌──────┐                          ┌──────────┐ ┌──────────┐
   │ EVM  │                          │Verification│ │CounterEx │
   │░░░░░░│                          │░░░░░░░░░░│ │░░░░░░░░░░│
   └──────┘                          └──────────┘ └──────────┘
```

## Module Dependency Graph

```
types  ←── evm  ←── ir  ←── abstract_interpretation
                                         │
                                         ▼
                                  path_constraints
                                         │
                                         ▼
                                  state_explorer (state merging)
                                         │
                                         ▼
                          ┌────────── smt ──────────┐
                          │           │              │
                          ▼           ▼              ▼
                    z3_backend    verification    counter_examples
                    ┌────┴────┐
                    │         │
              Native(Z3)  SmtLib(fallback)
                                         │
                    ┌────────────────────┼────────────────────┐
                    ▼                    ▼                     ▼
              vulnerabilities          taint                 gas
                                         │
                    ┌────────────────────┼────────────────────┘
                    ▼                    ▼
              concolic               spec_gen
              (concrete EVM            (spec inference
               + fuzzing seeds)         + candidates)
                    │                    │
                    └────────────────────┼────────────────────┘
                                         ▼
                                     analysis
```

## Key Data Flow

1. **Bytecode In** → EVM parser produces `ParsedBytecode` (instruction stream + jump destinations)
2. **IR Construction** → `build_ir()` creates `ContractIR` with control flow graph and function selectors
3. **Symbolic Execution** → `SymbolicExecutor` interprets instructions over `SymbolicValue` domain:
   - `Concrete(U256)` — known constant
   - `Symbolic(SymVarId)` — unknown input (calldata, callvalue)
   - `Expression(SymExpr)` — expression tree (Add, Sub, Mul, Eq, Lt, Gt, Keccak256, etc.)
4. **Path Constraints** → `ConstraintCollector` accumulates branch conditions at each JUMPI
5. **State Exploration** → `StateExplorer` forks execution at each branch, with **state merging** to mitigate path explosion (ITE expressions for converging paths)
6. **Concolic Execution** → `ConcolicEngine` runs concrete fuzzing seeds alongside symbolic exploration to guide path prioritization and discover new coverage
7. **SMT Solving** → Dual-backend architecture:
   - `Z3NativeBackend` — direct Z3 C-API bindings (10-100x faster, incremental solving)
   - `SmtLibBackend` — SMT-LIB 2 string generation (fallback)
8. **Keccak Optimization** → `SymbolicHashTable` models Keccak-256 as an uninterpreted function with preimage mapping, avoiding intractable bit-vector expansion in Z3
9. **Analysis** → Multiple passes run on the explored states:
   - Vulnerability scanning (6+ detectors: reentrancy, overflow, tx.origin, unchecked calls, access control, timestamp dependency)
   - Taint analysis (untrusted input → critical state)
   - Gas analysis (post-London: EIP-2929 cold/warm, EIP-150 call depth, EIP-1153 transient storage)
   - Formal verification (invariant proving)
   - Counter-example generation (exploit input synthesis)
   - Spec generation (invariant candidate inference)

## New Modules (Architectural Improvements)

### `concolic/` — Concolic Execution
- **Purpose**: Mitigate path explosion by combining concrete fuzzing with symbolic exploration
- **Key types**: `ConcolicConfig`, `ConcreteTrace`, `TraceStep`, `ConcreteEVM`, `ConcolicEngine`
- **Strategy**: Generate random seed calldata → execute concretely via `ConcreteEVM` → collect PC traces → negate branch constraints → solve for new inputs → repeat

### `z3_backend/` — Z3 Direct Bindings
- **Purpose**: Replace slow SMT-LIB string generation with direct Z3 C-API calls
- **Key types**: `Z3Backend` trait, `Z3NativeBackend`, `SmtLibBackend`, `Z3Solver`, `SolverBackend`
- **Architecture**: Abstract trait pattern — fallback `SmtLibBackend` works without `z3` crate, `Z3NativeBackend` bridges to `z3-sys` when available

### `hash/` — Symbolic Hash Table
- **Purpose**: Avoid Keccak-256 bit-blasting in Z3 by modeling it as an uninterpreted function
- **Key types**: `HashEntry`, `SymbolicHashTable`
- **Mechanism**: Real Keccak-256 for concrete preimages, fresh symbolic variables for symbolic preimages, injectivity constraint in SMT-LIB

## Gas Model Evolution

| EIP | Feature | Status |
|-----|---------|--------|
| Frontier | Base opcode costs | ✅ Implemented |
| EIP-150 | Call depth stipend (63/64 rule) | ✅ Implemented |
| EIP-2028 | Calldata cost reduction | ✅ Implemented |
| EIP-2200 | SSTORE net gas metering | ✅ Simplified |
| EIP-2929 | Cold/warm access costs (Berlin) | ✅ Implemented |
| EIP-3860 | Code size cost (200 gas/byte) | ✅ Implemented |
| EIP-1153 | Transient storage (Cancun) | ✅ Implemented |

## State Merging

To combat path explosion, the explorer can merge states at the same program counter:

```
Before merging:                  After merging:
  path_A ──→ PC=42 (stack=[1])     path_A ──→ PC=42 (stack=[ITE(cond, 1, 2)])
  path_B ──→ PC=42 (stack=[2])     path_B ──→ PC=42 (stack=[ITE(cond, 1, 2)])
```

This creates ITE (If-Then-Else) expressions for diverging values, using Z3's native support for conditional expressions rather than forking into separate solver contexts.