# Keller-Verify: Formal Verification Pipeline for Rust

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust: nightly-2026-07-24](https://img.shields.io/badge/rustc-nightly--2026--07--24-orange.svg)](rust-toolchain.toml)
[![Solver: Z3 4.16.0](https://img.shields.io/badge/solver-Z3%204.16.0-green.svg)](https://github.com/Z3Prover/z3)
[![Corpus Decision Rate](https://img.shields.io/badge/decision%20rate-94.3%25-brightgreen.svg)](tests/cases.txt)
[![Unsupported Assertions](https://img.shields.io/badge/unsupported%20assertions-0-success.svg)](tests/cases.txt)

A compile-time **SMT-based formal verification platform** for the Rust ecosystem powered by the Z3 theorem prover. 

`keller-verify` intercepts the Rust compiler's Mid-level Intermediate Representation (MIR), symbolically traverses program execution paths, and proves the mathematical absence of runtime failures—including arithmetic overflows, out-of-bounds indexing, division by zero, invalid pointer dereferences, and coroutine state violations.

When an invariant can be violated, `keller-verify` extracts the exact counterexample assignments from Z3 and can **automatically synthesize native, runnable Rust unit tests** (`cargo test`) to reproduce the failure.

---

## Key Capabilities

* **Path-Sensitive Symbolic Execution**: Tracks symbolic expressions and branch conditions through the control-flow graph (CFG). Condition guards (`if x < 100 { x + 1 }`) are mathematically incorporated into path preconditions, eliminating false positives on bounded operations while flagging unguarded or weakly-guarded arithmetic.
* **Automated Concrete Test Synthesis (`keller testgen`)**: Takes SAT counterexample models (`.witness.smt2`) and automatically generates native Rust unit tests (`#[test] #[should_panic]`) binding the exact failure-inducing inputs directly to the target function under test.
* **Coroutine & Async State Machine Verification**: Performs fixpoint analysis over coroutine state transitions, verifying resume discriminants (`ResumedAfterReturn`, `ResumedAfterPanic`) and modeling suspension-to-drop invariants (`ResumedAfterDrop`).
* **Interprocedural Leaf Summaries (S7)**: Automatically synthesizes precondition requirements for private helper functions from internal arithmetic bounds, checking caller call sites and analyzing function bodies under their proven caller invariants.
* **Multi-Crate Workspace Analysis**: Operates seamlessly across entire Cargo workspaces via `cargo keller verify`. Aggregates individual crate worker receipts into unified JSON reports, SARIF logs, and interactive HTML dashboards.
* **Content-Addressed Verification Caching**: Caches per-function verification outcomes based on MIR and environment hashes, accelerating incremental verification in local development and CI pipelines.
* **Deterministic SMT-LIB Obligations**: Emits standalone `.smt2` proof obligations for every checked term (`[PASS]` UNSAT proofs, `[FAIL]` SAT counterexamples, and folded constants), enabling independent verification with `z3 -smt2`.
* **High Decision Coverage**: Decides **94.3%** of obligations across the standard internal test corpus with **0 unsupported assertions**, and demonstrated **95.9% decision coverage** on large-scale real-world crates (e.g. `SymEx-EVM` with 439 analyzed functions).

---

## Quick Start

### Prerequisites

1. **Rust Toolchain**: Nightly toolchain with compiler internal components:
   ```bash
   rustup toolchain install nightly-2026-07-24 --component rustc-dev --component llvm-tools-preview
   ```
2. **Z3 Theorem Prover**: Version 4.16.0.
   * **Linux**: `libz3-dev` or unpack to `.z3/` / set `Z3_HOME`.
   * **Windows**: Download Z3 binary release, add `bin` to `PATH` or set `$env:Z3_HOME`.

### Building from Source

```bash
git clone https://github.com/KELLERBABG/Formal-Verification.git
cd Formal-Verification
cargo build --release
```

The build generates two primary binaries in `target/release/`:
* `keller-verify`: The core verification engine and compiler wrapper.
* `cargo-keller`: The Cargo plugin providing the `cargo keller` subcommand.

---

## Usage

### 1. Verifying a Project

Run the verifier inside any Rust crate or workspace root:

```bash
# Verify the current crate or workspace
cargo keller verify

# In CI: enforce that no undecided warnings or unmodelled operations remain
cargo keller verify --deny-warn

# Restrict analysis to specific functions or limit finding count
cargo keller verify --only "parse_*" --max-findings 5

# Set custom solver timeout per obligation (default: 30000 ms)
cargo keller verify --solver-timeout 10000
```

### 2. Synthesizing Runnable Reproduction Tests

When `keller-verify` detects a failure (`[FAIL]`), it writes a counterexample witness obligation to `obligations/<crate>/<function>_<bb>.witness.smt2`. 

To turn this formal witness into a runnable Rust reproduction test:

```bash
# Generate test for a single failing witness
keller-verify testgen obligations/my_crate/check_overflow_bb0.witness.smt2

# Or recursively generate tests for all failures in a directory
keller-verify testgen obligations/
```

Example synthesized reproduction test:
```rust
//! Auto-generated counterexample reproduction tests from Keller-Verify.
//! Run with `cargo test` to reproduce formal verification failures natively.

#[cfg(test)]
mod keller_counterexample_tests {

    #[test]
    #[should_panic]
    fn repro_my_crate_check_overflow_bb0() {
        // Crate: my_crate, Function: `check_overflow` (bb0)
        // Obligation: overflow (counterexample: [x = 255, y = 1])
        let x = 255;
        let y = 1;
        let _ = my_crate::check_overflow(x, y);
    }
}
```

### 3. CI Gating and Baselining

For large existing projects, adopt verification progressively without failing builds on existing legacy issues:

```bash
# Generate a baseline of current findings
cargo keller verify --write-baseline .keller-baseline.json

# Fail CI only if new violations or regressions are introduced
cargo keller verify --baseline .keller-baseline.json --sarif keller.sarif
```

Upload `keller.sarif` to GitHub Actions to render findings directly on pull request diffs via the GitHub Security / Code Scanning tab.

---

## Verification Verdicts & Exit Codes

### Verdict Classification

| Verdict | Meaning | Exit Code (Default) | Exit Code (`--deny-warn`) |
| :--- | :--- | :---: | :---: |
| **`[PASS]`** | **Mathematically proven safe.** The negation of the assertion is UNSAT under path preconditions. | `0` | `0` |
| **`[FAIL]`** | **Counterexample found.** Z3 found concrete inputs that violate the assertion (SAT), or constant propagation found a guaranteed panic. | `1` | `1` |
| **`[WARN]`** | **Undecided or unmodelled.** The solver timed out, hit complexity limits, or encountered unmodelled external functions. | `0` | `1` |

### Process Exit Codes

* **`0`**: Success. All assertions were checked and verified according to the active gate contract.
* **`1`**: Verification failure. At least one `FAIL` occurred (or a `WARN` under `--deny-warn`).
* **`2`**: Command-line invocation error (unrecognized flag or invalid arguments).
* **`3`**: Execution / setup failure. The target project failed to compile with `rustc`, the toolchain was missing, or the global `--run-timeout` was exceeded.

---

## What Keller-Verify Checks

1. **Integer Arithmetic Overflow**: `Add`, `Sub`, `Mul` on all signed and unsigned integer widths (`u8`–`u128`, `i8`–`i128`, `usize`, `isize`), including negation overflow (`-MIN`).
2. **Division & Modulo Safety**: Checks that divisors in `/` and `%` cannot evaluate to zero, and checks `MIN / -1` overflow conditions.
3. **Array & Slice Bounds**: Symbolic proofs that index operations (`slice[index]`, `array[index]`) are strictly bounded by `0 <= index < len`.
4. **Shift Width Compliance**: Validates that shift operands in `<<` and `>>` remain within `0 <= amount < bit_width`.
5. **Coroutine & Async Transitions**: Verifies coroutine resume invariants and models suspension-to-drop states, ensuring poisoned or returned states cannot be resumed.
6. **Decided Receiver Panics**: Evaluates `unwrap()` and `expect()` calls; statically proves calls on known succeeding variants (`Some`, `Ok`) and flags known failing variants (`None`, `Err`).
7. **Pointer Provability**: Validates non-null and alignment invariants for pointer provenance and refutes invalid memory configurations with concrete counterexample addresses.

---

## Real-World Validation Benchmarks

`keller-verify` has been stress-tested across external production codebases and complex mathematical libraries:

* **`SymEx-EVM`**: Analyzed **439 functions**, checked 74 formal obligations across EVM gas schedules, memory expansion routines, and arithmetic state. Proved 45 functions safe and uncovered 19 true integer overflow exploit paths, achieving a **95.9% decision rate** across 70 solver calls in 34.59s.
* **`OP-Roboter/op-math`**: Analyzed **108 mathematical functions** spanning 3D vectors, quaternions, rotation matrices, and spatial isometries in 1.28s.
* **`CityOS/city-types`**: Formally analyzed 20 spatial and municipal data-structure functions with a **100% clean PASS rate**.
* **Self-Verification**: Successfully verifies the entire `keller-verify` codebase itself (677 functions) clean in CI.

---

## Architecture

```
                    ┌────────────────────────────────────────┐
                    │          cargo keller verify           │
                    │        (or keller-verify CLI)          │
                    └───────────────────┬────────────────────┘
                                        │
                             Spawns cargo check with
                           RUSTC_WORKSPACE_WRAPPER
                                        │
                                        ▼
                    ┌────────────────────────────────────────┐
                    │       rustc_driver (after_analysis)    │
                    └───────────────────┬────────────────────┘
                                        │ Extracts MIR
                                        ▼
          ┌───────────────────────────────────────────────────────────┐
          │  Pass 1: Constant Propagation & Abstract Interpretation   │
          │  - Scalar integer evaluation via try_eval_scalar_int()    │
          │  - Constant folding on BinaryOp and discriminants         │
          └─────────────────────────────┬─────────────────────────────┘
                                        │
                                        ▼
          ┌───────────────────────────────────────────────────────────┐
          │  Pass 2: Symbolic Path State & CFG Traversal              │
          │  - Path-sensitive branch guard accumulation               │
          │  - Interprocedural helper summary propagation (S7)        │
          │  - Coroutine fixpoint & suspension-to-drop modeling (S6)  │
          └─────────────────────────────┬─────────────────────────────┘
                                        │
                                        ▼
          ┌───────────────────────────────────────────────────────────┐
          │  Pass 3: SMT Constraint Generation & Z3 Solving          │
          │  - Exact bit-vector theories (qfbv tactic) & Int bounds   │
          │  - Per-obligation timeout enforcement (solver-timeout)    │
          └─────────────────────────────┬─────────────────────────────┘
                                        │
                  ┌─────────────────────┴─────────────────────┐
                  ▼                                           ▼
          UNSAT (Proved Safe)                         SAT (Counterexample)
                  │                                           │
         [PASS] (unsat.smt2)                       [FAIL] (witness.smt2)
                                                              │
                                                              ▼
                                                   ┌─────────────────────┐
                                                   │   keller testgen    │
                                                   │ (Generates #[test]) │
                                                   └─────────────────────┘
```

---

## Documentation & References

* [User Guide & CLI Flag Reference](docs/USAGE.md)
* [Release Matrix & Architecture Limits](docs/RELEASE.md)
* [Schema Evolution & Versioning](docs/SCHEMA-EVOLUTION.md)
* [Trust Checklist & Soundness Audits](docs/TRUST-CHECKLIST.md)
* [Technical Whitepaper](docs/WHITEPAPER.md)

---

## Contributing

We warmly welcome community contributions! Please read our [Contributing Guide](CONTRIBUTING.md) and [Security Policy](SECURITY.md) before submitting pull requests.

To run the local verification and integration suite:
```bash
cargo test                 # Pure logic unit tests
tests/run_tests.sh         # Linux / macOS full fixture test suite
tests\run_tests.cmd        # Windows full fixture test suite
python scripts/progress-metric.py  # Run decision-rate regression check
```

---

## License

Dual-licensed under either of:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
