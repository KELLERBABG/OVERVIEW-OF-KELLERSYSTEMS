# Keller-Verify: Compile-Time Formal Verification Pipeline — Technical Whitepaper

**Version 1.2 — October 2026**

> A Rust compiler plugin performing SMT-based formal verification at compile time using the Z3 solver — intercepting the compiler's MIR (Mid-level IR), extracting bounds checks, arithmetic overflow checks, and division-by-zero assertions, and symbolically proving whether they can fail.

---

## Abstract

Keller-Verify is a `rustc_driver` compiler plugin that transforms the Rust compiler itself into a **formal verification tool**. By hooking into the compiler's `after_analysis` callback, it reads the Mid-level IR (MIR) of every compiled function, extracts the runtime safety assertions the compiler itself generates (bounds checks on array/slice access, integer overflow checks, division/remainder-by-zero checks), encodes them as Z3 SMT constraints, and mathematically proves each assertion either **can never fail** (`[PASS]`, UNSAT), **has a concrete failing input** (`[FAIL]`, SAT), or **cannot be decided** (`[WARN]`, timeout/complexity — or the analysis itself gave up precision, which a function's dataflow does when it cannot reach a fixpoint inside its round budget: that is now a recorded finding of its own, naming the budget and warning that the `FAIL`s beside it may be false positives, rather than a loss of precision that only showed up as one). This gives Rust developers machine-checked proof of safety-critical properties *at every compile*, with no separate verification workflow. The project ships **twenty-one integration fixture cases** — safe code; deliberate counterexamples covering overflow, division by zero, a branch-merged constant, a loop whose counter increment is *proved* while its accumulator overflows, a *nested* loop whose guarded increments the same widening proves, a shift amount, and a crate of six checks that constant folding decides without asking Z3; a case whose one obligation Z3 genuinely cannot decide, so the verdict is a `WARN` rather than a proof while its one bounded multiply is still proved, which is what the `--deny-warn` half of the exit-code contract is asserted against end to end; a guarded case that must be *proved* rather than flagged; a two-member workspace; and a crate the compiler rejects, which has to come out as a setup failure and not as a clean run; the panic sites an unmodelled call leaves as a disclosure, and the sites a known enum variant decides — fifteen of them with a committed golden `report.json`. That is **164 unit tests**, plus GitHub Actions CI configured to run the fixture suite on both runners, the unit tests and the verifier on its own crate; the workflow is in the tree, but no run of it has been observed from this checkout.

---

## 1. Motivation

Rust's type system and borrow checker eliminate whole classes of memory bugs, but **integer overflow, out-of-bounds indexing, and division by zero are still runtime panics** — the compiler inserts a check but doesn't prove it can't fire. In safety-critical contexts (embedded, medical, aerospace, kernel code), a *runtime panic is a failure mode that must be excluded*, not merely handled. Keller-Verify makes these checks **compile-time theorems**:

- Bounds check `index < len` → proven safe for all reachable program states
- Overflow check `x + y` → proven safe against the **type-aware** maximum (u8→255, i8→127, u32→4.2B…)
- Division/remainder by zero → proven impossible

## 2. Architecture

```
keller verify  ── sets RUSTC_WRAPPER=self ── runs cargo check
  └── for each crate compilation:
      └── rustc_driver callback (after_analysis)
          ├── PASS 1: Constant Propagation
          │   ├── Walk reachable basic blocks (bb0 → reachable set)
          │   ├── try_eval_scalar_int() for constant folding
          │   └── Fold BinaryOp with constant operands
          ├── PASS 2: Path-Sensitive Symbolic State
          │   ├── Walk the CFG from bb0, folding branch guards
          │   ├── Merge predecessors as "one of these" (phi)
          │   └── A cycle is unrolled to its back edge; past the trip bound the
          │       loop-carried values are widened while the guards stay
          ├── PASS 3: SMT Analysis on Assert Terms
          │   ├── BoundsCheck  → type-aware bounds (unsigned ≥ 0)
          │   ├── Overflow     → per-type max (u8/..,/i64)
          │   ├── DivByZero    → symbolic divisor = 0
          │   └── RemByZero    → symbolic divisor = 0
          │   Both operands constant? decided by folding instead, so Z3 is
          │   never asked — and it is still a FAIL, with a witness and a
          │   ground obligation of its own
          │   Z3 context: reused per analysis session
          └── Result: SAT=FAIL  UNSAT=PASS  UNKNOWN=WARN
      └── Report: report.json + live console stream
      └── A crate that does not compile: no verdict at all, exit 3
```

### Key design decisions
- **RUSTC_WRAPPER self-invocation** — transparently intercepts every `cargo check` without requiring build-script changes in target projects.
- **`try_eval_scalar_int()` API** — replaces the original buggy debug-string parsing; typed constant propagation that actually works on locals (`_1`, `_2`).
- **Numbered locals** — local references by index (not string names like `"len"` which never matched).
- **Type-aware overflow bounds** — u8 caps at 255 (not 9e18); the original always used `i64::MAX`.
- **Reachability analysis** — only reachable basic blocks from `bb0` are analyzed; panic internals skipped.
- **Shared Z3 context** — created once per analysis session (not per assertion).
- **Cross-platform** — Windows + Linux + macOS via conditional Z3 paths, `Z3_HOME` env var (defaults: `C:\z3-4.16.0-x64-win`, `/usr`, `/usr/local`).

## 3. Verification Semantics

For each runtime assertion term encountered in MIR, the verifier encodes its negation along with the reachable-state constraints into Z3:

| Assertion Type | Z3 Query | PASS (UNSAT) | FAIL (SAT) |
|---|---|---|---|
| Array index bounds | `¬(index < len ∧ index ≥ 0 lanes)` | index provably in bounds | Z3 finds a violating index |
| Integer overflow | `¬(result within Atype_min, type_max])` | no overflow possible | concrete overflowing operands |
| Division by zero | `¬(divisor ≠ 0)` | divisor provably non-zero | divisor can be zero |
| Remainder by zero | `¬(divisor ≠ 0)` | divisor provably non-zero | divisor can be zero |

Decision semantics:
- **`[PASS]`** — the assertion can never fail: mathematically proven UNSAT.
- **`[FAIL]`** — Z3 found a concrete input that triggers the assertion (SAT) — a real exploitable bug candidate.
- **`[WARN]`** — Z3 could not decide (timeout / nonlinear complexity).

**Exit codes:** `0` when every obligation was decided and none failed, `1` when the gate fails (a counterexample, or an undecided verdict under `--deny-warn`), `2` when the command line was not understood, `3` when the analysis could not run at all (the code did not compile, or `cargo` is missing). A setup failure deliberately does not borrow the code that means "a counterexample was found".

The `3` is worth spelling out, because it was documented before it was true. This binary runs `rustc` itself, as the wrapper, and it discarded the status `rustc_driver::catch_with_exit_code` returned — so a crate the compiler rejected came out as **exit 0 with an empty `report.json`**. A body that was never built contains no assertions, and "no findings" is not "nothing to check". The status is propagated now, which also brings the wrapper into line with `cargo build`: reading `optimized_mir` for every function runs MIR passes that a plain `cargo check` skips, so a deny-by-default lint such as `arithmetic_overflow` can surface here for code `cargo check` accepts and `cargo build` rejects. The wrapper follows the build, and the case is pinned by `tests/broken_code`.

## 4. Improvements Over the Original Version

| Feature | Before | After |
|---|---|---|
| False positives | Massive (string-based constant parsing always failed) | Eliminated (typed constant propagation) |
| Division-by-zero | Silently skipped | Detected (constant + Z3) |
| Remainder-by-zero | Silently skipped | Detected |
| Overflow w/ variables | Always SAT (unbounded symbols, i64::MAX) | Type-aware bounds |
| Constant propagation | Debug-string parsing (always None for locals) | `try_eval_scalar_int()` API |
| Reachability | All blocks analyzed | Only reachable blocks from bb0 |
| Panic internals | Not filtered | Skipped |
| Z3 context | Per assertion | Reused per session |
| Module structure | Single 520-line `main.rs` | 9 files — `main` plus verifier, constant_prop, constraint, path_state, reachability, report, types and host_env (toolchain/Z3 discovery, shared with `build.rs`), and a second binary `bin/keller-env` that prints it |
| Z3 path | Hardcoded `C:\z3...` | Configurable `Z3_HOME` |
| Cross-platform | Windows-only MSVC | Windows + Linux + macOS |
| Testing | None | 3 integration test crates + CI |
| Cargo.toml | `cargo.toml` (invalid) | `Cargo.toml` (correct) |

## 5. Integration Tests

The table below names the original twelve fixtures; the full list lives in `tests/cases.txt`.

| Test crate | Expectation | Verifies |
|---|---|---|
| `tests/safe_code` | exit 0 | All operations within bounds → every assertion proven safe |
| `tests/overflow_fail` | exit 1 | Intentional `i32` addition overflow detected, with a witness |
| `tests/divzero_fail` | exit 1 | Intentional division by zero detected, and the **divisor** named |
| `tests/branch_merge` | exit 1 | A constant merged from two branches is not folded to "safe" and skipped |
| `tests/guarded` | exit 1 | Four guarded functions proved and absent from the report; one too-weak guard still flagged |
| `tests/workspace_multi` | exit 1 | Both members of a workspace survive the report merge |
| `tests/loop_fail` | exit 1 | A loop body is checked under its own guards: the counter increment is proved, the accumulator's reachable overflow still flagged |
| `tests/nested_loop` | exit 1 | A nested loop converges: the cycle widening bounds the condition as well as the values, so its guarded increments are proved and only the accumulator's unbounded overflow is reported |
| `tests/shift_amount` | exit 1 | A shift is checked on its amount, and a value-truncating shift is *not* reported |
| `tests/fold_fail` | exit 1 | Six failures the constant-folding pass finds without asking Z3, each still carrying a witness and a re-dischargeable ground obligation |
| `tests/broken_code` | exit 3 | A crate the compiler rejects is a setup failure, not a clean run |
| `tests/undecided` | exit 0 | The one obligation Z3 genuinely cannot decide, reported as a `WARN` rather than proved or refuted — reaching its guarded increment needs `a * b == 2^61 - 1` with both factors above 1, and that number is prime. The same run under `--deny-warn` exits 1 |

All the cases in `tests/cases.txt` — 21 today, 15 of them with committed goldens — run locally through
`tests/run_tests.sh` (Linux/macOS) and `tests/run_tests.cmd` (Windows). Both runners compare each case's
whole `report.json` against its golden and re-discharge every emitted obligation with `z3 -smt2`; the
Windows runner uses `fc /b` and `fc /n` where the shell runner uses `diff -u`, and it checks the same
pinned witnesses and exact shift value. Both take the toolchain and the solver from one implementation —
`src/host_env.rs`, which `build.rs` shares, printed by the `keller-env` helper the same build produces —
so the Z3 the suite re-discharges with is the Z3 the verifier was linked against, resolved as
`KELLER_Z3`, then a `z3` on `PATH`, then `Z3_HOME`, then the project-local `.z3/` install, then the
per-platform default. A missing toolchain is a fatal setup error before the first case runs, rather
than twenty-one cases that each died at load.

`verify.yml` wires the shell runner, the 164 unit tests, the Windows runner and a verifier-on-itself job
into CI, smoke-checks `--version` and `--help`, generates a SARIF log and uploads `report.json` as an
artifact. The workflow is in the tree, but no run of it has been observed from this checkout, so this
paragraph describes what it is set up to do rather than a result.

## 6. Output

- **Console:** live `[PASS]` / `[FAIL]` / `[WARN]` stream per function/block with the block ID (`bb0`, `bb8`, …). Pass lines and the summary go to stdout; findings and diagnostics go to stderr, so a pipeline can read stdout as data (`--json` for machine-readable counts and findings, `--quiet` for a short log).
- **`report.json`:** a bare JSON array of the FAIL and WARN events. Each event names its `crate`, `function`, `block`, `status`, a machine-readable `kind` (`bounds`, `overflow`, `overflow-neg`, `divzero`, `remainder`), the source `file` and `line` when the compiler gave a usable span, free-text `details`, and the concrete `witness` inputs that trigger the FAIL — recorded for every FAIL, whether the obligation went to the solver or was decided by constant folding. A witness entry is either an operand of the failing operation or a value the obligation *pins* (the solver is asked whether any other value satisfies it), so the report names facts rather than the model's choices; a guard's own symbols are left out and the guard stays in the obligation's `; reachable:` comment.
- **`obligations/`:** the SMT-LIB formula behind every FAIL/WARN, plus a pinned `*.witness.smt2` that must re-discharge to `sat`, so a verdict can be re-checked with a stock `z3` rather than trusted. A failure the constant-folding pass decided is emitted there too, in the same two files and with the same `sat` guarantee, but what it states is a **ground restatement** rather than the question Z3 was handed — there was none: the folded operands pinned to the constants they were folded to, plus the violation predicate in the shape the symbolic encoder uses, with the obligation header recording the verdict as `FAIL (folded)`. Folded constants are written in their own type's domain, so a `u128::MAX` operand is reported as `340282366920938463463374607431768211455` and not as the `-1` the constant path carries it in.
- **`--sarif`:** a SARIF 2.1.0 log of the gated findings, for code scanning annotations on a pull request (rule per class, `error`/`warning` level, logical and physical location, `crate::function::block` fingerprint).

## 7. Limitations & Future Work

- **Path conditions:** a symbolic state is threaded through the control-flow graph from the entry block, and each branch guard is folded into the condition required to reach the assertion. `tests/guarded` measures the effect in both directions: with the precondition disabled all six of its obligations are SAT, with it enabled five are UNSAT (proved) and only the deliberately too-weak guard is still reported, with a witness that respects its guard. Merged values stay "one of these" rather than being forced to one predecessor.
- **Loops:** a cycle is unrolled to its back edge, so a loop body is checked under the loop's own guards — which is what proves the counter increment of `while i < steps { i += 1 }` — and the loop-carried values are modelled exactly for a bounded number of trips, past which they are widened to a type-bounded fresh symbol while the guards stay. A widened value is therefore a superset of what the loop can hold, and it is the guards, not the values, that carry a proof across a loop. `tests/loop_fail` exercises both halves end to end, and the same protection extends to a loop body's *condition*, not only its values: past the unrolling bound a cycle block's condition is rebuilt from its forward edges, so a back edge cannot carry the block's own condition back into itself and make it grow without end. That is what `tests/nested_loop` pins — a loop inside a loop whose guarded increments (`inner + 1` under `inner < cap`, `outer + 1` under `outer < limit`) are proved like a single loop's, with only the accumulator no guard bounds left as a counterexample.
- **Shifts:** modelled exactly, and only on the *amount*: `<<`/`>>` panic when the amount is negative or at least the operand's bit width, while a value truncated out of range is not a panic and is not reported as one.
- **Division:** modelled as unbounded integers, so a quotient value is not propagated (Z3's `div` rounds differently from Rust's truncating division); the divisor, the remainder's divisor and the `MIN / -1` overflow are checked.
- **Build mode:** the assertions verified are the ones rustc emits for a **debug** build. A release build wraps instead of panicking, so "cannot overflow" and "will not abort" are different claims.
- **Performance:** SMT solving adds overhead to compilation time. `--solver-timeout <MS>` bounds each obligation, at the cost of an undecided `WARN` when the budget runs out.
- **Spec coverage:** only the runtime assertions the compiler itself emits (bounds, overflow, division and remainder by zero). `unwrap`/`expect` on a known variant are decided, but explicit `panic!`, unknown-variant `unwrap`/`expect` and application invariants are not modelled. Function contracts, inductive invariants for loops and integration with the wider security toolchain remain aspirational.

## 8. Requirements

- Rust **nightly** with `#!Afeature(rustc_private)]`, matching the pin in `rust-toolchain.toml` (`nightly-2026-07-24`). Hosts verified in practice: `x86_64-pc-windows-gnu` and the Linux CI runner; the `-msvc` host needs the MSVC linker, which the machine this was developed on did not have.
- **rustc-dev** nightly component (for `rustc_driver` linking), plus `llvm-tools-preview`.
- **Z3 Prover** v4.16.0+, found through `Z3_HOME`, then a project-local `.z3/` install, then the per-platform default (`C:\z3-4.16.0-x64-win`, `/usr`, `/usr/local`).