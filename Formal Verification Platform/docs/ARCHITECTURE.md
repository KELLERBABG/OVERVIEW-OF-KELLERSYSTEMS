# Architecture and Trust Notes

How a verdict is produced, what makes it checkable by someone who does not trust this binary, and
where the reasoning stops being valid.

This is the document that answers "why should I believe the report?". It is written so that every
assumption appears as a *limitation* rather than a guarantee: the useful claim a verifier can make is
"here is exactly what I checked, and here is what I did not" — see
[../SECURITY.md](../SECURITY.md) for what counts as a bug here.

## 1. The pipeline

```
keller verify
  └── cargo check, with RUSTC_WRAPPER set to this same binary
        └── one process per crate: rustc_driver's after_analysis callback
              ├── mirror of the crate's MIR, function by function
              │     ├── which blocks are reachable from bb0
              │     ├── constant propagation over locals
              │     └── a path-sensitive symbolic state over the whole CFG
              ├── for each `Assert` terminator: encode the condition under which
              │   it fails, and ask Z3
              ├── verdict, then a witness read back out of the model on SAT
              ├── obligations/<crate>/*.smt2 and a per-crate report file
              └── report.json for the crate (FAIL and WARN only)
  └── the top-level process merges every crate's report into one report.json, and
        records the toolchain and the Z3 it ran under beside it (report.environment.json)
        └── the gate: exit 1 on a counterexample, or on a WARN under --deny-warn
```

Three details of that shape matter more than the boxes:

- **The worker is the compiler.** The wrapper runs `rustc` itself and returns its status, so a crate
  the compiler rejects is a setup failure (`3`), never a clean run. It also means reading
  `optimized_mir` runs MIR passes a plain `cargo check` skips, which is why the wrapper can raise a
  deny-by-default lint like `arithmetic_overflow` for code `cargo check` accepts and `cargo build`
  rejects. The wrapper follows the build.
- **One report per crate, merged by the parent.** Cargo compiles crates in parallel, so each worker
  writes its own file under a per-crate directory and the parent merges them. A workspace's verdict
  is a property of the merge, not of whichever crate finished last.
- **Constant folding decides some obligations without Z3.** When both operands of a check are
  constants, the answer does not need a solver. It is still a FAIL with a witness and an obligation —
  see below — but no model was involved in reaching it.

## 2. What makes a verdict checkable

Nothing here asks to be believed on the strength of the binary. The artefacts are the argument:

- **`obligations/<crate>/<function>_<block>.smt2`** is the exact question that was asked, in SMT-LIB,
  re-runnable with a stock `z3 -smt2`. It is the solver's own rendering of its declarations and
  assertions, not a second printer that could drift from it.
- **`obligations/<crate>/<function>_<block>.witness.smt2`** is the same formula with the reported
  inputs asserted as equalities. It must answer `sat`. A witness that does not reproduce is a report
  worth filing ([../SECURITY.md](../SECURITY.md)), and that file is what makes it checkable.
- **`obligations/<crate>/index.json`** lists every obligation the run emitted for that crate — proofs
  included, not only the findings — with the status, kind, `file:line` and artifact name. It is sorted
  and de-duplicated, so two runs of unchanged code produce byte-identical bytes. A `PASS` writes its
  own `<function>_<block>.smt2` and is listed there too.
- **`keller explain <FILE>` / `keller replay <FILE>`** read those files: `explain` describes the
  obligation, and `replay` re-runs Z3 and rejects output that disagrees with the header, is `unknown`
  or contains an SMT error.
- **What a witness is allowed to claim.** The reported values are the operands of the operation that
  would panic, plus the state values the obligation *pins*: the solver is asked whether any other
  value satisfies the same obligation, and one the guard merely bounds — a loop counter under
  `i < steps` — is left out, because the model chose it rather than the program forcing it. The guard
  that bounds the dropped values is still carried as a formula, in the obligation's own
  `; reachable:` line, so what a reader needs to interpret the report is checkable rather than a
  number the solver happened to pick. A *relational* failure such as `a + b` pins neither operand, so
  both are reported anyway: a report has to say which operation fails, and with what values.
- **`report.json`** names the source file, line and obligation class of every finding, so a reader can
  go to the code rather than search for the function.
- **A constant-folded failure** gets both files as well, but what they contain is a *ground
  restatement*: the folded operands pinned to the constants they were folded to, plus the violation
  predicate in the shape the symbolic encoder uses, with the header recording the verdict as
  `FAIL (folded)`. There was no formula handed to Z3, and the file does not pretend there was.

The honest boundary: a PASS is a proof **over the encoding**, not over Rust's own semantics. If the
encoder maps an assertion to the wrong formula, the obligation will re-discharge perfectly and still
be wrong. That is why the encoder is the trusted core, why it is unit-tested against Z3's own verdict
on the same operand triples, and why [../SECURITY.md](../SECURITY.md) treats a wrong PASS — not a
crash, not a false alarm — as the security-relevant bug class. Re-discharge checks the formula; it
cannot check the encoding's fidelity to the language. Neither can a proof object, for that matter:
what it would add is a smaller thing to trust.

## 3. Assumptions, and where they can break

Each of these is a limitation, not a caveat added for modesty. Every one of them is a way to read the
report as saying more than it says.

- **The verdict describes a debug build.** The assertions analysed are the ones rustc emits for
  `debug`. A release build wraps instead of panicking, so "this cannot overflow" and "this will not
  abort" are different claims, and only the first one is being made.
- **Guards are threaded; a loop's carried values are widened past a bound.** A branch guard is
  folded into the condition required to reach an assertion, so an assertion a branch already rules
  out is *proved*. A cycle is unrolled to its back edge, so a loop body is asked its question under
  the loop's own guards — which is what proves an increment guarded by `i < steps` — and past
  `MAX_LOOP_TRIPS` trips the loop-carried *values* are widened: a forgotten value comes back on the
  next read as a fresh symbol bounded by the local's type. Widening only weakens the precondition,
  so it can turn a PASS into a FAIL and never the reverse, and the guards survive it because they
  are facts about the program's control flow rather than about one trip. The cost is precision in the
  value dimension: past the bound a loop body's arithmetic is decided with values that are free
  within their type, so inside a loop it is the guard doing the work and not the value.
- **Division/remainder follow Rust's signed rounding rule.** Divide nonnegative magnitudes,
  restore the quotient sign, and derive remainder as `a - quotient * b`. Compiler success-edge
  guards exclude zero divisors and signed MIN/-1 overflow. Valid signed i8 pairs are checked
  exhaustively against native Rust; derived arithmetic is also tested through the CLI.
- **Bit-twiddling and narrowing casts are unmodelled.** `&`, `|`, `^`, wrapping and unchecked
  operators, and casts that lose bits all produce free bounded symbols. Again sound, again not
  complete.
- **Only the assertions the compiler emits are checked.** Five `AssertKind`s are turned into
  obligations (bounds, overflow, negation, division and remainder by zero), and a panic call site is
  decided when its path guard is unsatisfiable or an `unwrap`/`expect` receiver's enum variant is
  known. Other reachable kinds produce `unsupported-assertion` WARNs, and a panic whose variant is not
  known is disclosed as an `unmodelled-panic`. This still does not cover everything above that layer:
  explicit `panic!`, allocation failure and application invariants. A completed empty report means no
  findings in the selected encoded checks, not "this program cannot panic".
- **There is no cross-function reasoning.** A callee's precondition is not used at the call site, so
  contracts are not verified; an `assert!` inside a body is checked against that body's own reachable
  states.
- **A failed compile is a setup failure, not a verdict.** Exit `3` means no verdict was reached. It
  must never be read as a pass, and no `report.json` from a previous run should be read either — the
  run clears stale artefacts before it starts.
- **Suppression is not deletion.** A baseline removes findings from the *gate*; `report.json` keeps
  every finding, so the record of what is known is not silently shortened.
- **Coverage is the recorded compiled-target scope.** Worker receipts identify which crates
  actually completed callbacks; do not infer an unrecorded dependency or target was verified.
- **Run completion is evidence, not absence of files.** Every worker writes start/completion
  receipts, even with no findings. Incomplete/corrupt receipt sets and required artifact I/O errors
  fail closed. A private fresh target directory prevents Cargo's cached builds from skipping
  analysis after receipt reset; it rebuilds dependencies and is not a performance optimization.
  Unchanged function verdicts may be restored from a content-addressed cache, which must carry the
  same events, counters and obligation bytes as a fresh analysis; a mismatch is treated as a miss.
- **Counterexamples are model-relative.** Widening and unmodelled values may admit non-executable
  witnesses. Replay rechecks the SMT formula, not a concrete execution. Per-value abstraction
  provenance and a fixed-width semantics audit remain future work.
- **Provenance comes from workers.** `report.run.json` carries actual compiler paths, verbose
  versions, arguments and coverage; environment discovery alone is not evidence of the loaded
  solver library. `--strict-coverage` refuses encountered gaps even if a baseline lists them.

Where a limitation above is a caught-up-with item, it carries a status and a named
next step. [WHITEPAPER.md](WHITEPAPER.md) covers the encoding and the fixture suite;
[USAGE.md](USAGE.md) covers the flags, the exit codes and the baseline rollout;
[../tests/README.md](../tests/README.md) describes each fixture and what it pins down.
