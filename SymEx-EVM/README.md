# Symbolic Execution Engine for Smart Contracts

A formal verification engine for EVM smart contracts combining abstract interpretation, symbolic execution, SMT solving (Z3), path constraint generation, vulnerability pattern matching, taint analysis, and counter-example generation.

## Features

- **EVM Bytecode Analysis**: Parse and execute any EVM bytecode (Frontier–Shanghai)
- **Symbolic Execution**: Full instruction set with symbolic value domain (Concrete / Symbolic / Expression)
- **Path Constraints**: Automatic branch condition collection for all execution paths
- **State Space Exploration**: BFS, DFS, coverage-guided, and heuristic search strategies with **state merging** (ITE-based path convergence to mitigate explosion)
- **Concolic Execution**: Concrete fuzzing alongside symbolic execution — generate random calldata seeds, execute concretely, collect traces, negate constraints, solve for new inputs
- **SMT Solving**: Dual-backend architecture — fallback SMT-LIB 2 string generation OR direct `z3` C-API native bindings (10-100x faster, incremental push/pop solving)
- **Keccak Optimization**: `SymbolicHashTable` models Keccak-256 as an uninterpreted function with preimage mapping, avoiding intractable bit-vector expansion in Z3
- **Formal Verification**: Prove invariants, assertions, access control, and overflow safety
- **Vulnerability Detection**: 6 built-in detectors (reentrancy, overflow, tx.origin, unchecked calls, access control, timestamp dependency)
- **Taint Analysis**: Track untrusted input flow from CALLDATA/CALLVALUE to critical sinks (SSTORE, CALL, SELFDESTRUCT)
- **Gas Analysis**: Post-London gas model (EIP-2929 cold/warm, EIP-150 call depth, EIP-1153 transient storage, EIP-3860 code size costs)
- **Counter-Example Generation**: Synthesize concrete exploit inputs from SMT models
- **Automated Spec Generation**: Infer likely invariants from observed contract behavior

## Project Structure

```
├── Cargo.toml                          # Workspace root
├── crates/
│   ├── core/                           # Core analysis library
│   │   └── src/
│   │       ├── lib.rs                  # Public API & re-exports
│   │       ├── types/                  # Address, U256, SymbolicValue (Concrete|Symbolic|Expression), SymExpr (15 variants), ExecutionState, ExecutionError
│   │       ├── evm/                    # 256-entry opcode table, bytecode parser, symbolic executor, post-London gas costs
│   │       ├── ir/                     # BasicBlock, ControlFlowGraph, ContractIR, build_ir()
│   │       ├── abstract_interpretation/ # Abstract domain engine (Symbolic|Interval|Concrete)
│   │       ├── path_constraints/       # Branch condition collection & constraint forking
│   │       ├── state_explorer/         # BFS/DFS/Coverage/Heuristic + state merging (ITE)
│   │       ├── smt/                    # High-level SMT solver interface
│   │       ├── z3_backend/             # Dual Z3 backend: Native (z3-sys) + SmtLib fallback
│   │       ├── hash/                   # SymbolicHashTable — Keccak uninterpreted function optimization
│   │       ├── concolic/               # Concolic execution engine (concrete seeds + symbolic constraints)
│   │       ├── verification/           # Invariant proving & formal verification
│   │       ├── counter_examples/       # Exploit input synthesis from SMT models
│   │       ├── vulnerabilities/        # 6 pattern-matching detectors
│   │       ├── gas/                    # Gas consumption analysis
│   │       ├── taint/                  # Data flow tracking (sources → sinks)
│   │       ├── spec_gen/               # Automated specification inference
│   │       └── analysis/               # Full pipeline orchestrator
│   └── cli/                            # `symex` command-line tool
│       └── src/main.rs                 # analyze, scan, gas, detectors, version commands
├── docs/
│   ├── architecture.md                 # Architecture diagrams, module dependencies, data flow
│   └── whitepaper.md                   # 13-section technical whitepaper
├── tests/
│   └── integration_test.rs             # Integration tests
└── .github/workflows/ci.yml            # CI pipeline (build, test, clippy, fmt, docs)
```

## Quick Start

```bash
# Build the workspace
cargo build --workspace

# Run all tests (54 unit tests)
cargo test --workspace

# Run the example
cargo run --example basic_analysis -p symbolic-execution-core

# Use the CLI
cargo run -p symbolic-execution-cli -- --help
```

## CLI Usage

```bash
# Full analysis of a contract bytecode file
cargo run -p symbolic-execution-cli -- analyze -i contract.hex

# Vulnerability scan
cargo run -p symbolic-execution-cli -- scan -i contract.hex

# Gas analysis
cargo run -p symbolic-execution-cli -- gas -i contract.hex

# List available vulnerability detectors
cargo run -p symbolic-execution-cli -- detectors

# Show version
cargo run -p symbolic-execution-cli -- version
```

## Architecture

The engine processes bytecode through a multi-stage pipeline with three key architectural innovations:

### 1. Concolic Execution
Concrete fuzzing seeds guide symbolic path exploration. The `ConcolicEngine` generates random calldata, executes concretely via `ConcreteEVM`, collects PC traces, and feeds coverage back into the symbolic exploration prioritization.

### 2. Keccak Symbolic Hash Table
EVM storage mapping keys (`keccak256(key . slot)`) with symbolic keys normally cause intractable Z3 bit-vector expansion. The `SymbolicHashTable` models Keccak as an **uninterpreted function** — concrete preimages get real hashes, symbolic preimages get fresh symbolic variables. SMT-LIB declares `keccak256` as an opaque function with optional injectivity axiom.

### 3. Dual Z3 Backend
- **`Z3NativeBackend`**: Direct C-API bindings via `z3-sys` (10-100x faster, incremental push/pop, no string serialization)
- **`SmtLibBackend`**: SMT-LIB 2 string generation (fallback, works without `z3` crate)

### Pipeline Stages

1. **EVM Parser** → `ParsedBytecode` (instruction stream + jump destinations)
2. **IR Construction** → `ContractIR` (control flow graph + function selectors)
3. **Abstract Interpretation** → Lifts all values to the symbolic domain
4. **Symbolic Execution** → Step through instructions building symbolic state
5. **Path Constraints** → Collect branch conditions at each JUMPI
6. **State Exploration** → Fork execution at each branch, merge at converging PCs
7. **Concolic Feedback** → Concrete seed traces guide path prioritization
8. **SMT Solving** → Direct native solver calls (or SMT-LIB fallback)
9. **Analysis** → Vulnerability scanning, taint analysis, gas analysis, verification, spec generation

## Module Overview

| Module | Description | Key Types |
|--------|-------------|-----------|
| `types` | Core domain types | `Address`, `U256`, `SymVarId`, `SymbolicValue`, `SymExpr` (15 variants), `ExecutionState`, `ExecutionStatus`, `ExecutionError` |
| `evm` | EVM instruction set & symbolic executor | `Instruction` (256-entry table), `ParsedBytecode`, `SymbolicExecutor`, `BranchConstraints`, `StepResult`, `AccessList` |
| `ir` | Intermediate representation | `BasicBlock`, `BlockId`, `ControlFlowGraph`, `ContractIR`, `ContractMetadata` |
| `abstract_interpretation` | Symbolic state management | `AbstractDomain` (Symbolic/Interval/Concrete), `AbstractConfig`, `AbstractInterpreter` |
| `path_constraints` | Branch condition collection | `PathConstraint`, `ConstraintCollector` |
| `state_explorer` | State space search + merging | `SearchStrategy` (BFS/DFS/Coverage/Heuristic), `MergedState`, `ExplorerConfig`, `StateSnapshot`, `ExplorationResult` |
| `smt` | High-level SMT solver | `SmtConfig`, `SmtSolver`, `SmtResult`, `SmtModelValue` |
| `z3_backend` | Dual Z3 solver backend | `Z3Backend` trait, `Z3NativeBackend`, `SmtLibBackend`, `Z3Solver`, `SolverBackend` |
| `hash` | Keccak uninterpreted function | `HashEntry`, `SymbolicHashTable` |
| `concolic` | Concolic execution engine | `ConcolicConfig`, `ConcreteTrace`, `TraceStep`, `ConcreteEVM`, `ConcolicEngine` |
| `verification` | Formal invariant verification | `Invariant`, `InvariantSeverity`, `VerificationResult`, `PropertyType`, `VerificationProperty`, `Verifier` |
| `counter_examples` | Exploit input synthesis | `CounterExample`, `CounterExampleGenerator` |
| `vulnerabilities` | Vulnerability pattern matching | `Severity` (Critical/High/Medium/Low/Info), `Vulnerability`, `ScanResult`, `VulnerabilityDetector` trait, `VulnerabilityScanner` |
| `gas` | Gas consumption analysis | `GasAnalysisResult`, `GasPath`, `GasBreakdown`, `GasAnalyzerConfig`, `GasAnalyzer` |
| `taint` | Taint tracking | `TaintStatus` (Clean/Tainted/Partial), `TaintSource` (Calldata/Callvalue/Returndata/Storage/Block), `TaintSink` (Sstore/Call/SelfDestruct), `TaintFlow`, `TaintAnalyzer` |
| `spec_gen` | Automated specification generation | `SpecificationCandidate`, `SpecSource` (Observed/PatternMatch/Derived/UserProvided), `SpecGenConfig`, `SpecGenerator` |
| `analysis` | Full pipeline orchestration | `AnalysisConfig`, `AnalysisReport`, `AnalysisEngine` |

## License

Apache 2.0