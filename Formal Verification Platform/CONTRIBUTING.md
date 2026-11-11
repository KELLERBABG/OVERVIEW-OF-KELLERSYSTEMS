# Contributing

This is a verifier, so the bar for a change is different from a normal tool's: its
output is a *claim about someone else's program*. A bug in the safe direction is a
false alarm; a bug in the reassuring direction is a report that says a crate is
fine when it is not. [SECURITY.md](SECURITY.md) states which of those this project
treats as a security bug, and the one instance of the second kind that has been
found. Read it before changing anything that decides a verdict.

## Getting a tree that can run

```sh
rustup toolchain install nightly-2026-07-24 --component rustc-dev --component llvm-tools-preview
cargo build            # links against Z3; see build.rs and README.md for Z3_HOME / .z3
cargo test             # the inline unit tests
tests/run_tests.sh     # the fixture suite, through the real binary (Windows: tests\run_tests.cmd)
```

`rust-toolchain.toml` pins the channel because the verifier is built on
`rustc_private` internals; `rustc-dev` is what makes `#![feature(rustc_private)]`
compile. Z3 is found through `Z3_HOME` or a release unpacked under `.z3/`, and both
the build and the fixture suite ask the same helper (`keller-env`) rather than each
carrying their own lookup — a unit test fails if either runner grows a second copy
of that rule.

Do not add the pinned toolchain's *uninstall* as a way to test something: the
missing-toolchain branch is covered by unit tests, and this checkout cannot remove
the toolchain it is running on.

## What a change has to leave behind

* **Run the checks, and report what they said.** `cargo fmt --all --check`,
  `cargo test`, and the fixture suite for the platform you are on.
  `scripts/lint-gate.sh` is the lint gate (D3): it denies `clippy::correctness` and
  `clippy::suspicious`, and fails if the style-warning count exceeds its ceiling.
  Clearing those warnings is a job of its own; raising the ceiling to get a change
  through is not.
* **A changed verdict is a changed golden, and the diff is the review.** The suite
  compares each case's whole `report.json` against `tests/goldens/`. After an
  intended change, re-record with `UPDATE_GOLDENS=1 tests/run_tests.sh` and read
  the diff — it is the only place the fields nobody thought to assert are guarded.
* **Both runners, or neither.** `tests/run_tests.sh` and `tests/run_tests.cmd` are
  kept equivalent: a new case is added to both, and `case_dir`/the third argument
  of `:run_case` is how a fixture outside `tests/` is reached.
* **A new fixture is asserted in both directions.** A case that passes because the
  tool reports *nothing* is not a case: pair every "this is reported" with the
  control that has to be proved beside it.
* **Never silence an obligation.** An assertion the build cannot model is a `WARN`
  with a kind, never a silent `continue`. `coverage.unsupported_kinds` is where the
  shortlist of what to model next is collected, and a class that disappears without
  being reported is the failure mode [SECURITY.md](SECURITY.md) describes.
* **Every FAIL names an input and leaves a re-checkable obligation.** A witness in
  `report.json` plus `obligations/<crate>/<fn>_<bb>.witness.smt2`, which must
  re-discharge to `sat` under the standalone solver. The rule holds for failures
  the constant-folding pass reaches without asking Z3 as well.
* **Say what was checked.** The verdict describes a debug build, because that is
  where rustc emits the assertions this tool reads; every report states that
  (`analysis_profile`) rather than leaving a reader to assume the release profile.
* **A claim in the docs that the code does not implement is a bug.** If you add a
  number, a flag or a guarantee to `README.md`, `docs/` or `index.html`, wire it to
  something that fails when it stops being true, or leave it out.

## Where work is recorded

Work status and release history are kept in maintainer notes outside this
repository. Each item keeps the id it was written with (`K6`, `A14`, `D8`, …), so
a source comment can still cite one. Do not start a list file for them in the
tree.

When you close an item, say what changed the answer. When you find a
contradiction between a list and the tree, fix the list: two truth claims in one
repository is the state this rule exists to end.

## Adopting the tool in a repository

`keller init` writes a starter `keller.toml` whose every setting is commented out,
because an initialiser that starts gating on its first run looks broken in the one
place a new user tries it. The file supplies *defaults*: anything typed on the
command line still wins, including a `--profile` that loosens what the file set.
Unknown keys are errors rather than silent defaults, so a typo cannot leave the
gate looser than it looks. Per-crate `[[policy]]` tables are how a repository
records which crates are gated and which are advisory.

## Releases and reporting

Packaging, the supported matrix and the installer are in [docs/RELEASE.md](docs/RELEASE.md).
For anything that looks like a wrong verdict — a proof for code that can fail, a
skipped assertion, a witness that does not reproduce — open an issue with the
`report.json`, the obligation file and the `report.environment.json` from the run,
rather than only the console output: the two sidecars are what make the claim
checkable on another machine.
