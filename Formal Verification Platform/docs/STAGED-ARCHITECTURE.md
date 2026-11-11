# Staged verification architecture

## Acceptance status

- [x] Every invocation owns `.keller/runs/<run-id>/`, retaining findings, run/environment records, receipts, and obligations independently.
- [x] `--json` identifies the authoritative run. Legacy root artifacts are a serialized compatibility view, published with the completion record last.
- [x] Concurrent runs cannot clear each other's receipts. Shared Cargo-target use is locked; each run gets a unique workspace-wrapper path so workspace analysis remains fresh. Dependencies may reuse Cargo artifacts, and unchanged function verdicts are restored from a content-addressed cache unless `--no-cache` is given; a hit whose counters or obligation artifacts are incomplete falls back to a fresh analysis.
- [x] `--no-cache` provides a cold reference. Dependency compilation, identical findings, fresh workspace workers, and wall time were measured on a local-dependency workspace.
- [x] Fixed-width integer propagation, bitwise and shift values, casts, and wrapping plain arithmetic have regression/differential coverage. Checked overflow predicates retain mathematical results; unchecked validity stays unmodelled.
- [x] Exact unsigned 128-bit literals and projected-place handling have regressions. Cross-width targets fail closed until target-aware pointer modelling exists.
- [x] Run evidence distinguishes model-relative candidates from native reproduction and attaches conservative abstraction disclosures. Native reproduction is always `not-attempted` today.
- [x] The existing ten findings goldens remained unchanged in the fixture suite. (Written when
there were ten: the number is fifteen as of 2026-11-05 — `panic_sites` and `panic_decided` were added
since, and a later pass refreshed two earlier ones, one `defect_class` field each — while every
pre-existing verdict, witness and line stayed put.)

## Cache and concurrency measurements

The CLI probes use a fresh temporary package and a local dependency containing 1,500 functions. Wall times include process startup, fresh workspace analysis, and evidence publication; they are machine-specific observations, not a performance guarantee. Cargo output confirms whether the dependency was compiled, and all three findings arrays were byte-identical. Every pass analyzed one workspace worker. The measurement artifact records the most recent repeated probe; timing varies with machine state:

| Mode | Wall time | Dependency compiled | Workspace workers |
|---|---:|---:|---:|
| Cold (`--no-cache`) | 4.289 s | yes | 1 |
| Cache fill | 2.154 s | yes | 1 |
| Warm | 1.185 s | no | 1 |

This establishes dependency-artifact reuse, not that every warm run is faster than every cold run. The analysis wrapper is unique per run, so workspace crates are rechecked; only Cargo's normal dependency compilation artifacts are shared. `--no-cache` isolates its Cargo target under the run directory and removes that target when finished.

Two concurrent failing invocations in one output directory both completed with exit 1. They retained distinct completed evidence directories; the compatibility view matched one complete run. This validates run isolation and publication consistency, not a throughput improvement.

## Verification results and limitations

**The counts below are the staged-architecture pass as recorded, not the current tree.** The current
green run, with both fixture runners and the cache-enabled trust gate, is recorded in
[TRUST-CHECKLIST.md](TRUST-CHECKLIST.md).

- Final `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo build` all exited 0. `cargo test` ran 17 `keller-env` tests and 101 verifier tests (118 passed total). One dead-code warning remains for path-state helper methods.
- `tests/staged_checks.py`: six semantic, concurrency, evidence, and dependency-cache probes passed after adding the pinned toolchain's rustc driver DLL directories to `PATH` through `target/debug/keller-env.exe`. Without that setup, the Windows executable failed to load with `3221225781`; this is an environment/runtime prerequisite, not a test pass. In this Windows environment, invoke Python as `py -3` (`python` and `python3` resolve to Store aliases).
- `tests/trust_checks.py`: all eight trust probes passed with the same runtime environment setup, covering unchanged clean/failing runs, evidence replay, baseline/strict coverage behavior, publication failure retention, compiler flags, and workspace aggregation.
- The full POSIX fixture, golden, witness-replay and trust checks passed. The landing page is hand-written now, so there is no generated-page step and no `UPDATE_INDEX`; `tests/run_tests.cmd` has since been run green on the same bytes.
- SMT replay confirms that the recorded formula/model is satisfiable; it does not establish that the caller can reproduce the behavior in native Rust. There is no concrete counterexample execution harness or general soundness certificate. Heap behavior, some calls, unchecked-operation validity, and cross-width targets are outside the supported model; bounded loop widening can lose precision.
- User contracts and inductive loop invariants are not implemented. They must not affect gates until preconditions become call-site obligations; postconditions are checked on normal returns; recursion/cycles cannot rely on unproved summaries; invariants satisfy initiation, preservation, and exit checks; termination is kept distinct from partial correctness; and inconsistent assumptions/vacuity are rejected or reported. Negative fixtures must cover false contracts, violated preconditions, false/insufficient invariants, recursion, and vacuity.

## Broader correctness and verification roadmap

Treat verification as separate claims with separate evidence; a green build or an SMT `sat` result must not be promoted into a stronger claim:

1. **Build identity and analysis freshness (implemented, tested):** record toolchain/compiler flags, create a unique workspace wrapper per invocation, and retain worker start/completion receipts. Cache a function verdict only by content (MIR structure and contracts), restore it together with its counters and exact obligation artifacts, and treat an incomplete entry as a miss; `--no-cache` supplies an isolated cold build.
2. **Concurrency and evidence integrity (implemented, probed):** isolate run roots, serialize shared-target use and compatibility publication, publish the completion record last, retain evidence on publication errors, and prune only completed inactive runs. Existing parallel-run probes verify two invocations preserve distinct full run evidence while the legacy root view points at one completed run.
3. **Semantics and analyzer validation (partial):** fixed-width folding and symbolic values have differential tests against native Rust for enumerated small integer domains; branches, some arithmetic, casts, shifts, and pointer-width mismatch fail-closed handling have targeted regressions. These do not prove soundness for all MIR constructs, integer types, targets, panic profiles, or compiler versions. Expand differential/property-based testing across all widths/operators, checked and unchecked modes, signed division corner cases, casts, shifts, and optimization levels; keep a list of constructs that are explicitly unsupported and make coverage report them.
4. **Counterexample evidence (model-only):** `explain` and `replay` validate the emitted SMT obligation and pinned model. Add a separately opt-in concrete Rust harness that pins source/build identity, reconstructs caller inputs, preserves compilation/panic/overflow flags, runs under a deadline, identifies the same failing site, and distinguishes reproduced, not reproduced, unsupported, and execution error. A different panic or a SAT model alone is not a native reproduction.
5. **Contracts and invariants (not implemented):** preconditions become call-site obligations; postconditions are checked on normal returns; recursive/cyclic calls cannot use unproved summaries; loop invariants require initiation, preservation, and exit checks. Keep termination distinct from partial correctness. Reject or report inconsistent assumptions/vacuity, and add negative tests for false contracts, violated preconditions, false/insufficient invariants, recursion, and vacuity before such annotations can influence a gate.
6. **Soundness and model boundaries (ongoing):** document every abstraction and unsupported operation; use sound over-approximations, fail closed when representation is unavailable, and report precision loss rather than silently claiming a proof. Heap/aliasing, calls, unwind behavior, unchecked validity, target modeling, bounded loop widening, and general soundness certification still need dedicated designs and adversarial tests.
7. **Independent solver/checker evidence (future):** keep the standalone Z3 replay path and add versioned solver/toolchain provenance. Explore proof-producing solver output or an independent certificate checker; solver agreement on the same encoding is useful but does not certify that MIR-to-SMT translation is correct.
8. **Reproducible integration baseline (partially verified):** both fixture runners have passed on the same bytes, and the landing page is hand-written so it is no longer part of the check. The Windows fixture runner still needs execution in CI, and future runner changes should keep environment setup failures and stale goldens as distinct outcomes.

## Results

The staged run/cache and selected integer-semantics behavior is exercised by tests, but these results do not certify the verifier's general soundness. Model satisfiability, native execution, and a proof of the analyzer's own abstraction are separate evidence levels. Contracts, invariants, concrete replay, heap modelling, comprehensive integer-semantic validation, complete golden/manifest consistency, and solver certificates remain milestones.

## Concrete counterexample validation milestone

A native harness must pin source/build identity, reproduce caller inputs (not merely MIR operands), preserve panic/overflow settings, enforce a deadline, identify the relevant failing site, and report `reproduced`, `not-reproduced`, `unsupported`, or `execution-error`. A different panic is not confirmation. Do not execute arbitrary project code silently during ordinary SMT replay; retain SMT results separately when execution is unavailable or disagrees.
