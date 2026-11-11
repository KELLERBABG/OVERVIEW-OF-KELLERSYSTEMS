# Trustworthy, actionable verification — task checklist

## Execute now: trust foundation

- [x] Every real compiler worker writes a versioned start receipt and a completion receipt, including clean crates; incomplete/malformed/missing receipts fail with setup exit 3.
- [x] Repeat verification of unchanged code cannot let Cargo's cache turn an earlier failure into a clean verdict. Unchanged function verdicts are restored only from a content-addressed cache that carries the failure, its counters and its exact obligation artifacts; an entry that disagrees with its event count is re-analysed. Preserve existing compiler flags.
- [x] Resolve worker artifacts against the invocation root, including workspace members compiled from different working directories.
- [x] Fail on report preparation/read/write errors; successful clean runs write `report.json` as `[]`; preserve the bare-array findings schema and existing golden bytes.
- [x] Aggregate functions, reachable assertions, supported/unsupported assertions, PASS/FAIL/WARN results and solver-call counts into a versioned `report.run.json` and `--json` coverage field.
- [x] Unsupported reachable MIR assertion kinds produce an explicit `unsupported-assertion` WARN, never silence; `--strict-coverage` blocks gaps even when a baseline suppresses them.
- [x] State that a SAT witness satisfies the encoded model, not necessarily a concrete execution; publish model limitations and effective analysis settings.
- [x] Encode Rust division truncating toward zero and remainder with the dividend's sign; verify against native Rust across signed 8-bit inputs.
- [x] Differentially test constant overflow and SMT predicates against Rust checked arithmetic, including signed/unsigned 8-bit domains and shift boundaries.
- [x] Provide `keller explain <FILE>` and `keller replay <FILE>` for emitted obligations; replay compares the recorded verdict with fresh Z3 output and rejects errors/unknown/mismatches.
- [x] Exercise clean/failing unchanged reruns, workspace aggregation, unsupported assertion gates, baseline interaction, incomplete/corrupt receipts and unwritable outputs.
- [x] Run `cargo check`, unit tests, both fixture runners where available, and CLI replay/explain checks; record actual results, failures and limitations below.

## Subsequent milestones — not claimed by the 2026-10 trust-foundation pass

- [ ] Exact fixed-width casts, bitwise operations, wrapping arithmetic and shift results, with differential tests. Audit current unbounded arithmetic propagation before expanding supported claims.
- [ ] Explicit debug/release/target selection and evidence tying each worker's effective compiler configuration to the report.
- [ ] Track abstraction provenance per value/finding; distinguish potential counterexamples from concretely reproduced failures; generate executable reproductions where supported.
- [ ] Export successful obligations and bind findings, obligations, settings and tool versions with stable content hashes; verify linked rather than merely discovered solver identity.
- [ ] Precise assertion-site spans, stable source-based fingerprints and machine-readable diagnostic output.
- [ ] Representative timing/solver benchmarks before adding verified incremental caching, filters or watch mode; no speedup claim without measurements.
- [ ] Function contracts, call-site obligations and inductive loop invariants, including initiation/preservation/exit checks and negative fixtures.
- [ ] Cross-platform CI and a working formatting/lint gate; normalize line endings intentionally rather than suppressing checks.

## Verification results

Verified 2026-10-09 on Windows, against the final formatted source files. **These are the
2026-10-09 numbers and are left as the record of that run.** Four days later the same commands give
143 unit tests (19 + 124 — the verifier's own count last moved with the `--run-timeout` cap's parse
test) and `tests/run_tests.sh` exit 0 / 0 FAIL / 0 SKIP (243 PASS lines: 235 while the landing page was
still an artefact of the run, 234 after it stopped being one, 243 once the wall-clock cap's nine checks
landed) with **thirteen** goldens (two of them refreshed in a later pass, in one `defect_class` field
each); the Windows runner's row below is the 2026-10-09 run and has since been re-run — the 2026-10-13
paragraph below carries that result — and the A19/A20 entries and the release notes of
2026-10-13 carry what changed in between.

Run again on 2026-10-13, after the pass that built the open `keller.toml`/`keller init`, obligations
manifest, `witness_kind` and profile-statement rows and wired the three fixtures nothing had run into
both runners: **`cargo fmt --all --check` exit 0** repository-wide (the scoped six-file check in the
table below was the 2026-10-09 state, and the repository-wide row no longer describes this tree),
**`cargo test` 155 passed, 0 failed** (20 `keller-env` + 135 verifier — one more in each, because
`src/host_env.rs` is compiled into both binaries and that is where the two runner checks live),
**`tests/run_tests.sh` exit 0, 283 PASS lines, 0 FAIL, 0 SKIP** over the nineteen fixtures in
`tests/cases.txt`, and **`tests\run_tests.cmd` exit 0, 134 PASS lines, 0 FAIL, 0 SKIP** on the same
bytes - 85 before it ran the three fixtures it had no block for, and a printed **1** with
`[FAIL] safe_code: expected exit 1, got 0` when a row's expected exit was deliberately flipped to prove
that path, the edit reverted to the same SHA-256 before the green run. The shell count
moved for three reasons: the three newly wired cases, the parameterised checks they bring, and the
twelve goldens re-recorded — that refresh is 34 added fields in nine files (seventeen `defect_class`,
seventeen `witness_kind`) and nothing else. The Windows runner's row below is the 2026-10-09 number:
65 was its count before the three fixtures were wired in, and it was not green on them until the two
repairs recorded below.

Run once more on 2026-10-13, after the pass that stopped dropping the panics this analysis does not
read (`unmodelled-panic` sites, counted in `coverage.panic_sites`): **`cargo fmt --all --check` exit
0**, **`cargo test` 156 passed, 0 failed** (20 `keller-env` + 136 verifier - the classifier that
decides which call sites those are is a pure function now, with a unit test built from the strings
this compiler actually emits), **`tests/run_tests.sh` exit 0, 308 PASS lines, 0 FAIL, 0 SKIP** over
the twenty fixtures in `tests/cases.txt`, and **`tests\run_tests.cmd` exit 0, 156 PASS lines, 0 FAIL,
0 SKIP** on the same bytes, with **fourteen** goldens. Three of the re-recorded goldens are the new
disclosure appearing where it should have been all along (`overflow_fail`, `divzero_fail`,
`workspace_multi` - each one `env::args().next().unwrap()` in `main`), one is the new
`tests/panic_sites` case, and the published K7 figures are unchanged (`2/18 = 0.111`) because a
panic site is deliberately not counted in that table's `undecided` column.

Run again on 2026-11-05, after the decided-panic, coverage-headline, obligation-manifest and
release-packaging pass: **`cargo fmt --all --check` exit 0**; **`cargo test` 164 passed, 0 failed**;
**`tests/run_tests.sh` exit 0** and **`tests\run_tests.cmd` exit 0** on the same bytes, both with
`KELLER_NO_CACHE=1`, over the fixtures in `tests/cases.txt` (now including `panic_decided`); and
**`tests/trust_checks.py` 8/8 with the cache enabled**, including `explain`/`replay` on a cached
witness. Fifteen goldens. The release workflow and packagers are in the tree, but no run of
`release.yml` has been observed; only the Windows packager was run locally, and its staged launcher
printed the GNU compiler identity and exited 0.

Audited 2026-11-06 (K20): every number this file and the other docs quote was checked against a run
rather than carried forward. `cargo test` **190 passed, 0 failed** (21 `keller-env` + 169 verifier; the
three provenance tests were added later the same day by P1.4, at 187 when this audit began);
`tests\run_tests.cmd` **exit 0, 219 PASS lines, 0 FAIL, 0 SKIP** over the **25** rows of
`tests/cases.txt` (23 analysed, plus `broken_code` and `missing_toolchain`, which exit 3 before
analysis); **fifteen** goldens; `cargo fmt --all --check` exit 0; the lint gate
`[PASS] no correctness/suspicious denials, 79 style warning(s) (ceiling 90)`; Z3 `4.16.0 - 64 bit`
from the bundled binary; version `1.0.0`. The shell runner and the Ubuntu clean-install job were not
run from this Windows checkout, so their numbers here remain the dated records below, not this audit's.
The audit's one real finding was a stale default: `--run-timeout` is `0` (disabled) since 2026-11-04,
while README, `docs/USAGE.md` and the constant's own doc comment still said 1800 s. All three now match
the code and `keller init`'s template.

| Check | Result |
|---|---|
| `cargo check` / `cargo build` | Exit 0; existing dead-code warning for three path-state methods remains |
| `cargo test` | 99 verifier tests + 17 environment-helper tests passed; 0 failed |
| `bash tests/run_tests.sh` | 140 PASS lines, exit 0, including eight CLI trust regression groups |
| Windows `tests/run_tests.cmd` | 65 PASS lines, exit 0 |
| Golden findings reports | All ten matched byte-for-byte; no golden refresh (as of 2026-10-09 — thirteen by 2026-10-13, and two of those were refreshed for a `defect_class` field each) |
| Generated landing page | Regenerated for explicit unsupported checks, then passed normal check mode |
| Scoped rustfmt check (six edited Rust files) | Exit 0 |
| Repository-wide `cargo fmt --check` | Exit 1: broader existing formatting differences in files outside this change; not merely CRLF |

Failures found and repaired: JSON stdout banner pollution; compiler discovery disagreeing with worker identity; Windows Python decoding Rust's Unicode output using cp1252; gate probes deleting the findings file paired with a captured measurement. The affected checks were rerun after each repair.

The same pass closed the runner drift: the case list, its exit codes and its goldens are one file,
`tests/cases.txt`, that both runners read and check at both ends, so the Windows runner's ten goldens
against the shell runner's thirteen cannot happen again without a named failure. Proving the Windows
failure path red was part of it, and the first attempt was informative in the way these things are: the
solver is invoked with a quoted path, this checkout's path has a space in it, and `for /f` split it -
the obligation re-discharge read `Das System kann den angegebenen Pfad nicht finden`. The answer is now
taken from a file with `set /p`, which is not affected by the quoting.

Wiring the three fixtures on 2026-10-13 added two more. A case whose name is a path was looked for under `tests\` by the Windows runner, which reported `test-crates/safe-project: expected exit 0, got 3` for a crate that was never analysed; `:case_dir` and the shell runner's `case_dir()` now decide that the same way. And a `cmd` parenthesised block ends at the first `)` its parser meets, so `echo [FAIL] … produced %N% finding(s)` closed the block early and the `set "FAILED=1"` beneath it was no longer inside its condition: the runner printed every check as passing, printed no `[FAIL]` at all, and exited 1. Both were fixed at the level they occurred — the second is now asserted on the script's text by `host_env::tests::no_echo_inside_a_cmd_block_carries_a_bare_parenthesis`, and the rule it checks was already stated in the script's own comment. The Windows runner is exit 0, 85 PASS, 0 FAIL, 0 SKIP on the repaired bytes.

Implemented and tested the trust foundation above. The unchecked milestones in that section describe
the state of the 2026-10 pass, not the current tree; current capabilities and limits live in
[RELEASE.md](RELEASE.md) and the README.

Known boundaries: fresh builds deliberately rebuild dependencies; cached runs restore content-addressed verdicts and their artifacts, but no wall-clock speedup is claimed. Unsupported MIR assertions are reported, but abstraction provenance is not yet tracked per value and this is not a no-panic or general soundness certification. Replay checks the emitted model, not native execution. Discovered standalone Z3 does not attest the loaded solver library. Run records include absolute paths and compiler arguments: inspect before publishing. Concurrent top-level runs in the same directory still share the output root; serialize them pending per-run output isolation. Linux/macOS CI was not run locally; the shell runner and native Windows runner were both tested on this Windows checkout.
