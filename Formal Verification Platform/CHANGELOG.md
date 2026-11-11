# Changelog

Notable changes per pass. Dates are the day the change was verified, not the day it was started.

The entries below are append-only history and cite the status documents that existed at
the time: `Additional-Enhancements.md` (the A/B/C/D/E/F register), `BUCKET-LIST.md`
(K1 to K25), `OVERHAUL.md` (O1 to O16), `PROGRESS.md`, `ASSESSMENT.md`,
`IMPACT-OPPORTUNITY.md`, `ROADMAP.md` and `FORMAL-VERIFICATION-BUILD-PLAN.md`. Those
files were consolidated into [DONE.md](DONE.md) (finished) and [TODO.md](TODO.md)
(open) and deleted; the links in these older entries are left as written, because
editing history to match a later layout is what makes a changelog untrustworthy.
Every ID they cite still resolves in [TODO.md](TODO.md), which lists the sources it
was assembled from.

## Unreleased 2026-11-06 (S8 pointer half: null and alignment checks are decided)

- **A pointer check is now a proof about provenance, not an unchecked WARN.** The
  `NullPointerDereference`, `NullReferenceConstructed` and `MisalignedPointerDereference`
  assertions the compiler emits before a dereference are decided when the address comes
  from a `&T`, `Box<T>`, `NonNull<T>` or `Unique<T>`: such an address is non-null and a
  multiple of the pointee's alignment by construction, so both the null test and the
  `addr & (ALIGN-1) == 0` test are proved without any assumption about the value.
- **The model follows what MIR actually writes.** A MIR temp can be shared by several
  sites, so a temp is traced through *all* its definitions and accepted only when they
  agree; a store through the pointer (`(*_p).1 = …`) is not a store to `_p`; and an
  address this model cannot derive — an `extern` pointer, offset arithmetic, a value
  behind an opaque call — keeps its `unsupported-assertion` WARN instead of being guessed.
- **It proves and never refutes.** No `FAIL` is emitted from this path: a counterexample
  needs a ground obligation and a witness, and an address model that does not model
  allocation cannot supply one. `tests/safe_code` gained a `Box<[i32; 3]>` site so the
  fixture exercises both the `NonNull`-field path and the `Box` path, and its progress
  baseline now records those kinds as decided rather than unsupported.

## Unreleased 2026-11-06 (P1.6 first slice: `keller reproduce`)

- **A failing site can be pinned into a bundle another machine checks.**
  `keller reproduce <OBLIGATION.smt2> [--out <DIR>]` writes `reproduce.json` with
  the build that decided the site (`provenance.build`, the loaded solver,
  `config_hash`), the flags it ran under, the site (`crate`, `function`, `block`,
  `kind`, `file:line`, verdict, witness) and the obligation's digest, then
  re-discharges the formula with the solver the recorded run discovered. Exit 0 only
  when the fresh answer matches the recorded one.
- **A different compiler is refused, not reproduced.** The bundle compares the
  recorded rustc release/commit/host with the binary asked to reproduce it and exits
  3 on a mismatch: replaying an obligation from another compiler is a different
  analysis, and calling that a reproduction would be a claim nobody checked.
- **What it is not is written down in the bundle.** Seven `cannot_reconstruct`
  entries name what a matched replay still does not establish — concrete execution,
  heap and allocation addresses, results of unmodelled calls, loop trip counts, the
  source expression behind an operand, flags a crate's build sets, and the compiler
  binary's own contents. `finding_evidence` gained the matching note, so a report
  says the same thing without the bundle being opened.

## Unreleased 2026-11-06 (P1.5 CI half: the fixture suite runs on macOS)

- **`verify.yml` gains a `macos-fixtures` job.** It runs the same suite the Linux job
  runs — `cargo test`, the false-positive budget and `tests/run_tests.sh` — on
  `macos-13`, after the formatting and lint gates, with the Z3 macOS archive selected
  by `*x64-osx*.zip` from the release rather than by a hardcoded asset name that
  would silently rot. Until this existed, "macOS is development-tested" meant one
  machine and nothing in CI.
- **The loader variable is the one macOS actually reads.** The Linux job exports
  `LD_LIBRARY_PATH`; copying that line would leave every case unable to start the
  wrapper on macOS, and the suite would have reported it as a load failure rather
  than as the setup problem it is. The job exports `DYLD_LIBRARY_PATH` instead,
  composed from `keller-env`'s own output so the job does not re-derive the lookup
  the runners share.
- **A job is not a support claim, and the docs say which is which.**
  `docs/RELEASE.md` keeps macOS out of the supported table and adds what would move
  it in: an observed green run, a macOS installer with a pinned Z3 digest, a
  packaged archive smoke-tested on macOS, an arm64 decision, an OS floor and a
  clean-machine install check. No run of the new job has been observed from this
  checkout, so it is recorded as being in the tree.

## Unreleased 2026-11-06 (P1.4: evidence says what ran)

- **A retained verdict now names the solver library that decided it, and the settings it
  was decided under.** `report.run.json` gains `provenance`: the rustc release, commit and
  host this binary was built with, its target triple, build profile and whether debug
  assertions were on; the solver's `loaded_version`, asked of the loaded library through
  `Z3_get_full_version` rather than taken from the build script (measured here
  `Z3 4.16.0.0`), with the module path the loader resolved
  (`…/.z3/current/bin/libz3.dll`) beside the directory the build linked against; and
  `config_hash`, an FNV-1a digest over the effective settings and the baseline's bytes,
  published together with `config_canonical` so the digest can be recomputed.
- **The digest is stable on purpose.** `DefaultHasher` is not promised across Rust releases;
  a digest that moved when the compiler moved would make two runs incomparable for a reason
  that has nothing to do with the analysis. FNV-1a is pinned by its published test vectors
  in a unit test.
- **Identity is not attestation, and the docs say so.** Nothing signs `libz3`: the record
  says which library answered and under which settings, not that either is uncompromised.
  `env.identity_note` and `docs/RELEASE.md` now point at `provenance` instead of saying the
  solver's identity was unknown.

## Unreleased 2026-11-06 (K20: the documentation audit)

- **Every number the docs quote was checked against a run (K20).** `cargo test` is
  **187 passed, 0 failed** (21 `keller-env` + 166 verifier); `tests\run_tests.cmd` is
  **exit 0 with 219 PASS lines, 0 FAIL, 0 SKIP** over the **25** rows of
  `tests/cases.txt` — 23 analysed, plus `broken_code` and `missing_toolchain`, which
  exit 3 before analysis; **fifteen** goldens; `cargo fmt --all --check` exit 0; the
  lint gate is 79 style warnings against a ceiling of 90; Z3 is `4.16.0 - 64 bit` from
  the bundled binary; `--version` is `1.0.0`. The shell runner and the Ubuntu
  clean-install job were not run from this Windows checkout, and are recorded as not
  covered rather than as passing.
- **The audit found a stale default, and the docs were the side that was wrong.**
  `--run-timeout` has defaulted to `0` (disabled) since 2026-11-04, when the 1800 s cap
  was removed so an honest large crate could finish; `--help`, `keller init`'s template
  and `DEFAULT_RUN_TIMEOUT_SECONDS` agree. README, `docs/USAGE.md` (three places) and
  the constant's own doc comment still said 1800 s, and are corrected. A doc comment
  that describes a value the constant no longer holds is the same defect as a stale
  README: both read as a contract.
- **Dated records stay dated.** The run counts in `docs/TRUST-CHECKLIST.md` and
  `docs/STAGED-ARCHITECTURE.md` are the record of the runs that produced them; the
  2026-11-06 figures are added above them instead of overwriting, so the file keeps
  both the history and the current answer. `PROGRESS.md`'s measured paragraph moves to
  the current run, **92 of 102 decided (90.2%)**, and states that the frozen baseline
  is still the S7 one, whose two gains arrive as additions rather than a re-freeze.
- **No live reference to a deleted register remains.** `PROGRESS.md` had said the rest
  of the repository still linked to those old names; measured, none does outside the
  history sections that name them as the sources that were removed.

## Unreleased 2026-11-06 (K2 / O3: the missing-toolchain branch, end to end)

- **K2 / O3 closed with an end-to-end run, not a unit test.** The row stayed open
  because exercising the branch meant uninstalling the pinned toolchain, which
  `CONTRIBUTING.md` forbids — and that rule is right, since a suite that removes the
  compiler it runs on can destroy the machine it audits. The branch is reached
  instead through the real binary: a real `rustup` pointed at an empty `RUSTUP_HOME`
  genuinely answers `no installed toolchains`, so the verifier's pre-flight listing
  is empty and it exits `3` before `cargo` starts, naming
  `nightly-2026-07-24-x86_64-pc-windows-gnu` and the `rustup toolchain install …`
  line. Observed, then pinned by a `missing_toolchain` case in both runners: exit `3`,
  no `report.json` written, and not the `built with:` / `installed:` mismatch branch.
  The row states its scope rather than implying more — this is an emptied rustup home,
  **not** a real uninstall, and no machine without the compiler was tested. A compiled
  stub `rustup` was not used: it would have to be built by the toolchain whose absence
  the case is about.

## Unreleased 2026-11-06 (D3: the lint gate runs)

- **D3 lint half:** the row stayed open because `clippy` could not be installed for
  the pinned toolchain here, and a lint step nobody had run is a claim rather than a
  check. The component now installs (`rustup component add clippy --toolchain
  nightly-2026-07-24-…`), so the command was run and its result decided the gate.
  Measured on this tree: 79 style warnings and none in `clippy::correctness` or
  `clippy::suspicious` (the one the denial did catch, `empty_line_after_doc_comments`
  in `src/report.rs`, is fixed in this pass). `scripts/lint-gate.sh` denies those two groups and fails
  above a style ceiling of 90 (headroom over the measured 79, so another host's
  count is not a flake), and `.github/workflows/verify.yml` runs it after the build.
  A `-D warnings` gate was deliberately not taken: it would be a sixty-site refactor
  of test code in the same pass as verification changes, and the ceiling is what
  stops the number growing while that cleanup is pending.

## Unreleased 2026-11-06 (helper summaries: S7)

- **S7 (a private leaf helper's precondition is its callers'):** a function is
  analysed on its own body, so `fn inc(x: u8) -> u8 { x + 1 }` was reported an
  `unguarded` overflow even when every caller passed a value it can take. A
  small, private, cycle-free helper now has a precondition derived from its own
  arithmetic (`x + 1` implies `x <= 254` for `u8`), and the derivation is used only
  after every call site was shown to pass a constant satisfying it — one caller
  passing a runtime value, one passing the forbidden value, or a `pub` helper whose
  callers are not all visible, and the summary is abandoned and the body reported
  exactly as before. Where it holds, the body is analysed under the bound through
  the same entry guard `#[requires]` uses, so the finding becomes a proof carrying
  the bound as its guard, and each call site is recorded as a `helper-precondition`
  `PASS`. `tests/precond_probe`'s A8 boundary moved with it: `private_callee` is
  decided from its caller, `public_callee` and `private_unknown` are not.

## Unreleased 2026-11-06 (resume-after-drop: S6 correction)

- **S6 (the third coroutine kind, checked against real MIR):** `ResumedAfterDrop`
  was handled by symmetry with its two siblings, and `tests/async_drop_state` — an
  `async fn` holding a value with an `AsyncDrop` impl — shows that is wrong. The
  transform emits `resumed after async drop` into blocks its dispatch reaches
  through *suspension* states, which a resume may legitimately arrive in, so the
  resume-entry invariant says nothing about them and the symmetric treatment
  reported two counterexamples on code that cannot produce one. The kind is now
  reported as a named `unsupported-assertion` instead of a decision, and the two
  kinds the invariant does cover are still decided `PASS` in the same body.

## Unreleased 2026-11-06 (coroutine fixpoint: S6, second half)

- **S6 (the resume dispatch settles):** a coroutine body's abort blocks are
  `assert(const false, …)` whose success edge targets themselves, so merging that
  self-edge grew the block's condition by one disjunct per round and the six-round
  budget never settled — the whole body fell back to path-insensitivity and lost
  every guard. A constant condition that contradicts `expected` cannot fall
  through, so `successor_guards` now returns no edge for it. The assertion is still
  reached and reported; only its impossible continuation is dropped.
  `tests/coroutine_state` reports no `path-insensitive` line, `2 of 2` decided, and
  no FAIL or WARN.

## Unreleased 2026-11-06 (coroutine resume-state discriminants: S6, first half)

- **S6 (the state a coroutine is resumed in):** the discriminant the coroutine
  transform dispatches on is modelled where it is read, carrying the ABI's
  resume-entry invariant (state `0` or a suspension state, never returned or
  poisoned). `ResumedAfterReturn`, `ResumedAfterPanic` and `ResumedAfterDrop` are
  now decided assertions rather than `unsupported-assertion` WARNs, proved from the
  CFG — every edge into the block selects a forbidden state — so the verdict does
  not depend on the body reaching its fixpoint. `tests/coroutine_state` pins it:
  2 of 2 assertions decided, no unsupported kind, no counterexample. The adaptive
  round budget remains open.

## Unreleased 2026-11-06 (local dereference resolution: S5)

- **S5 (a read through a reference to one local):** `PathState` carries `pointees`, the
  single local each pointer in the body borrowed, seeded from `BodyWrites` and consulted
  only for an unrelocated state. `deref_key` resolves `*r` and `(*r).field` to that
  local's place key, so a function that takes a reference and reads through it compares
  against what the body wrote instead of a free symbol. A store through the pointer still
  retires the target, so the resolution cannot outlive a write it did not see.
  * `tests/potency_probe::through_local_ref`: `i < *r` bounds `v[i]`, decided PASS; the
    probe is 13 of 13 assertions decided, up from 12 of 12.
  * `tests/run_tests.cmd` and `tests/run_tests.sh` assert that line by name. Both now
    state the probe's real size (eighteen functions); the shell runner said fourteen and
    the Windows one was one short.

## Unreleased 2026-11-06 (container and iterator theory: S3 and S4)

- **S3 / S4 (Container lengths and desugared iterators):**
  `Vec`, `String` and slices answer `.len()`, `.capacity()` and `.is_empty()` from pseudo fields on
  the place key (`PSEUDO_LEN`, `PSEUDO_CAPACITY`, `PSEUDO_CURSOR`, `PSEUDO_COUNT`) rather than from an
  unmodelled call, with the length carrying `ValueDomain::SliceLen` so it keeps the type's bound.
  `Range`, slice iterators and `enumerate()` expose a cursor and a length through `iterator_parts`, so
  the desugared `next()` bounds the `Some` payload. A length the body never writes is also constrained
  equal across loop trips (`inv_loop_…`), which is what ties a loop's own bound to the length its body
  indexes. `widened_state` no longer emits a vacuous `v = v` disjunct for a carried place.
  * `tests/potency_probe`: 11 of 12 assertions decided before, 12 of 12 now. `range_len_of_slice`
    (`for j in 0..v.len() { ... v[j] }`) proved, and `fixed_array`, previously undecided on
    `Range::next`, decides.
  * `tests/run_tests.cmd` potency block asserts the end state: 18 functions, no counterexample, no
    undecided obligation.
  * Whole suite green: `cargo test` 21 + 164 passed; `tests\run_tests.cmd` all cases.

## Unreleased — 2026-11-04 (CFG dataflow O(1) fingerprinting, dependency verification skip, portable Windows launcher)

- **K3 / B12 (Root cause of CFG dataflow CPU wedge resolved):**
  Diagnosed the long-standing solver wedge / timeout affecting large functions with wide CFG merges.
  `PathState::fingerprint` previously converted the entire AST of path condition (`pc`), defs, and values
  to SMT-LIB strings on every block iteration and merge point, creating an $O(2^d)$ explosion in string allocations.
  Replaced with $O(1)$ AST-ID hashing using `z3::ast::Ast`'s native `Hash` implementation, and simplified
  term comparison in `merge_all` to direct term equality.
  * Verified on `OP-Roboter\crates\op-kinematics`: 127 functions analyzed in 4.31 seconds (was 900+ s timeout).
  * Verified on `OP-Roboter\crates\op-gate`: 8 crates analyzed in 19.65 seconds (212 proofs, 3 real integer overflows).
  * Tested on `Vantablack`: 2,557 functions, 1,292 assertions (707 proven safe, 80 exploit counterexamples found).
- **K4 (Third-party dependency skip & `--verify-deps`):**
  Added `--verify-deps` flag (and `verify_deps = true` in `keller.toml`). External dependencies from crates.io
  are compiled passively via `PassiveCallbacks` without undergoing MIR verification or SMT obligation generation,
  drastically reducing cold build times on large dependency trees.
- **F2 (Portable Windows launcher `keller-verify.cmd`):**
  Removed hardcoded machine paths from `keller-verify.cmd`. The script now dynamically asks `keller-env.exe`
  for runtime paths (`PATH_ADD`, `RUSTLIB`, `Z3_BIN`) and sets up PATH precedence for 64-bit MSYS2 toolchains.
- **CLI Invocations:**
  Unified command line handling in `src/main.rs`: `keller-verify verify ...` now behaves identically to `cargo keller verify ...`.

## Unreleased — 2026-10-13 (the open list is built, and the fixtures nothing ran are run)

This pass takes the strongest in-repo rows of [TODO.md](TODO.md) and implements them rather than
re-describing them. Every bullet below is the row it closes, the evidence, and what is deliberately
still open.

- **`keller.toml` and `keller init` (B8 / K18 / K19 / B9 partial).** Flags and `KELLER_*` describe one
  run; a repository also needs to record *decisions* in something a reviewer can read in a diff. `init`
  writes a starter file whose every setting is commented out — the run immediately after it behaves
  exactly as the run before it — and refuses to replace an existing file (exit `2`) without `--force`,
  because the file is where a team writes decisions down. The file supplies **defaults** and the command
  line is applied on top of them, including when the two disagree: verified on `tests/safe_code`,
  `profile = "strict"` in the file exits `1` on its WARNs while `--profile advisory` on the command line
  with that same file exits `0`. An unknown key, an unknown table or a wrong type is an error naming the
  file and line (`keller.toml:1: unknown key 'strict_covrage'`, exit `2`) rather than a silently ignored
  line, so a typo cannot leave the gate looser than it looks. `[[policy]] crate = "…" gate = "advisory"`
  keeps a crate's findings reported and out of the gate; the count lands in `report.run.json` as
  `gate.advisory_excluded` (verified: 1 excluded, gate passed, exit `0`). Still open: per-path overrides
  (a policy matches a crate name, not a path or a glob).
- **The obligations manifest (the last row of "Reporting and artifacts").**
  `obligations/<crate>/index.json` now exists: every obligation the crate emitted with its status,
  kind, function, block, `file:line`, the artifact it left and **the verdict that artifact's own header
  records**, sorted and de-duplicated so an unchanged crate produces a byte-identical manifest, plus the
  `re_discharge` command that re-checks any entry. The proofs are in it, which is the point —
  `report.json` is the list of things to act on, so a proved assertion was only *addressable* once
  something listed it. Writing it exposed a bug in the first version: the records were collected as an
  `Option<Vec<_>>`, so one event that left no file (an unsupported assertion, a path-insensitive
  fallback) blanked the manifest for the whole crate. `tests/safe_code` writes two `PASS (unsat)`
  entries now, and `tests/trust_checks.py` asserts it.
- **`witness_kind` (K10 / A14 residual).** A relational failure named one instance among many without
  saying so: `a + b` overflowing pins neither operand, so its operands are a pair the solver chose, and
  two runs could print different numbers as though they were facts. Every FAIL with a witness now says
  which claim it is — `pinned` or `one-of-many` — decided per value by asking the solver whether the
  obligation forces it (`value_is_pinned`). Measured on real runs: `tests/overflow_fail` is
  `one-of-many`, a folded failure is `pinned`, and `test-crates/buggy-project` reports both in one file
  (`check_div` pinned, `check_index` one-of-many). Still open: making the instance deterministic, which
  means choosing a canonical one.
- **Every report states the profile it describes (A5).** `analysis_profile` and
  `analysis_profile_note` are in `report.run.json`, in the `--json` summary and in the SARIF run
  properties, and the console prints one line before the verdict; the four surfaces read one constant,
  so they cannot disagree. A release build wraps instead of panicking, so "this cannot overflow" and
  "this will not abort" are different claims, and this is the half of the row that stops a report being
  quoted for the wrong artifact. The `--release`/`--wrap` **mode** is still not built; the row's "or" is
  what this satisfies.
- **The last silent skip is gone (A7 residual).** The bounds arm used to `continue` when the index
  rendered as a panic-internal place: no verdict, no kind, nothing on the console — an assertion that
  had been *encountered* and then vanished, which is the one wrong answer this project treats as a
  security bug. It reports a `WARN` naming `PanicInternalBoundsIndex` and moves the count from
  `supported` to `unsupported`, so the receipt's own consistency rule still holds. No case reaching that
  branch could be constructed on this toolchain (a place's debug text does not look like
  `core::panicking::…`) — that is the limitation, and it is stated rather than papered over. The filter
  itself is now pinned by a unit test that a function merely *called* `PanicHandler::run` is not skipped.
- **The fixtures nothing ran are run, by both runners (D8).** `tests/precond_probe` and
  `test-crates/{buggy-project,safe-project}` were in the tree and run by nothing. All three now go
  through the same checks every other case gets (obligations re-discharged by the standalone solver,
  witnesses, `file:line`, the environment sidecar) plus assertions in both directions. `precond_probe`
  pins what the tool actually does with a precondition: the in-body `assert!` is **believed** — its
  condition is the `; reachable:` guard of the proof beneath it — while a callee is still judged on its
  own body and not on its call sites, which pin out-of-range values that never occur (A8's boundary,
  asserted *as* a boundary). Wiring them found the failure mode the wiring exists to prevent: the
  measured log was named after the case, so `test-crates/buggy-project` made bash redirect into a
  directory that does not exist, the run never happened, and the case "passed" its exit-code check
  while proving nothing. The log name is slugged and the case name is not. The Windows runner then
  failed the same case with a wrong exit code, because a case whose name is a path (`test-crates/…`)
  was looked for under `tests\`; `:case_dir` in the script and `case_dir()` in the shell runner now
  decide that one way. Wiring it also exposed a quieter defect of the same family: `cmd` ends a
  parenthesised block at the first `)` its parser meets, so `echo [FAIL] … produced N finding(s)`
  closed the block early and the `set "FAILED=1"` beneath it was no longer inside its condition —
  the runner printed every check as passing, printed no `[FAIL]` at all, and exited 1. It was a
  one-line reproduction to confirm (`echo …^(s^)` keeps the block, the bare form does not) and the
  rule was already in the file's own comment ("no parentheses in any echo below"); it is now a unit
  test on the file's text, so the next one fails `cargo test` with the line number instead of leaving
  a suite that fails and cannot say what failed.
- **`CONTRIBUTING.md` and `CODE_OF_CONDUCT.md` (E3 residual).** The contribution rules are the ones
  this repository actually enforces (the pinned toolchain and why, the three commands, goldens
  re-recorded from a run and read as the diff, both runners kept equivalent, no assertion silenced, a
  claim in the docs that nothing checks is a bug); the conduct document keeps the same evidence-first
  standard without inventing a reporting channel that does not exist.
- **The references to the deleted registers are gone (K20 partial).** The workflow's `OVERHAUL.md §4b`
  and the manifest's `OVERHAUL §5` named files that are no longer in the tree; they point at
  [TODO.md](TODO.md) now, and the `BUCKET-LIST Kx` citations are the bare ids the fused lists ask for.
  `keller init`/`keller.toml`, the manifest and the analysis profile are documented in
  [docs/USAGE.md](docs/USAGE.md), `tests/README.md` and [README.md](README.md). The quoted result
  counts in this file's neighbours are refreshed below; no pass has audited every number in every doc
  against a run.
- **The panics a report does not mention (A6).** Every check this tool makes is a proof about an
  *assertion* the compiler inserted; `unwrap`, `expect`, `panic!`, `unreachable!`, the failure path of
  `assert!`/`assert_eq!` and the allocation-failure abort are not assertions but calls into another
  crate whose body it does not read, and until this pass they were walked past in silence — so a crate
  full of unwraps produced the same `[]` a crate that cannot panic produces, and "no findings" could be
  read as "no panics". Each reachable such call site is now a `WARN` with `kind: "unmodelled-panic"`
  naming the callee, counted in `coverage.panic_sites`, and printed on a line of its own (`6 call
  site(s) that can panic were not modelled`); they are counted *apart* from the assertions in the
  coverage headline and in the summary, which is the half that keeps the number honest — the fixture
  with two assertions and six call sites reports two assertions, not eight. The rule's first version
  was written from the documented spelling and reported nothing at all: this toolchain renders
  `Option::unwrap` as `std::option::Option::<T>::unwrap` and the `panic!` entry point as
  `std::rt::panic_fmt`, not the `core::…` spelling, so the classifier now works from the crate and the
  last path segment, with the exact strings taken from a MIR dump as unit tests. A local callee is
  deliberately excluded (its body is analysed, so its panic is named where it happens rather than at
  every caller — the fixture's `main` calls all six and contributes none), and nothing became a `FAIL`:
  three fixtures gained the WARNs they always should have had (`overflow_fail`, `divzero_fail`,
  `workspace_multi` — all `env::args().next().unwrap()` in `main`) and `tests/precond_probe` gained the
  one its in-body `assert!` always was, which is the gap that case's own comment had recorded as A6.
  Still not modelled: naming a site is not deciding it, so no function is proved panic-free — the row's
  own list is what remains.
- **The two runners check the same cases, from one owner (D8).** The Windows runner ran fifteen cases
  and compared ten goldens while the shell runner ran fifteen and compared thirteen, and the two cases
  whose whole point is the *reason* inside a `WARN` - `unnamed_write` and `statement_writes` - had no
  block there at all; `potency_probe` was run by nothing but the shell runner. The case list, the exit
  code each case must produce and which cases have a golden are now one file, `tests/cases.txt`, which
  both runners read: `run_table_case` in the shell runner and `:run_table_case` in the script look the
  exit code up from the row and compare the golden exactly when the row names one, so neither runner
  decides the golden set for itself any more. Both ends are checked at the end of each run - every row
  must have been consumed (a case needing its own invocation marks itself, so "deliberately different"
  and "forgotten" are different states), and the rows naming a golden must be exactly the files in
  `tests/goldens`. The Windows runner went from **85 to 134** passing checks by running three fixtures
  it never ran, and its failure path was proved to work rather than assumed: with one row's expected
  exit deliberately flipped and one golden row deliberately renamed, `run_tests.cmd` printed
  `[FAIL] safe_code: expected exit 1, got 0` and two `[FAIL]`s naming the renamed golden, ended with
  `*** FAILURES DETECTED ***` and exited **1** - the printed failure the earlier silent-`FAILED=1` bug
  could not produce. The two edits were reverted and the table's SHA-256 confirmed back at its previous
  value before the green run below; the A6 block later in this entry takes the same runner to **156**.
  Still different between the runners, and now written down in
  [tests/README.md](tests/README.md): no per-obligation re-discharge with the standalone solver, string-
  only needles instead of exact witness values, `fc` instead of `diff`, and no `--run-timeout` cap probe
  on Windows.
- **Evidence for this pass.** `cargo fmt --all --check` exits **0** repository-wide (the scoped check
  in [docs/TRUST-CHECKLIST.md](docs/TRUST-CHECKLIST.md) was the 2026-10-09 state). `cargo test` is **155
  passed, 0 failed** (20 `keller-env` + 135 verifier tests — one more in each binary, because
  `src/host_env.rs` is compiled into both and that is where the two runner checks live).
  `tests/run_tests.sh` is exit **0**, **308 PASS lines, 0 FAIL, 0 SKIP** over the **twenty** fixtures
  in `tests/cases.txt`, including the four wired in here and the two checks on the table itself;
  **`tests\run_tests.cmd` is exit 0 on the same bytes, 156 PASS, 0 FAIL, 0 SKIP** - 85 before this pass
  ran the three fixtures it had no block for, and 1 when a row's expected exit and a golden row were
  deliberately changed to prove its failure path prints. Thirteen of the
  fourteen goldens were re-recorded; most of that refresh is the earlier one — **34 added fields in nine
  files**, seventeen `defect_class` and seventeen `witness_kind`, with no verdict, witness value or
  finding count changed — plus, in this pass, the unmodelled panic sites three fixtures should always
  have reported and the new `panic_sites` golden. `scripts/fp-rate.py` publishes the same figures as
  before it (`2/18 = 0.111` over one more report) because a panic site is excluded from its `undecided`
  column: it is not an obligation the analysis declined to decide.

## Unreleased — 2026-10-13 (a run that stops making progress is stopped)

- **`--run-timeout <SECONDS>` bounds the analysis, not just a query.** The default is **1800 s**;
  `--run-timeout 0` removes it. A run that reaches the cap is stopped, the process tree it started is
  killed, and it reaches **no verdict**: exit `3`, a record whose `gate.status` is `run-timeout` with
  `coverage: null`, the crates that were still in flight named on stderr from their own `.started`
  receipts, and nothing published over the last completed `report.json`. This is the half of K3 that a
  pipeline needs: `--solver-timeout` bounds one query, and the failure it cannot reach is a run that
  stops progressing at all. Measured: the controlled wedge (`tests/undecided` with `--solver-timeout 0`)
  stopped at 15 s and again at 20 s, with no process of either run alive afterwards, and a normal run
  under `--run-timeout 0` still finished in 31 s and exited `0`.
- **The kill is a tree kill, because killing the child is what leaked processes.** `cargo` starts a
  second cargo, one workspace wrapper per crate and rustc under those, and the wrappers are the
  analysis — so killing the direct child left a `keller-verify` → `cargo` → `cargo` →
  `workspace-wrapper` tree running. Measured here: one such tree from a sweep attempt was still alive
  **six hours** after the wrapper that started it was killed. The cap now kills the tree (`taskkill /T
  /F` on Windows; a process-group signal on POSIX, from a `cfg(unix)` `libc` dependency), and
  `scripts/sweep-local-crates.ps1` does the same and hands its own cap to the tool instead of timing
  the run out from outside.
- **Nine suite checks pin the contract, not the cause.** `tests/undecided` is invoked a third time with
  no solver budget and a 20 s cap: exit 3, the console naming the cap and the crate still in flight,
  the run record saying `run-timeout` with no verdict in it, nothing published, and no wrapper or cargo
  process left behind (a count taken before and after, plus a path check where `ps` prints paths). The
  suite is **243 PASS lines, 0 FAIL, 0 SKIP**, and `cargo test` is **143 passed, 0 failed** (19 + 124).
- **The cause is not named, and the register says so.** What is known now has a receipt that did not
  exist before: at the cap every process sits at a few milliseconds of CPU and none is spending more,
  which was first read as a blocked process; it is an unbounded Z3 call (K3) — and `op-gate` wedges the same way with a solver
  budget set, so it is not the budget path either. The stopped run's directory keeps the last
  obligation written and the crates that were in flight, which is where a controlled reproduction
  starts (`BUCKET-LIST.md` K3, `Additional-Enhancements.md` B12).

## Unreleased — 2026-10-13 (the landing page stops being generated)

- **`index.html` is hand-written and nothing generates it.** The page used to be an artefact of the
  fixture run: `tests/gen_index.py` rebuilt three marked regions (the obligation panel, the evidence
  panel and the status paragraph) from `tests/.measured/` and each case's `report.json`, and
  `tests/run_tests.sh` failed the build when the committed page stopped matching the run. The effects
  were that any edit to the page was a two-file change, that a stale measurement made the page wrong
  rather than merely old, and that machine-formatted prose sat in the middle of a landing page. The
  generator, its `tests/gen_index.sh` wrapper and the page's generated regions are removed, so the page
  carries no per-run figures and the suite has nothing to compare. What it used to publish stays in the
  repository: the goldens under `tests/goldens/`, the emitted obligations under each case's
  `obligations/`, and the register in `Additional-Enhancements.md`.
- **The suite still captures each case's console stream** into `tests/.measured/` (gitignored), because
  its later cases grep those streams for the lines they assert on. It just no longer feeds a page.
  `scripts/measured-run.ps1` reproduces those streams on Windows and now covers the same fifteen cases
  `tests/run_tests.sh` runs, instead of the twelve it covered when it also fed the generator.
- **The page was rebuilt around what it is for**: light surface, one accent, a console transcript in the
  hero, the assertion classes as a table, the pipeline as steps, one real obligation with the answer Z3
  gave when handed it, the comparison against testing, an explicit "what it does not do" section, and
  the build commands. The numeric tiles, the card grids of small promises and the obligation table are
  gone; that evidence lives in `tests/`, where it can be run.

## Unreleased — 2026-10-13 (the class the rate is published from was a location label)

### `widening-limited` is a dependence test now, and the number it counts was re-measured

K7 publishes one number: the share of `FAIL`s attributed to the loop abstraction. The class it was
computed from could not fail. The class test asked whether any *declared input name* contained
`prev_bb` (the audit's name for it, `mentions_widening`, was never a function in this code), and
`widened_state` declares a renamed copy of every symbol on the back edge — so
every obligation inside or downstream of a widened block was `widening-limited` by construction, and
the number measured "this finding sits near a loop" rather than "this finding leans on a generalised
value" (audit finding; register **A19**).

- **The failing value is now the input.** `asserted_operands` gives the failing assertion's own
  operands — exhaustive over `AssertKind` with no wildcard arm, so a new assertion kind is a compile
  error rather than a finding classified from the state alone — and `PathState::symbol_of` names the
  places they are stated over. `classify` reads those names together with the witness symbols; the
  state's own inputs (which is what the old test read) are not part of it.
- **What the widening gave up on is recorded.** `PathState::generalised` holds the places
  `widened_state` empties because the back edge carries them, so a finding whose own value is one of
  those leans on the abstraction even where no relation was written down for it. It is a `BTreeSet`
  because it is rendered into `fingerprint`, and a fingerprint that varied between runs would make
  the contribution order — and so the emitted SMT-LIB — vary with it. A widened symbol declared for a
  place the finding never reads is no longer evidence, and neither is another place's generalisation.
  Both non-cases are pinned as assertions in the classifier's own unit test
  (`a_failure_is_classified_by_the_state_it_was_checked_in`), beside the whole-name matcher test that
  the `defs` closure relies on.
- **Measured on the committed corpus:** `py scripts/fp-rate.py --dir tests/goldens --budget 0.15` →
  **2 of 18 (0.111)**, the same figure as before and a different fact: `tests/loop_fail` and
  `tests/nested_loop` are `widening-limited` because their failing operands are loop-carried values
  the widening gave up on. No other finding in any golden moved class, and the two goldens differ
  from the previous pass's in the `defect_class` field alone (the suite's byte comparison named both
  before they were re-recorded).
- **The real-crate sample was re-measured, and it reproduces.** The instrument changed, so the sample
  the headline rests on was re-run rather than assumed: the four crates carrying its `FAIL`s were
  analysed again with this build (56.8 s, 14.0 s, 22.8 s, 1.9 s), and the finding sets are identical
  finding for finding — `sca-security` 13 → 13, `homomorphic-compare` 6 → 6, `threshold-crypto` 2 → 2,
  `zk-core` 1 → 1, **0 classes moved** — so the sample is **1 of 9 (0.111)** under both tests. The one
  attribution is the one the old test got right (`MemoryEncryptor::xor_with_keystream`), and the two
  other `FAIL`s in that crate are `unguarded` and stay `unguarded`, which is the separation the
  location test could not express. Raw tables:
  `docs/measurements/fp-rate-2026-10-13{,-recheck}.json`, `sweep-results.csv`, `sweep-recheck.csv`.
- **The gate is shown to fail, not assumed to.** `0.111` passes at `--budget 0.15` (exit `0`) and the
  same command exits `1` at `--budget 0.10` (`[FAIL] the suspect rate 0.111 is over the budget
  0.100`). The CI budget's margin is one finding: a third widening-attributed `FAIL` in that corpus
  of 18 would read `0.167`.

### The CI claims are marked unverified, because nothing can be shown to execute them

- **The two newest gates are uncommitted lines, and the workflow has never been observed to run.**
  `git status` → `M .github/workflows/verify.yml` (+22 lines: *Formatting gate*, *False-positive
  budget*); the repository's Actions API answers `404` to an unauthenticated request and no `gh` CLI
  is installed here, so no run of any job can be shown from this checkout. Every row and paragraph
  that read "enforced in CI" (this file, `BUCKET-LIST.md` K7, `OVERHAUL.md` O14/O16 and the D3 row,
  `docs/FALSE-POSITIVE-RATE.md`, `Additional-Enhancements.md`) now says what *is* shown: the command
  and its exit status here — `cargo fmt --all -- --check` → `0`,
  `py scripts/fp-rate.py --dir tests/goldens --budget 0.15` → `0` at 0.111 and `1` at 0.10 — both of
  which `tests/run_tests.sh` runs as its own steps. `Additional-Enhancements.md` carries the standing
  note the CI rows point at; the landing page's generated Status paragraph was corrected with it in
  `tests/gen_index.py` — and then **that whole path was removed the same day by a concurrent change in
  this working tree**: `index.html` was replaced by a hand-written page at `09:42` (`README.md` at
  `09:43`), and `tests/gen_index.py` and `tests/gen_index.sh` are deleted (`git status` → `D`) with the
  suite's page step replaced by a note that the page now carries no per-run figures at all
  (`tests/run_tests.sh:1370`). So the correction is not in the page and the generator no longer exists
  to put it there; the figures quoted in this entry are from the 09:38 run, and the replacement page
  deliberately publishes none. Its one enforcement claim was fixed in that hand-written text
  ("driven by CI on every push" → the shell runner, with what is shown named) because the CI note's
  rule applies to the page too.
- **D3's stale reason is corrected where it was written.** The row said the pinned toolchain has no
  `rustfmt` component; it has, `cargo fmt --all --check` exits `0` on this tree, and the `fmt` half is
  landed. The lint half stays open for a reason that was re-checked rather than inherited:
  `cargo clippy --version` fails here with "`cargo-clippy.exe` is not installed for the toolchain
  `nightly-2026-07-24-x86_64-pc-windows-gnu`".

### Two runs that did not reproduce, recorded as unreproduced (A20)

- The suite exited `3` once on the `workspace_multi` case, and one intermediate run reported six
  `FAIL`s — the `loop_fail`/`nested_loop` proofs plus a stale page — none of which recurred on the
  same bytes, while that run's own captured logs and obligation files contain the proofs its runner
  said were missing. Both are recorded as **unreproduced** rather than resolved, in
  `Additional-Enhancements.md` **A20** and in `tests/README.md`, with one observation and its numbers
  rather than a story: a `keller-verify.exe` from the earlier sweep (WINPID 12404, started `03:08`)
  was still alive about six hours later, blocked with its `workspace-wrapper.exe` child in an
  unrelated crate's `.keller/runs/` directory and holding `0.06 s` of CPU. An orphan, not a load, and
  not demonstrated to have caused either failure.

### Verified on the final bytes

- `tests/run_tests.sh` **exit 0, 0 FAIL, 0 SKIP**, with the 13 goldens byte-compared, the K5 probe
  green and the K7 budget green at **2/18 = 0.111**. The PASS total reads **235** on this pass's bytes
  (the page step was one of the lines) and **234** once the page stopped being an artefact of the run
  — the same suite with that step removed, re-run on the bytes now in the tree with the SHA-256 of
  every tested file identical before and after, which is what makes the result cover them rather than
  race the editor who changed them.
- `cargo test` **142 passed, 0 failed** (19 + 123); `cargo fmt --all -- --check` exit `0`.

## Unreleased — 2026-10-13 (every other way a write is spelled)

### Four wrong proofs, two of them plain safe Rust

The previous pass closed a store the key scheme could not *name*. This one closes the other half: a store
spelled somewhere the walk was not looking. `apply_assign` was the only thing that retired anything, and
the call terminator retired only its own destination, so a write made by a body this analysis does not
read — a callee's `&mut` argument, drop glue, `copy_nonoverlapping`, an inline asm block's output — left
the pre-call value in the state. Each of these was **proved** for a function that panics on every input
its guard admits:

```text
[PASS] Verified formal integrity: call_writes (bb2)      // fn bump(v: &mut u8) { *v = 250; } bump(&mut v);
[PASS] Verified formal integrity: call_writes (bb4)      //   v + 240 is then "proved" from s + x <= 8
[PASS] Verified formal integrity: drop_writes (bb2)      // a `Drop` impl writing *self.0, reached at scope exit
[PASS] Verified formal integrity: drop_writes (bb4)
[PASS] Verified formal integrity: copy_over (bb2)        // core::ptr::copy_nonoverlapping(&source, &mut v, 1)
[PASS] Verified formal integrity: copy_over (bb4)
```

Those are the lines the scratch crates that established reachability printed, which is why
`copy_over` is not a name in the fixture: `tests/statement_writes` pins the same four shapes under
its own names (`call_writes`, `drop_writes`, `intrinsic_writes`, `asm_writes`), and the pre-fix state
of each is asserted there in the direction it failed.

### The fix: the walk is a table, not a patch

Every statement kind and terminator kind the pinned toolchain can emit now has a disposition in one
exhaustive `match` each (`PathState::transfer` and its terminator table, `successor_guards`,
`compute_reachable_blocks`), with **no wildcard arm**: a variant nobody has decided about fails to
compile rather than silently writing nothing. The disposition per variant is written out beside the
match. Writes are retired through the existing discipline (`invalidate` + `forget`, so a later read is
*undecided with the reason named*), and reach one hop further than before: a call's pointer argument is
resolved to the local its own MIR borrowed (`f(&mut w)` retires `w`, not everything borrowed), a pointer
the body never wrote is a parameter — the caller's memory, not one of these locals — and only a pointer
whose pointee is unknown (`&*p`, a second borrow, a value out of a call) falls back to every
address-taken local. `SetDiscriminant` is retired for the case that emits it (the coroutine transform,
whose bodies this analysis does read) and `Yield` for completeness; both are documented as unreachable
or unkeyable *today* rather than silently handled.

`tests/statement_writes` is the fixture and asserts both directions: the four shapes are never `PASS`
again (the runner requires each old `[PASS] … (bb4)` line to be absent), each is a `WARN` naming the
write that cost the value, nothing is laundered into a `FAIL`, no witness is pinned, and the two
controls — a call whose pointer names one local, and a call handed no pointer into this body's storage at
all — stay proved. That pair is the point: a fix that retired everything any call could touch would buy
the four WARNs with two false positives.

The `asm_writes` shape found a second defect next door, in the walk rather than the write: `InlineAsm`
was not in either successor walk, so the assertion *after* an asm block was never entered — no verdict at
all, and no miss reported either, because the coverage figure counts the assertions the walk
*encountered*. `InlineAsm`, `FalseEdge`, `FalseUnwind` and `Yield` now follow their real edge, and the
non-returning variants are listed explicitly so a new one cannot go unnoticed.

**Cost, measured rather than asserted.** On the curated fixture corpus no verdict moved at all: the
goldens are byte-identical to the previous pass's apart from the new case (the only diff is the previous
pass's `defect_class` field). The real-crate cost is precision that can only *add* WARNs or FAILs — a
local handed to a call as a pointer loses its value — and the K7 sweep that measures it is not re-run
here. Register: **A18**.

## Unreleased — 2026-10-13 (a store the key scheme cannot name retires what it recorded)

### The failure it prevents: a proof of a program that is not

`place_key` names a place by its local and its field path, so a store through a reference —
`*r = 250` — names nothing at all, and `apply_assign` returned on that answer without touching the
state. The store had still happened: `v` kept the value the copy out of `s + x` had put there, the
surrounding guard bounds that by 8, and `v + 240` was answered from it and *proved*, so the tool
printed `[PASS] ref_write (bb3)` for a function that panics. A stale value is the one way this
analysis could report a proof it does not have, and the shape is ordinary safe Rust.

A store the key scheme cannot name now retires what it may have reached and records why: its own base
local — an index write `a[i] = …` destroys the whole `a` the state may have recorded — plus every
local the body has taken the address of, when the projection goes *through* a pointer, since which of
them the reference points at is the aliasing question and this analysis does not answer it.

`tests/unnamed_write` is the fixture, and it asserts both directions: the obligation that depended on
the destroyed value is a `WARN` naming the store and never a `PASS` again (the runner requires the old
`[PASS] … ref_write (bb3)` line to be absent), its emitted obligation re-discharges to `sat` — the
counterexample is real, the attribution is what is missing — no witness is pinned for a verdict that
names no input, and the control whose store the analysis *can* name stays proved. That control is the
point: a fix that gave up on every place a reference touches would report it, which is a false positive
bought for nothing. Four unit tests cover `invalidate` and what a store retires, which nothing did
before.

The cost is precision on an unnamed write — the storage loses its value, so anything downstream of it
is answered over the local's type again — and that direction can only add counterexamples. Register:
**A17**.

## Unreleased — 2026-10-13 (K6, K9, K7: a class per FAIL, the safe arithmetic, and the rate)

The Tier-1 tranche that K5 unblocked, closed: a failure now says which state it was checked in,
the safe-arithmetic idioms have encodings with tests, and the false-positive rate on code nobody
here wrote is measured, published and gated.

### Every FAIL is attributed to the state it was checked in (K6)

- **A `FAIL` carries a `defect_class`**: `widening-limited` when the state mentions a symbol a
  widened cycle block introduced (the failure leans on a value the loop abstraction generalised —
  where a residual false positive comes from), `guard-limited` when the guards that lead to it are
  all in the formula and the model satisfies them, `unguarded` when there is no path condition at
  all, and `constant-folded` for the folded-operand path. Decided where the obligation is checked,
  from the state it is checked in.
- It is machine-readable and routable: one console line (`FAIL(s) by class: widening-limited 1 …`),
  `counts.by_defect_class` in the `--json` summary, and a SARIF rule `keller/<kind>/<class>` with the
  class as a result property, so a dashboard can route "possibly the loop abstraction" separately
  from "a bug".
- The goldens recorded the new field with **zero deletions**, so no verdict, witness, reason or line
  moved. The suite asserts each fixture finding names its file, line and class.

### The safe-arithmetic idioms (K9)

- **`wrapping_*` and `saturating_*` are modelled exactly** rather than left as free symbols: the
  wrap uses the analysis' existing fixed-width reduction (`fixed_binary`, whose `Mul` case is why one
  add-or-subtract of the modulus is not the reduction at all) and the saturating form clamps. Both
  are checked against an independently written `i128` reference over every width and operand pair,
  and widths of 128 bits are refused rather than approximated. Writing that test caught a real bug in
  the first attempt: `-128 * -128` in `i8` wraps to `0`, not `16128`.
- **`checked_*` is modelled with both halves of its `Option`**, and the row's warning was right: with
  the payload modelled and the discriminant not, `checked_sum_gives_the_sum` was reported as an
  underflow for `s = 0, x = 1` — which is `y = 255`, the `None` arm, where the program cannot read
  the payload at all. The encoding stores the sum, the payload place and both variant indexes, and
  answers `discriminant` from them: `Some` exactly when that sum is in the operand type's range. Its
  soundness conditions are structural, not guessed — a downcast collapse is allowed only where one
  variant is the only variant carrying fields (`Option<T>`: `None` has none; `Result<T, E>` is
  refused) and only when the payload's type is the operand type, and the fact is retired with the
  place it names and dropped on a merge unless every path holds the same one.
- The probe proves all four forms instead of pinning one as debt: `saturating_never_below (bb1)`,
  `checked_sum_gives_the_sum (bb4)`, `wrapping_sum_under_input_bound (bb4)` and
  `narrowed_under_input_bound (bb2)` are `[PASS]`, and `tests/potency_probe` exits `0` with a single
  undecided obligation (the unmodelled `Range::next` behind `fixed_array`). The runner asserts the
  two needles the checked obligation carries — `(ite (and (>= (+ mir_1 mir_2) 0) (<= (+ mir_1 mir_2)
  255)) 1 0) 1` and `(= overflow_l (mod (+ mir_1 mir_2) 256))` — and that it re-discharges to `unsat`,
  so closing or breaking it is a diff rather than a paragraph. `cargo test` adds
  `the_checked_payload_is_the_exact_sum_whenever_the_sum_fits` over the whole `u8` square.
- **Narrowing casts were already modelled**, and the register said they were not; the probe now
  demonstrates it, and the claim is corrected below.

### The false-positive rate, measured (K7)

- **1 of 9 `FAIL`s (0.111)** is attributed to the loop abstraction, and **5 of the 9 carry a witness
  at a type's boundary** (`((*_1).1: u64) = 18446744073709551615`), over seven runs that completed
  from sixteen crates attempted. Method, sample and limits: [docs/FALSE-POSITIVE-RATE.md](docs/FALSE-POSITIVE-RATE.md);
  machine-readable table: [docs/measurements/fp-rate-2026-10-13.json](docs/measurements/fp-rate-2026-10-13.json);
  classifier: [scripts/fp-rate.py](scripts/fp-rate.py).
- What the sample says is not flattering and is the point: 8 of the 9 are `unguarded` and none is
  `guard-limited`, so on real code so far the tool has produced *models* rather than reachable
  violations — and crates where it declines (one sample crate: 12 obligations, all `WARN`) contribute
  no rate at all.
- The measurement caught a bug in its own instrument: a crate analysed twice because it is a
  dependency of two targets appeared in two reports, and was counted twice. `fp-rate.py` now
  identifies a finding by crate, function, block, status, kind, details and witness and counts it
  once. `scripts/sweep-local-crates.ps1` gained `-TimeoutSeconds N`, so a crate that runs past its cap
  is killed and reported `TIMEOUT` (leaving no `report.json`) instead of reading as a crate with no
  findings.
- The budget is `--budget 0.15`: the curated corpus sits at 0.111 (the two loop fixtures, whose
  counters are loop-carried) and the real-crate sample at 0.111, so one more widening-attributed
  finding trips it. The computation runs in `tests/run_tests.sh` over the committed goldens; the
  workflow's *False-positive budget* step holds the same command, but see the correction in the entry
  above — the step is in the file, and no run of the workflow can be shown from this checkout.

### Claims the new encodings made false

- `docs/RELEASE.md`, `README.md` (twice), the generated status paragraph in `tests/gen_index.py` and
  the comment beside the `_ => BinaryResult::Unmodelled` arm all said bit-twiddling, shifts, narrowing
  casts and all three safe-arithmetic families were unmodelled. They are modelled; what is still
  unmodelled is the `unchecked_*` operators (no defined result, so wrapping them would invent
  semantics for UB) and `PtrMetadata` outside a slice length.
- The landing page's header line quoted "122 unit tests · 13 fixture projects" by hand — the very
  drift `gen_index.py`'s generated regions exist to prevent — and now states no counts, so the only
  figures on the page are the ones the fixture run regenerates.

### A formatting gate (D3, `fmt` half)

- `cargo fmt --all` reformatted the tree (30 hunks; seven lines of the diff are not layout) and
  `cargo fmt --all --check` now exits `0`. The *Formatting gate* step in
  `.github/workflows/verify.yml` runs that same command — but the step is an **uncommitted** line in
  this checkout and the workflow has never been observed to run, so what is shown is the command and
  its exit status on this host, not a red build. The lint half stays open and named: `cargo clippy`
  fails here with "`cargo-clippy.exe` is not installed for the toolchain
  `nightly-2026-07-24-x86_64-pc-windows-gnu`" (re-checked), and a gate that has never been run would
  be a claim.

- Test counts after this pass: **134 unit tests**; `tests/run_tests.sh` **188 PASS, exit 0** (both
  superseded by the refresh in the entry above).

## Unreleased — 2026-10-13 (K5: the loop relation, and a probe that asserts)

K5 — the dominant false positive, the loop's own exit value reported as the violation — is closed,
and the case that measured it is no longer a known-debt pin.

### The loop's guard now reaches the assertion it guards

- **A widened loop head relates its loop-carried values to the previous trip instead of forgetting
  them.** `widened_state` keeps, for each place the back edge defines, the disjunction
  `value = <this trip's> ∨ (previous trip's guard ∧ value = <previous trip's value>)`, written about
  the same symbol the body's reads invent — so the guard on the edge out of the head and the value its
  body checks are one quantity. `while i + 8 <= v.len()` used to be reported as an overflow whose
  witness `18446744073709551608` the loop condition excludes, and `v[i]` in the body as a bounds
  failure at the loop's *exit* index.
- **A relation is asserted only when both halves exist.** A place only the back edge defines is left
  free, which is sound and merely imprecise; asserting one half would be asserting the previous trip's
  guard as a fact, which is exactly the over-constraint that proves a loop with a bug two iterations
  in.
- **A cycle block is widened once, and that state is held while the graph settles.** Re-deriving it
  every round does not converge: widening reads which places the back edge carries off the back edge's
  own values and then forgets them, so each round carries fewer of them than the last and the states
  swing between two shapes forever — which ended in the path-insensitive fallback and lost every guard
  in the function at once. `tests/nested_loop` still proves its two guarded increments and still
  reports the one unguarded one, with the same witness.
- **A declaration no longer outranks what a read learns.** A symbol registered before any read of its
  place — widening registers the symbols its relation mentions — carried no type, and the read that
  knew the place's type could not replace it. A model then answered `-2` for a `u8` local: a witness
  nobody can replay, on an obligation the place's own type rules out. An entry with no domain is now a
  placeholder a read replaces.
- **An obligation that cannot be decided says so, with the reason.** `fixed_array`'s index comes out of
  `Range::next`, a call this analysis does not model; that obligation is reported
  `Undecided … not modelled` rather than as a counterexample — not claimed to fail, and not claimed to
  hold either. A model that leans on an unmodelled value is not a bug report.

### The probe is an assertion, not a pin

- **`tests/potency_probe` exits `0`.** Ten functions of safe Rust, nothing in it can panic; the three
  that used to be reported as counterexamples are no longer reported as failures — two are decided
  `PASS` and the third is reported undecided with its reason named (below). The block in
  `tests/run_tests.sh` carries
  no `[KNOWN]` line any more: the exit code, the three `[PASS]` verdicts, the loop relation in the
  emitted obligation (`mir_3 = 0 ∨ (prev guard ∧ mir_3 = prev_bb6_phi_bb1_3 + 8)` next to
  `(<= (+ mir_3 8) len_1)`) and the one named undecided reason are all asserted, so a regression fails
  the suite instead of being described.
- The case is still kept out of the page manifest, so a counterexample from a crate that cannot panic
  is never published as a finding.
- Test counts after this pass: 130 unit tests; `tests/run_tests.sh` **179 PASS, exit 0** (171 before).

## Unreleased — 2026-10-12 (overhaul pass)

The six planning documents — `ROADMAP.md`,
`FORMAL-VERIFICATION-BUILD-PLAN.md`,
`Additional-Enhancements.md`, `ASSESSMENT.md`,
`IMPACT-OPPORTUNITY.md` and `BUCKET-LIST.md` — are
consolidated into `OVERHAUL.md` (and subsequently into [PROGRESS.md](PROGRESS.md), [DONE.md](DONE.md), and [TODO.md](TODO.md)), which was then the single work list and status
artifact, with every row checked against the code rather than against another document. That
check corrected the register in both directions: **A7**, **B9**, **C6** and **F2** were already
implemented while still marked open, and the rows below were genuinely missing.

### No default run can hang any more

- **The per-obligation solver budget now has a default (30000 ms).** Z3's nonlinear *integer*
  arithmetic is not a decision procedure for every shape this encoder emits, and with no budget a
  pathological obligation simply never returns: `tests/undecided` had not finished after 16
  minutes, which is a CI job that reports nothing at all rather than a verdict. Measured after the
  change, with no budget flag given: **exit 0 in 31 s**, the expected `WARN`, and the run completes.
  `--solver-timeout 0` is how a caller asks for no limit, so the two meanings are now distinct;
  the value in force and whether it came from the flag or the default are recorded in
  `report.run.json` under `settings`.
- **A missing pinned toolchain is named instead of read as a version mismatch.** The verifier
  links against one exact nightly, so `keller verify` checks once, before `cargo` starts, that the
  toolchain it was built against is installed, and otherwise exits `3` with the name and the
  `rustup toolchain install … --component rustc-dev --component llvm-tools-preview` line. The
  listing is matched by name and not by substring, so a check for `nightly-2026-07-24` cannot be
  satisfied by a bare `nightly`.

### A proof is evidence now, not a verdict to be trusted

- **Every verdict leaves an obligation**, not only the failures: a proved assertion writes
  `PASS (unsat)`, so the same file that lets a third party re-check a counterexample lets them
  re-check a proof. Re-discharging `nested_walk_bb4.smt2` answers `unsat`; both runners now assert
  that direction for the proved sites, and that only a `FAIL` pins a witness.
- **The obligation header is self-locating.** It carries `kind`, `file:line` and the report's own
  `details` line beside the crate, function, block, verdict and guard, so the artifact and the
  report describe one site the same way. `keller explain` prints that header verbatim instead of
  re-summarising it — a summariser is one more place for the two to disagree.

### A first run can be read

- **`--profile advisory|strict`** names the gate contract instead of leaving it to two flags: a
  buyer reading a CI file should not have to infer what `--deny-warn --strict-coverage` promises.
  `strict` on `tests/safe_code` exits `1` where `advisory` exits `0`, and a misspelled profile is
  refused with exit `2`.
- **`--only <PATTERN>` and `--max-findings <COUNT>`** make a wall of findings reviewable without
  ever weakening the verdict. `--only folded_add` on `tests/fold_fail` reports and gates the one
  matching finding, and the summary says `scope: partial` with the five it did not look at, so a
  narrowed run cannot be read as a statement about the whole crate. `--max-findings 2` lists two of
  six and still exits `1`: the gate judged all six, and `counts.not_listed` says so.
- **Coverage is the headline, not a footnote.** Every run prints one line — `5 of 7 encountered
  assertion(s) decided (71.4%)` — and `--json` carries a `coverage_summary` object with the same
  numbers plus the names of the assertion kinds that were skipped, which is the shortlist of what
  to model next.

### Version and documentation

- `Cargo.toml` is `1.0.0`, which is what the `v1.0.0` tag now means; `keller-verify --version`
  agrees with both.
- [docs/RELEASE.md](docs/RELEASE.md) is the single support and assurance matrix — platform,
  toolchain, solver, profile, exit codes, checked and unmodelled assertion kinds, and what a
  passing run does not prove. [README.md](README.md) and [docs/USAGE.md](docs/USAGE.md) link to it
  instead of restating it.
- **The fixture runner builds the binary it is about to test.** It used to build only when
  `target/debug/keller-verify` was missing, so a source edit that only `cargo check` had seen was
  tested as though it had been applied — which is how this pass first "verified" a change that was
  not in the binary at all.
- The integer encoder has property tests (D5): `overflow_detected` is compared against an
  independently written reference over every integer width and operator, with a fixed-seed
  SplitMix64 and no new dependency, and the shift rule is pinned in both directions (a truncated
  value is not a panic; an out-of-range amount is).
- Test counts: 130 unit tests (from 89); `tests/run_tests.sh` 171 PASS, exit 0.

## Unreleased — 2026-10-12

### Verification runs on the compiler it was built against

- **A project that pins its own toolchain is analysed instead of refused.** `cargo` is a rustup
  shim, and the shim follows the toolchain of the project being checked: a `rust-toolchain.toml`
  pinning `stable` made `cargo -Zjson-target-spec` fail with `error: the -Z flag is only accepted on
  the nightly channel` before a single assertion was lowered. The run ended as a setup failure (exit
  `3`) with no verdict, on code that compiles cleanly, and nothing in the message said which
  toolchain was wanted. Eight of the fifteen crates this was tried on failed that way, and every one
  of them analysed once `RUSTUP_TOOLCHAIN` named the linked nightly by hand. `build.rs` now records
  the toolchain name beside the compiler path it already embedded (`KELLER_DRIVER_TOOLCHAIN`,
  derived from the rustc rustup named by `host_env::toolchain_name`), and `keller verify` selects it
  on the `cargo` it spawns. The project's own pin still governs its editor and its own builds; only
  the analysis run overrides it, because the MIR the verifier reads is the MIR exactly one compiler
  emits. `tests/stable_pin` is the fixture: a crate with its own `toolchain.channel = "stable"` pin,
  which must exit `1` with its one real counterexample rather than `3`, with the guarded increment
  beside it proved.

## Unreleased — 2026-10-09

### Trust foundation

- Versioned start/completion receipts from every compiler worker, including clean crates; malformed, incomplete and inconsistent receipt sets fail closed with setup exit 3.
- Fresh private Cargo target directories prevent unchanged reruns from losing analysis to fingerprints; compiler flags are preserved, and normal build caches are untouched. Dependencies are rebuilt intentionally.
- Clean completed runs write `report.json` as `[]`. Required report/environment/run/obligation I/O errors propagate instead of silently succeeding.
- `report.run.json` and JSON coverage expose worker/function/assertion/verdict counts, primary solver-call counts, actual worker compiler paths/versions/arguments, budgets and explicit model limitations.
- Previously skipped reachable assertion kinds become `unsupported-assertion` WARNs. `--strict-coverage` rejects missing/undecided coverage even when a baseline records it. The safe Vec fixture now exposes pointer null/alignment gaps and correctly fails under `--deny-warn`.
- Rust truncating signed division/remainder propagation, checked exhaustively over valid i8 operand pairs; signed/unsigned i8/u8 overflow differential tests and SMT boundary tests.
- `keller explain <FILE>` and `keller replay <FILE>` expose obligation metadata and check fresh solver results against recorded verdicts. Replay is model validation, not executable-counterexample validation.
- New CLI trust regressions cover unchanged reruns, empty reports, strict baseline interaction, arithmetic derived from division/remainder, compiler flags, workspace aggregation, replay mismatch/errors and unusable outputs.
- The measured-page harness preserves the original report across gate probes; regenerated page includes unsupported checks instead of silently claiming they were decided.

## Unreleased — 2026-10-08

### Added

- **A verdict says which toolchain and Z3 decided it.** `report.json` recorded *what* was found and
  nothing about *what found it*, so a report committed or uploaded as an artefact could not be
  attributed to the compiler and solver that produced it — and a `FAIL` that a newer Z3 decides
  differently would read as a regression in the code. Every run now also writes
  **`report.environment.json`** beside the report: the `rustc` the driver dylibs came from, its host
  triple, the `rustlib` directory, the Z3 binary and where it was found (`Z3_HOME`, `.z3/current`,
  the platform default or `PATH`), and the directories prepended to `PATH`. The facts are the ones
  `src/host_env.rs` already computes for `build.rs` and `keller-env`, so what is recorded is the
  environment the verifier was linked and run against rather than a second opinion about it, and a
  clean run writes it too — nothing to report is not nothing to attribute. It is a separate file
  because `report.json` is a bare array that is byte-compared against the committed goldens and
  these facts are absolute paths: an `env` block inside it would turn every golden difference into
  machine noise and fail the suite on any other machine. Both runners assert the sidecar in both
  directions — every key `keller-env` prints is present, and `report.json` has not grown the block
  back.
- **Loop bodies are analysed under the loop's own guards.** A block inside a cycle used to be cut
  out of the path-sensitive analysis — every one of them got `PathState::initial()` — so a loop body
  was checked path-insensitively: its iteration variable was a free `u8`, and `i + 1` was flagged at
  255, a value the loop's own guard `i < steps` makes unreachable. A cycle is now unrolled to its
  back edge (`MAX_LOOP_TRIPS` trips are modelled exactly), and past that bound `PathState::widen`
  forgets the loop-carried *values* while keeping the guards, so a loop body is asked its question
  under the guards that actually lead into it while the result stays an over-approximation — a
  forgotten value is re-derived on a read as a fresh symbol bounded by the place's type — and a term
  stops growing once per trip. In `tests/loop_fail` the counter increment of `accumulate` is now
  *proved*, and so is the new `count_to`, the loop-counter pattern on its own, where the same body
  used to be a false FAIL naming `i = 255`; the accumulator's overflow stays a counterexample
  (`acc = 250` over six trips walks 250..=255, so dropping it would call a reachable overflow safe)
  and its witness is `acc (_1) = 255`, the value the obligation pins. The golden was re-recorded, and
  the two new unit tests pin the widening rule.
- **A witness names only what the obligation forces.** A SAT model completes every symbol it can, so a
  FAIL's `witness` used to list values the solver had merely *chosen* — the loop body above reported
  `steps (_2) = 1` and `i (_3) = 0` beside `acc (_1) = 255`, although `i < steps` admits a range for
  both, and two runs of the same check could print two different pairs. A witness entry is now either
  an operand of the operation that would panic — which is what the report is about, and which for a
  relational failure like `a + b` keeps both of its operands even though neither is forced — or a
  value the obligation *pins*: the solver is asked whether any other value satisfies the same
  obligation, and an unpinned one is left out. The guard that bounds the dropped values is still
  carried, as a formula, in the obligation's `; reachable:` line. Every constraint builder used to
  append the path's own inputs to the variables it returned, which is what made the two kinds of entry
  indistinguishable; naming the path's inputs is now the caller's job alone. `tests/loop_fail` pins
  both directions, and three unit tests cover the pinned, unpinned and relational cases.
- **A nested loop is checked under its guards now: the cycle widening bounds the *condition* as well as
  the values.** `tests/nested_loop` is an inner loop inside an outer one, with `continue` in the inner
  body and `break` in the outer, and it is the shape that used to fall back to the path-insensitive
  state: `PathState::widen` forgot the loop-carried values but kept the path condition, and at a join
  that condition is a disjunction over the incoming paths — one of them the back edge, carrying the
  previous round's condition — so a header reached by a *conditional* edge re-wrapped its condition once
  per round and grew instead of settling (about 8 KB, 33 KB, 123 KB and 383 KB after three, four, five and
  six rounds). The round cap fired long before a fixpoint and `PathAnalysis::compute` took its documented
  last resort, `PathState::unknown()` for every block, so all three of the body's increments were
  reported. `compute` now rebuilds a widened cycle block's in-state from its *forward* contributions
  alone, so the back edge no longer re-feeds the block's condition into itself — a dropped back-edge
  disjunct is a subset of a forward one, so the union is unchanged and the result is the fixpoint. In
  `tests/nested_loop` that proves both guarded increments (`inner + 1` under `inner < cap`, `outer + 1`
  under `outer < limit`) and leaves only the accumulator, which no guard bounds, as a counterexample; the
  single-cycle control is proved too. The fixture asserts both directions, the counterexample's
  obligation must carry both headers' guards, and a unit test pins the forward-edge rule. Registered as
  A15, and `tests/goldens/` holds nine, one per reporting case.
- **A lost precision is reported instead of only showing up as a false positive.** `PathAnalysis`
  falls back to the path-insensitive state when its round budget runs out without a fixpoint. That is
  sound, but it costs the function every guard at once, and until now its only outward sign was a
  `FAIL` the guards would have ruled out — a false positive with nothing in the report to attribute it
  to. The fallback is now recorded against the function as a `WARN` with `"kind": "path-insensitive"`
  and `"block": "body"`, naming the round budget that was hit and stating the direction that matters:
  the `FAIL`s in that function are the suspect verdicts, while a `PASS` there is still a proof, because
  forgetting a guard only ever weakens the precondition. It is announced on the console, counted in
  the summary (and in the `--json` summary as `counts.path_insensitive`), given a SARIF rule of its own,
  and fatal under `--deny-warn` like any other `WARN`. The budget is overridable with
  `KELLER_MAX_ROUNDS`, which is what makes the record reproducible and lets a shape the cap cut short
  be given more room. No fixture trips the fallback by default — the dataflow really does converge —
  so `tests/nested_loop` exercises it by cutting the budget to one round: the two increments the default
  budget proves come back as `FAIL`s, and the report has to say why. Three new unit tests pin the note,
  the event and the gate policy, and the SARIF and summary tests were extended.
- **Command-line surface.** `keller verify` now accepts `--solver-timeout <MS>`, `--baseline <FILE>`,
  `--write-baseline <FILE>`, `--sarif <FILE>`, `--json`, `--quiet`, `--help`/`-h` and
  `--version`/`-V`, alongside the existing `--deny-warn`. `--help`/`--version` work as the first
  argument and before or after the subcommand. The parser is covered by unit tests, so the flag
  surface a CI file talks to no longer depends on manual checking.
- **Exit code `3`** for a setup failure — `cargo check` did not complete, or `cargo` is missing —
  instead of reporting it as `1`, which is the code that means "a counterexample was found".
  `--version` prints the version from `CARGO_PKG_VERSION`.
- **Per-finding source locations and classes** in `report.json`: `file` and `line` (omitted when the
  compiler gave no usable span, never guessed) and a machine-readable `kind` (`bounds`, `overflow`,
  `overflow-neg`, `divzero`, `remainder`). `report.json` stays a bare JSON array, so existing
  consumers keep working.
- **SARIF 2.1.0 output** (`--sarif <FILE>`) for the findings the gate acted on, with a rule per
  class, `error`/`warning` levels, logical and physical locations, and a
  `crate::function::block` fingerprint, so a finding can be annotated on the pull request that
  introduced it.
- **Baselines** (`--write-baseline <FILE>`, `--baseline <FILE>`), so the gate can be switched on for
  a codebase that is not clean yet: record what is already there once, fail only on new findings,
  and shrink the file as the old ones are fixed. Matching uses crate/function/block/status only, so a
  reworded message or a changed witness is still the same obligation. The file is versioned, and an
  unreadable or unknown-version baseline exits `2` rather than silently gating with nothing.
- **`--json`** prints counts and every finding on stdout (implying quiet workers, so stdout carries
  only the JSON) for pipelines that would otherwise have to read a file out of the verified project.
- **`tests/shift_amount`**, an eighth fixture crate, plus committed golden reports under
  `tests/goldens/` that both runners compare the whole `report.json` against, and a check that every
  finding carries a source file and a class.
- **`tests/fold_fail`** (six constant-folded failure sites) and **`tests/broken_code`** (a crate that
  does not compile), taking both runners to ten wired fixture cases, plus a golden for the folded
  report — `tests/goldens/` now holds eight.
- **A generated landing page.** `index.html`'s figures and obligation rows — the panel header, the
  rows and their verdicts, the caption, the four readouts, the reproduced obligation and the Status
  counts — are rebuilt from the fixture run by `tests/gen_index.py`, and `tests/run_tests.sh` diffs
  the committed page against that run, so a published number cannot drift from the measurement
  behind it (`UPDATE_INDEX=1 tests/run_tests.sh` re-records it, `tests/gen_index.sh --check` runs the
  diff alone). The prose stays hand-written, and the file's header says which passages the page does
  not claim to measure.
- **`tests/undecided` — the fixture that pins the other half of the exit-code contract.** The suite
  could assert what `--deny-warn` does to a *clean* run and to a *counterexample*, but not to the
  verdict the flag exists for: no fixture produced one, so `WARN` was covered only by unit tests over
  the policy. This crate produces a real undecided verdict. Its `guarded_sum` has two obligations: the
  checked `a * b` under the weight bound is proved, and `step + 1` sits behind
  `product == 2^61 - 1 && a > 1 && b > 1`, an equality with no solution because 2^61 - 1 is prime — a
  fact about factorisation rather than about that arithmetic, which is exactly what Z3's nonlinear
  *integer* reasoning cannot establish. The multiply is checked rather than `wrapping_mul` on purpose: a
  wrapped call's result never reaches the solver as a term, and the case would collapse into an ordinary
  `FAIL` naming `step = 255`. Both runners run it twice under `--solver-timeout 5000` and assert both
  sides: exit `0` with the `WARN` tolerated (announced on the console, recorded as `"status": "WARN"`
  with `"kind": "overflow"`, attributed to `guarded_sum` and to no other crate, and never laundered
  into a counterexample), and exit `1` under the flag with the gate naming the half that failed. It
  writes no pinned witness, because `unknown` has no model to pin — a rule the suite checks rather than
  assumes — and its `report.json` is compared byte for byte against a new golden. The budget is what
  keeps the case bounded and what separates its two obligations: with no budget Z3 does not answer
  `unknown`, it does not answer at all (still running after 90 s), while the bound multiply is decided in
  about 0.4 s. It is also the first case whose exit-0 run carries a finding, so it declares its expected
  `WARN` count in the fixture-run manifest, and the generated page now shows that obligation as its
  single amber (undecided) row — 34 rows, one of which the run did not decide — instead of saying that no
  `WARN` was recorded. `tests/goldens/` now holds ten.

### Changed

- **The build script and both test runners now share one toolchain/Z3 lookup.** The rule for "where are
  the rustc driver dylibs and which Z3 will answer?" existed four times and the copies had drifted:
  `build.rs` took the *last* of several sorted releases unpacked under `.z3/`, `tests/run_tests.sh` took
  the *first* `.z3/*/bin` entry with a runnable `z3`, `tests/run_tests.cmd` did not glob at all and fell
  back to a hardcoded `C:\z3-4.16.0-x64-win` without checking that it would start, and
  `tests/gen_index.py` carried a fourth. A checkout with two releases unpacked could be *linked* against
  one and *re-discharged* by the other, and a solver that existed but did not start was a silent `[SKIP]`
  on one host and a spurious failure on another. The decision now lives in `src/host_env.rs`: `build.rs`
  includes it, so the verifier links against the release the suite will run, and a new second binary,
  `src/bin/keller-env.rs`, prints it as `KEY=VALUE` lines — `RUSTC`, `HOST`, `RUSTLIB`, `Z3_BIN`,
  `Z3_SOURCE`, `PATH_ADD`. Both runners consume that instead of looking for themselves, the shell one
  asking for `--shell sh` (quoted assignments, `/c/...` paths) so it can simply `eval` them, and the page
  generator asks the same helper for the solver it re-discharges with. The order is `KELLER_Z3` → a `z3`
  already on `PATH` → `Z3_HOME` → the project-local `.z3/` (`.z3/current`, else the last of the sorted
  releases, exactly as the build picks it) → the platform default. The helper is a separate binary and
  uses nothing but `std` on purpose: the verifier is dynamically linked against Z3 and loads the
  toolchain's driver dylibs at run time, so it cannot start until the directories the helper reports are
  already on `PATH`. A missing toolchain is now reported once, before the first case, instead of as
  twelve cases that each died at load with exit `127` (Windows reports `STATUS_DLL_NOT_FOUND`). A unit
  test — `the_runners_do_not_look_for_the_toolchain_or_z3_themselves` — scans both scripts for the
  primitives a lookup would need, comments excluded, and fails the build if either grows one again; it
  was verified to bite by planting a lookup in the Windows runner and watching it fail. `cargo test`
  runs 89 tests, `tests/run_tests.sh` 129 PASS and `tests/run_tests.cmd` 63 PASS, all exit 0.

### Fixed

- **Shifts are now checked on their amount, exactly.** `a << b` and `a >> b` panic when `b` is
  negative or at least the width of `a` — and *only* then. Two bugs came out of the old treatment:
  a shift with a symbolic amount was not checked at all, and the constant path called a
  *value-truncating* shift an overflow, so `200u8 << 1` (which Rust silently truncates to `144`) was
  reported as a counterexample for a program that cannot panic.
- **An obligation the encoder cannot express is reported instead of skipped.** It is emitted as a
  `WARN` reading `Not modelled: …`, and an operation that provably cannot fail (unsigned division
  overflow) is reported as a proved `PASS` rather than as silence. Previously both were dropped
  without a trace, which read as "checked and fine".
- **Constant-folded failures and shift witnesses** now report the operand that actually fails: for a
  symbolic shift amount, the witness names the amount, not the shifted value.
- **A crate that does not compile is no longer reported as a clean run.** The wrapper discarded the
  status `rustc_driver::catch_with_exit_code` returned, so a crate rustc rejected exited `0` with an
  empty `report.json`: the analysis had found no assertions in a body that was never built, and "no
  findings" was indistinguishable from "nothing to check". It exits `3` now. The reason it could
  happen at all is worth recording — the verifier reads `optimized_mir` for every function, so it
  runs MIR passes that plain `cargo check` skips, which is how a deny-by-default lint such as
  `arithmetic_overflow` surfaces for code `cargo check` accepts and `cargo build` rejects; the
  wrapper follows the build. `tests/broken_code` pins the exit code.
- **A constant-folded failure is no longer a second-class FAIL.** When both operands are constants
  the folding pass decides without asking Z3, and those findings carried no structured `witness` and
  wrote nothing under `obligations/` — so the rule that every FAIL names a concrete input, and can
  be re-discharged independently, held only for the obligations Z3 happened to decide. They now
  carry a witness and write a ground obligation whose header records the verdict as
  `FAIL (folded)` and which replays to `sat`. It is a restatement rather than the formula Z3 was
  handed — the pinned operands and the violation predicate in the encoder's own shape — because
  there was no query. New unit tests cross-check each ground instance against Z3 itself. A
  folded `u128` is named in its own domain (`u128::MAX`, not the `-1` it is carried as).
- **The emitted obligations are re-discharged, not skipped.** The workflow unpacks Z3 under `.z3/`
  without exporting `Z3_HOME`, and the runner looked for the solver only in `PATH` and `$Z3_HOME/bin`
  — so every obligation check *under that configuration* printed `[SKIP]`, and the claim that a
  counterexample is independently checkable was enforced locally and skipped under the configuration
  the workflow installs. The runner now resolves Z3 the way `build.rs` resolves the library:
  `Z3_HOME`, then the project-local `.z3/` installs. (That configuration was reproduced by running the
  suite with the workflow's environment; no run of the workflow has been observed from this checkout
  — see the CI note in the entry above.)
- **A constant-folded FAIL prints its block like every other FAIL.** It formatted a `String` with
  `{:?}`, so the line read `("bb0")` where the symbolic path prints `(bb0)` — the same kind of event
  in two shapes for a reader or a script to reconcile.
- **The landing page's caption is readable.** It was a single SVG `<text>` line of about 1,100
  characters; SVG text does not wrap and the 900-unit viewport clipped it, so almost every sentence
  explaining what each fixture pins down never appeared. It is HTML below the SVG now, where it wraps.
- **The shell runner can find the pinned toolchain again.** `tests/run_tests.sh` built the path to the
  rustc driver dylibs by piping `rustup which rustc` through `xargs dirname`. On Windows that answer is
  a backslash path, and on this machine it also contains a space, so `xargs` split it, `dirname` got a
  fragment, the directory was not found and nothing was added to `PATH` — the wrapper then died at load
  time with `STATUS_DLL_NOT_FOUND`, which the shell reports as exit `127`, and every case looked like a
  verdict failure rather than the setup problem it was. The path is now quoted and normalised, and put
  through `cygpath` when it is a Windows form an MSYS shell will not hand to a spawned process; Linux CI,
  where it is already a POSIX path, is unaffected. (The lookup has since moved out of the script
  altogether — see the Changed entry above — so what remains here is why the shared one exists.)

### Removed

- The `rustc_tools_util` dependency, which no code referenced. The version comes from Cargo.
