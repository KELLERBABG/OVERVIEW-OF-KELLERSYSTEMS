# Security Policy

## What counts as a security bug here

This project is a verifier, so its output is a claim about a program. The dangerous bug is therefore
not a crash and not a false alarm — it is a **claim that is wrong in the reassuring direction**:

- **A wrong PASS.** The tool reports an obligation as proved, or omits it from `report.json`, when
  the assertion can in fact fail. A developer reads a clean report as permission to ship.
- **A silently skipped check.** An obligation the tool did not examine must never disappear from the
  report. If a class of assertion is dropped without being reported as a `WARN`, the report is
  misleading even though nothing in it is false.
- **A witness that does not reproduce.** A FAIL names a concrete input; if that input does not
  actually fail the assertion, every downstream decision based on it is built on sand.
- **A gate that can be switched off silently.** A misspelled or ignored flag, or a baseline that
  fails to load and is treated as empty, must fail loudly rather than weakening the gate.

A **false FAIL** is a bug too, and a serious one — a tool that cries wolf gets turned off — but it is
reported here as a correctness bug rather than a security one, because it fails in the safe
direction.

One instance of a **wrong PASS** has been found and fixed. A crate the compiler rejected was reported
as a clean run: the wrapper discarded the status `rustc_driver::catch_with_exit_code` returned, so
`keller verify` exited `0` with an empty `report.json` for code that does not compile, because the
analysis had found no assertions in a body that was never built. It exits `3` ("the analysis could
not run") now, and `tests/broken_code` pins that. The wrapper is also the one place the tool can
disagree with a plain `cargo check`: reading `optimized_mir` for every function runs MIR passes
`cargo check` skips, so a deny-by-default lint such as `arithmetic_overflow` is raised for code
`cargo check` accepts and `cargo build` rejects. The wrapper follows the build.

## Reporting

Open an issue describing the program you verified, the verdict the tool gave, and what you expected.
The most useful reports are:

- the source of the function that was misjudged,
- the emitted obligation from `obligations/<crate>/<function>_<block>.smt2`, since it is the exact
  question that was asked, and
- what `z3 -smt2` says when you re-discharge it.

Every FAIL's obligation can be re-checked independently — that is the point of emitting it — and the
pinned `*.witness.smt2` file must re-discharge to `sat`. If it answers `unsat`, the reported witness
does not fail the assertion, and that is a report worth filing.

That now covers every FAIL, including one the constant-folding pass decided without a query. A
folded failure carries a witness like any other, and its obligation — a ground restatement of the
pinned operands and the violation predicate, since there was no formula to hand Z3 — is marked
`FAIL (folded)` in its header and must also replay to `sat`.

## In scope

The encoder (what formula an assertion turns into), the path-sensitive state (which guards are part
of that formula), the witness extraction (which inputs are read back out of a model), the obligation
and report emission, and the exit-code gate. A mistake in any of those can produce a wrong PASS.

## Out of scope, by design

- **The program being verified.** Code that is reported as failing is not this tool's bug.
- **A debug/release mismatch.** The report describes the assertions rustc emits for a **debug** build.
  A release build wraps instead of panicking, so "this cannot overflow" and "this will not abort"
  are different claims; using the report for the second one is out of scope for the current
  encoding.
- **What is not modelled at all.** `unwrap`, `expect`, explicit `panic!`, allocation failure and
  application invariants are never checked. An empty report does not mean "this program cannot
  panic"; it means none of the checks this tool performs can fire. The same caveat applies to values
  that pass through unmodelled operations (integer bit-twiddling, narrowing casts) — they become free
  symbols bounded by their type, which makes downstream verdicts sound but not complete.
- **Third-party dependencies.** Only the crate under verification is analysed.
