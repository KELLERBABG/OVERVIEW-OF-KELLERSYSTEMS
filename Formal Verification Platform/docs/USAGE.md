# Keller Verify - Usage Guide

## How to run

See [RELEASE.md](RELEASE.md) for the supported platform, toolchain and solver baseline, the
installer, and the assurance limits — this guide describes how to use the tool, not what is
supported. Other OSes, toolchains, Z3 releases and targets have not been clean-install validated.


Build the verifier once, then run it from the project you want to check:

```bash
cd "Formal Verification Platform"
cargo build            # add --release for a faster binary

cd /path/to/your-rust-project
/path/to/keller-verify/target/debug/keller-verify keller verify
```

On Windows the binary is `target\debug\keller-verify.exe` (use `target\release\...` if you built
with --release). The repository also provides keller-verify.cmd, a portable launcher on
Windows that queries keller-env.exe to configure runtime library and toolchain paths. The
supported launcher is the one the [source installer](RELEASE.md#install-from-a-clean-checkout)
generates: it resolves the pinned toolchain and the Z3 library directory, and with no arguments
prints Keller's usage rather than falling through to rustc's.

> **PATH note.** The verifier is built against the pinned nightly and loads that toolchain's rustc
driver libraries at run time, so the toolchain's `lib/rustlib/<host>/lib` directory has to be
findable. Neither test runner works that out for itself any more: both ask `keller-env`, a second
binary from the same `cargo build`, whose answer comes from `src/host_env.rs` — the same decision
`build.rs` links against, so the Z3 the suite re-discharges with is the Z3 the verifier was built
against. The solver is looked for as `KELLER_Z3`, then a `z3` already on `PATH`, then `Z3_HOME`,
then the repo-local `.z3/` install, then the platform default; the driver dylibs come from the
toolchain rustup names for the checkout. A missing toolchain is a fatal setup error before the
first case runs, rather than every case dying at load one at a time. Run `target/debug/keller-env`
yourself to see what it resolved — the solver, where it came from, and the directories to prepend to
`PATH` — and if the binary cannot find `libz3`, set `Z3_HOME` and put `$Z3_HOME/bin` on `PATH`
yourself.

## The toolchain the analysis runs under

`keller verify` drives `cargo check` with the compiler this verifier was built against, not the
toolchain the project under verification pins. `cargo` is a rustup shim, and the shim follows
`rust-toolchain.toml`; since the verifier lowers and reads the MIR of one exact compiler, a project
pinned to `stable` (or to another nightly) is asked its question by that compiler anyway. The
verifier sets `RUSTUP_TOOLCHAIN` to the toolchain it was built with when it spawns cargo, so the
project's own pin still governs its editor, its tests and its builds, and only the analysis run
overrides it.

Consequences worth knowing before the first run:

- The project has to compile with that compiler. A `rust-version` higher than the pinned nightly's, a
  target that nightly does not carry, or `edition = "2024"` on an older pin is reported as a setup
  failure (exit `3`), not as a verdict, and the rustc that refused it is named in the output.
- Nothing about the verified project is edited: no `rust-toolchain.toml` is written and no
  environment variable is exported into your shell.
- The analysis is always the `dev` profile, because that is where rustc emits the overflow and
  bounds assertions this tool checks. See [RELEASE.md](RELEASE.md).

## Options

```
keller verify [OPTIONS]
keller init [--force]
keller explain <OBLIGATION.smt2>
keller replay <OBLIGATION.smt2>
keller reproduce <OBLIGATION.smt2> [--out <DIR>]
```

| Flag | What it does |
|---|---|
| `--profile <NAME>` | The named gate contract: `advisory` (the default — gates only on a counterexample) or `strict` (also fails on unsupported checks and undecided verdicts, i.e. `--deny-warn --strict-coverage`). Flags given after a profile still win, so a profile is a starting point rather than a mode. What each profile promises is stated once, in [RELEASE.md](RELEASE.md#gate-profiles-and-exit-codes). |
| `--strict-coverage` | Fail on WARNs and assertions without a verdict. Coverage/undecided WARNs remain fatal even if listed in a baseline. This covers encountered MIR assertions, not every possible panic or unmodelled operation. |
| `--deny-warn` | Treat an undecided (`WARN`) verdict as a failure, so a build cannot pass without a definite answer for every assertion. |
| `--only <PATTERN>` | Report **and gate** only findings whose `crate::function` contains `PATTERN` (case-insensitive). Findings outside the filter are neither reported nor gated, and the summary prints `scope: partial`, so a narrowed run cannot be read as a statement about the whole crate. |
| `--max-findings <COUNT>` | List at most `COUNT` findings — on the console, in SARIF and in the `--json` summary. It is a listing cap and nothing else: the gate still judges every finding, so it can make a large first run readable but can never pass a counterexample. `0` is refused. |
| `--solver-timeout <MS>` | Per-obligation solver budget. A budget that runs out makes Z3 answer `unknown`, which is reported as `WARN`. The default is **30000 ms**, so no default run can hang on a pathological obligation; the value in force and whether it came from the flag or the default are recorded in `report.run.json` under `settings`. `--solver-timeout 0` is how a caller asks for **no** limit. |
| `--run-timeout <SECONDS>` | Wall-clock cap on the whole analysis phase, build included. A run that reaches it is stopped, the process tree it started is killed, and it reaches **no verdict**: exit `3`, with the stopped run recorded as `"status": "run-timeout"` under `settings`, the crates that were still in flight named on stderr, and nothing published over the last completed `report.json`. The default is **0 s**, i.e. no cap: a cap is a deliberate bound for a crate whose honest run time has been measured, and `--run-timeout 0` states the default explicitly. It exists because a solver budget bounds one query and not a run: measured here, one crate ran past 20 minutes with `--solver-timeout 5000` set, and another ran for minutes with no solver budget set, an unbounded Z3 call (K3). |
| `--baseline <FILE>` | Report findings normally, but do not fail the build on the ones listed in `FILE`. See the roll-out recipe below. |
| `--write-baseline <FILE>` | Record the current findings in `FILE` (sorted, de-duplicated) and exit `0`. Recording what is already known is not a verdict, so this never fails a build. |
| `--sarif <FILE>` | Also write a SARIF 2.1.0 log of the findings the gate acted on, for GitHub code scanning or any other SARIF consumer. |
| `--json` | Print the summary — counts, the run's coverage, the scope and the findings — as JSON on stdout. The listing is capped by `--max-findings` and narrowed by `--only`, and `counts` says how many were kept out. Implies `--quiet` for the workers, so stdout carries the JSON and nothing else. |
| `--quiet` | Print findings and the final verdict, not progress. A failing verdict is **never** suppressed. |
| `--help`, `-h` | Print the usage. Works before or after `verify`. |
| `--version`, `-V` | Print the version. Works before or after `verify`. |

An unrecognised option is **refused** (exit `2`) rather than ignored, and a near miss of a real flag
says so: `--deny-warns` is answered with "Did you mean `--deny-warn`?". A `--profile` name that is
neither `advisory` nor `strict` is refused the same way, and `--max-findings 0` is refused rather than
read as "list nothing".

`--baseline` and `--write-baseline` cannot be combined: one reads a baseline and the other writes
one, so accepting both would mean one of them silently did nothing.

### Gate profiles

```bash
keller verify --profile strict        # the assurance gate: nothing left unanswered by design
keller verify --profile advisory      # the default: a counterexample fails, a WARN is recorded
keller verify --profile strict --no-cache
```

`advisory` is the default and sets neither `--deny-warn` nor `--strict-coverage`; `strict` sets both.
A profile is applied where it appears on the command line, so an explicit flag after it wins — which
is what makes `--profile strict` a safe default to put in a CI file and adjust per job. The two flags
remain the primitive contract; the profile is the name for it. The full promise of each, including
what a strict exit `0` still does not establish, is in [RELEASE.md](RELEASE.md#gate-profiles-and-exit-codes).

## Checking in a run policy (`keller.toml`)

Flags and `KELLER_*` environment variables describe one run; a repository also needs to record
*decisions* — which crates are gated, which are advisory, what the budgets are — somewhere a reviewer
can see them in a diff. That is `keller.toml`:

```bash
keller init            # write a starter keller.toml in the working directory
keller init --force    # replace one that is already there
```

The file `init` writes is **inert**: every setting in it is commented out, so the run immediately
after it behaves exactly as the run before it. An initialiser that started gating on its first run
would make the tool look broken in the one place a new user tries it. Without `--force`, `init`
refuses to replace an existing file (exit `2`) rather than overwrite decisions someone wrote down.

```toml
profile = "strict"          # or "advisory"; the same two named contracts as --profile
deny_warn = true            # the individual settings a profile stands for
strict_coverage = true
solver_timeout = 30000      # ms, 0 means no limit
run_timeout = 1800          # s, a cap you chose; 0 (the default) means no cap
only = "crate::function"    # narrow a run while adopting the tool
max_findings = 50
baseline = ".keller-baseline.json"
sarif = "keller.sarif"
no_cache = false            # true is the same as --no-cache
keep_runs = 10

[[policy]]                  # per-crate policy; repeat the table once per crate
crate = "legacy_crypto"
gate = "advisory"           # or "strict"
```

The file supplies **defaults**, and the command line is applied on top of it — including when the two
disagree. A config that says `profile = "strict"` cannot stop you typing `--profile advisory`, and a
setting the file never mentions is untouched by it; a run policy that silently overrode what someone
typed would be a worse trap than no policy at all. Values the file does not set are the tool's own
defaults, not a second copy of them here.

An unknown key, an unknown table or a value of the wrong type is an **error** naming the file and the
line, not a silently ignored line:

```
[Keller-Pipeline] keller.toml:1: unknown key 'strict_covrage'
```

A `[[policy]]` with `gate = "advisory"` keeps a crate's findings **reported** — they stay in
`report.json`, on the console and in SARIF — while keeping them out of the gate, so a crate nobody has
cleaned up yet does not block a repository that is otherwise ready to gate. The count is recorded in
`report.run.json` under `gate.advisory_excluded`, and the excluded findings are named on stderr, so
"why did this exit 0" is answerable from the run record rather than by reading the policy file again.

## Output

### `report.json`

**`report.json`** is written to the **current working directory** — the project you ran
`keller verify` from — and contains all `FAIL` and `WARN` events, of the functions `--only` selected
when it was given. It is never capped by `--max-findings`: that changes the listing, never the record.

> **Note:** each crate-compilation worker files its findings under `.keller-reports/`, and the
> top-level command merges every one of those files into `report.json`. A multi-crate workspace
> therefore keeps every crate's findings, and each entry is tagged with its `crate`. The merge is
> sorted and de-duplicated, so identical runs produce byte-identical reports.

A successfully completed clean run writes `[]`; an absent report is not a clean verdict.
The file is a **bare JSON array** of events (deliberately: existing consumers parse it as one).
Each event carries:

| Field | Meaning |
|---|---|
| `crate` | Which crate the finding came from. |
| `function` | The function's path, e.g. `add`. |
| `block` | The MIR basic block the assertion sits in, e.g. `bb0`. |
| `status` | `FAIL` or `WARN`. (`PASS` is printed, not recorded: the file is a list of problems to act on.) |
| `kind` | The class of the obligation, as a stable token: `bounds`, `overflow`, `overflow-neg`, `divzero` or `remainder`. Group on this rather than on the prose in `details`. |
| `file` | The source file the function was defined in, relative to the project root, with forward slashes. Omitted when the compiler gave no usable span (a macro expansion, for instance) — never guessed. |
| `line` | The line the function starts on. Omitted together with `file`. |
| `details` | Human-readable description, including the witness values. |
| `witness` | The concrete inputs that trigger the finding: the operands the failing operation is stated over, plus each other symbol whose value the obligation **pins** — the solver is asked whether any other value satisfies the same obligation, and one that could have been anything else is left out rather than printed as though it were a fact. Recorded for **every** `FAIL`, whether the obligation went to Z3 or was decided by constant folding. Omitted for a `WARN`. |

### `report.run.json`: completion and coverage

Every successful analysis writes a versioned run record with `schema_version: 1`,
`completed: true`, tool/version, settings, environment, model limitations and `coverage`.
Coverage includes worker/function counts, reachable supported/unsupported assertions,
PASS/FAIL/WARN counts, assertions without a verdict and primary obligation solver calls
(excluding reachability and witness probes). The same coverage appears in `--json`, and its headline
also appears in the console, the HTML report and the SARIF log's run properties.
`coverage.panic_sites` is counted beside those totals rather than inside them: it is the number of
call sites whose panic this analysis did not decide (below), which is not an assertion it checked.
`coverage.panic_sites_decided` is the number it did decide, either as a proved non-panic or a ground
`FAIL`.
`coverage.compilers` records the **actual worker compiler path, `-vV` output and arguments**.
These records contain absolute paths and potentially private compiler flags; review before sharing.

Each run builds in a fresh private target directory, removed afterwards, so Cargo cannot skip
analysis after receipts are cleared. Compiler flags are preserved. This intentionally costs a
fresh dependency build; no performance improvement is claimed. Unchanged function verdicts may be
restored from a content-addressed cache, which carries the same events, counters and exact obligation
artifact bytes; `--no-cache` forces a cold analysis, and an entry that disagrees with its event count
is re-analysed instead of trusted. Worker start/completion receipts
are required even for clean crates. Missing, unfinished, malformed or inconsistent receipts,
and required artifact I/O failures, exit `3` rather than becoming a clean verdict.
The completion flag describes analysis completion, not whether the gate passed.

Reachable unsupported assertion kinds are `WARN` findings with `kind: "unsupported-assertion"`.
For example, `vec!` can introduce pointer null/alignment assertions that this encoder does not model;
the no-FAIL `safe_code` fixture now warns, and consequently fails under `--deny-warn`.

**A report is a list of what was checked, which is why the panics it does not check are counted
separately.** An `unwrap` or `expect` is decided when the receiver's enum variant is known: a known
failing variant (`None`, `Err`) is a ground `FAIL` with `kind: "decided-panic"` and
`defect_class: "known-panic"` — no witness and no solver call — while a known succeeding variant is
proved and counted in `coverage.panic_sites_decided`. Every other call into a body this analysis does
not read — `panic!`, `unreachable!`, `assert_eq!`'s failure path, the allocation-failure abort, and an
`unwrap`/`expect` whose variant is not known — is a `WARN` with `kind: "unmodelled-panic"`, naming the
callee, counted in `coverage.panic_sites`, and printed on one line of its own — so a crate full of
`unwrap`s cannot produce the `[]` a crate that cannot panic produces and read the same. Those warnings
are not failures (nothing was proved false, so the exit code is what it always was), never summed into
the assertion counts, and fatal only under `--deny-warn`. A call to a *function in the crate under
analysis* is deliberately not one of these: that body is analysed, so its panic is reported where it
happens rather than again at every call site. `tests/panic_sites` pins the six unmodelled shapes, the
two controls and the counts; `tests/panic_decided` pins the decided variants.

**Scope:** results concern the encoded MIR model, not absence of all panics. A SAT witness
satisfies that model; widening and unmodelled operations may make it impossible to reproduce as
an execution. Signed division/remainder now use Rust's truncation-toward-zero semantics;
bitwise values, narrowing casts, calls and loop invariants remain limitations.

### `report.environment.json`

Every run also writes **`report.environment.json`** beside the report — in the same working
directory — recording where the toolchain and the solver came from:

| Field | Meaning |
|---|---|
| `env_source` | Shared discovery source. Compiler identity is corrected from worker receipts; discovery alone is not an attestation. |
| `rustc` | Actual worker compiler path; verbose version and full compiler arguments are recorded in the run record. |
| `host` | That compiler's host triple, e.g. `x86_64-unknown-linux-gnu`. |
| `rustlib` | The directory holding the `rustc` driver dylibs the verifier loads at run time. |
| `z3_bin` | Discovered standalone replay executable. This does not attest the identity of the Z3 library loaded by the verifier. |
| `z3_source` | Where that solver came from: `Z3_HOME`, `.z3/current`, the platform default or `PATH`. |
| `path_add` | Directories prepended to `PATH` for the run. |

> **Why a second file, not a field in `report.json`.** `report.json` is a bare array on purpose
> (existing consumers parse it as one) and is byte-compared against the committed goldens — while
> these facts are absolute paths. An `env` block inside it would turn every golden difference
> into machine noise and fail the suite on any other machine, so the environment is written beside
> it instead. It is machine-specific by nature and is gitignored. A run that reported nothing
> still writes it: nothing to report is not nothing to attribute.

### `--json`

Prefer `--json` for anything programmatic: it prints the same events together with the counts and
the tool version, and does not require reading a file out of the verified project's directory.

```json
{
  "tool": "keller-verify",
  "version": "1.0.0",
  "analysis_profile": "debug (dev profile, rustc overflow/bounds assertions enabled)",
  "analysis_profile_note": "A release build wraps instead of panicking, so a PASS here is not a guarantee about a shipped release artifact.",
  "scope": "full",
  "counts": {
    "fail": 1, "warn": 0, "path_insensitive": 0,
    "suppressed": 2, "gated": 1, "filtered_out": 0, "not_listed": 0
  },
  "findings": [ { "crate": "…", "status": "FAIL", "…": "…" } ],
  "coverage_summary": {
    "encountered": 36, "decided": 35, "proved": 34, "counterexamples": 1,
    "undecided": 0, "unsupported": 1, "decided_ratio": 0.972,
    "unsupported_kinds": ["NullPointerDereference"]
  }
}
```

`analysis_profile` states which build the verdict is about, in the summary rather than only in
`report.run.json`, because a pipeline that quotes a proof in a pull request is quoting the *debug*
profile: a release build wraps instead of panicking, so "this cannot overflow" and "this will not
abort" are different claims. The same two fields appear in `report.run.json` and in the SARIF log's
run properties, and the console prints one line naming the profile before the verdict.

`fail` and `warn` count every finding in the report, `suppressed` counts the ones the baseline
covered, and `gated` counts the ones the gate actually judged. `filtered_out` is what `--only` kept
out of the run entirely, `not_listed` is what `--max-findings` kept out of the listing, and `scope` is
`partial` whenever `--only` was used. `coverage_summary` is the headline: how many encountered
assertions were decided and why the rest were not — `unsupported` is a kind this build does not model
(named in `unsupported_kinds`) and `undecided` is a solver that did not answer. The console prints the
same line once for the run, before the verdict.

### `obligations/`

**Every** verdict writes the exact formula it was decided from, as SMT-LIB, under
`obligations/<crate>/<function>_<block>.smt2`, so a verdict can be re-checked without trusting this
binary:

```
z3 -smt2 obligations/overflow_fail/add_bb0.smt2
```

Each header carries the crate, function, block, the obligation's `kind`, its `file:line`, the same
`details` line the report holds, the verdict, and a `; reachable:` comment naming the guards that lead
to the assertion — so the file answers *what was checked and where* without opening `report.json`.

The verdict recorded in that header is what an independent solver has to reproduce:

| Recorded verdict | Which reporting path | Must re-discharge to |
|---|---|---|
| `FAIL (sat)` | Z3 found a model | `sat` |
| `FAIL (folded)` | the constant folder found it without asking Z3 | `sat` (a ground instance of the same violation) |
| `PASS (unsat)` | Z3 proved the negation unsatisfiable | `unsat` — the proof itself, not a promise that one exists |
| `WARN (unknown)` | the solver did not decide | any valid answer: it is a budget result, not a formula property |

A `FAIL` writes two files: `<function>_<block>.smt2`, the formula behind the verdict, and
`<function>_<block>.witness.smt2`, the same formula with the reported values — the failing operands
and whatever the obligation pins — asserted. When the
obligation went to Z3, the first is the exact goal the solver was given. When the constant-folding
pass decided it without asking Z3 there is no such goal, so the file is the **ground** counterpart —
the folded operands pinned to their constants plus the violation predicate in the shape the symbolic
encoder uses — and its header records the verdict as `FAIL (folded)` rather than `FAIL (sat)`. Either
way an independent solver replays it, and the pinned file must answer `sat`; that is what the fixture
suite checks on every pinned witness.

Emission can be turned off with `KELLER_EMIT_SMT2=0` (recorded in the run settings).
Otherwise failure to write an obligation is fatal; no complete receipt is issued.

Beside those files, each crate's directory gets an `index.json` listing every obligation the run
emitted for it — including the **proofs**, which `report.json` leaves out because it is a list of
things to act on rather than a list of everything that was checked:

```json
{
  "schema_version": 1,
  "crate": "safe_code",
  "re_discharge": "z3 -smt2 obligations/safe_code/<artifact>",
  "obligations": [
    {
      "crate": "safe_code", "function": "main", "block": "bb10",
      "status": "PASS", "kind": "divzero", "verdict": "PASS (unsat)",
      "artifact": "main_bb10.smt2", "file": "src/main.rs", "line": 2
    }
  ]
}
```

One file instead of a directory glob, so a reviewer or a dashboard can read every verdict a run
reached from a single artifact — and so the proofs are *addressable*: an obligation nobody can find
is a proof nobody re-checks. `verdict` is the same string the obligation's own header records, so the
index and the file it points at cannot disagree about what was decided, and `pinned_artifact` names
the witness file when the FAIL had one. The entries are sorted and de-duplicated, so two runs of an
unchanged crate produce a byte-identical manifest and the file can be reviewed as a diff.

### Explain and replay

```bash
keller-verify keller explain obligations/overflow_fail/add_bb0.witness.smt2
keller-verify keller replay obligations/overflow_fail/add_bb0.witness.smt2
keller-verify keller reproduce obligations/overflow_fail/add_bb0.witness.smt2
```

`explain` prints the obligation's own header verbatim — so the kind, the `file:line` and the guard
are all in it — and then the verdict a replay must reproduce, rather than re-summarising what the
file already says. `replay` uses the discovered standalone Z3 with a ten-second budget and checks
its answer against the header. Matching SAT/UNSAT exits `0`; disagreement or unknown exits `1`;
an invalid header/argument exits `2`; unavailable solver or rejected SMT exits `3`.
Unknown obligations can be explained but cannot count as successful proof replay.
Replaying a folded instance checks its ground violation, not its earlier reachability probe.
Neither command is concrete execution validation.

### Console

`[PASS]` lines and the summary table go to **stdout**; `[FAIL]`/`[WARN]` lines and the
`[Keller-Pipeline]` diagnostics go to **stderr**. `--quiet` removes the progress and the pass
lines, and `--json` implies it, so a pipeline can read stdout as data.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Analysis completed and the selected gate passed; advisory WARNs may remain. This is not a no-panic guarantee. |
| `1` | The gate failed: a counterexample was found, or an obligation was left undecided under `--deny-warn`, or an encountered assertion was unsupported/undecided under `--strict-coverage` (both are what `--profile strict` sets). |
| `2` | The command line was not understood. This includes an unreadable or unknown-version baseline file, because a typo in that path must not become a gate that suddenly reports every long-known finding as new. |
| `3` | Setup/analysis/evidence failure: compiler or Cargo failed, worker receipts are incomplete/corrupt, required outputs could not be prepared/read/written, or the run was stopped by `--run-timeout`. A stopped run reaches no verdict, which is neither a `0` nor a `1`. |

`3` exists because "your code does not compile" and "your code has a counterexample" send the
reader to different places, and both used to be reported as `1`.

The wrapper runs rustc itself, so it reports the status rustc returned: a crate the compiler rejects
fails `cargo check` exactly as it fails a normal build. Until this pass that status was discarded,
and a crate that did not compile came out as exit `0` with an empty `report.json` — the analysis had
found no assertions in a body rustc never built, and "no findings" was indistinguishable from
"nothing to check". One consequence is worth knowing about: because the analysis reads
`optimized_mir` for every function, it runs MIR passes that a plain `cargo check` may skip, so a
deny-by-default lint such as `arithmetic_overflow` or `unconditional_panic` can be raised here for
code `cargo check` accepts and `cargo build` rejects. The wrapper follows the build.

## Requiring explicit coverage (`--strict-coverage`)

The default gate permits `WARN` findings. That includes unsupported assertion kinds such as the
pointer null/alignment assertions produced by `vec!`; a successful default exit is therefore not an
all-clear. In release or security CI, use the strict option:

```bash
keller-verify keller verify --strict-coverage
```

`--strict-coverage` fails with exit `1` on any encountered unsupported or undecided assertion, even
if a finding is listed in a baseline. It does not prove absence of uninstrumented panic paths or
validate the analyzer's MIR-to-SMT translation; see [the model and support boundaries](RELEASE.md).
`--profile strict` is the name for this flag plus `--deny-warn`, and it is what a release or security
gate should write rather than the two flags separately: `keller verify --profile strict`.

## Requiring a definite answer (`--deny-warn`)

A verdict is one of three things: `[FAIL]` is a counterexample, `[PASS]` is a proof, and `[WARN]`
is neither — the obligation was never decided. By default only `FAIL` fails the build, so a `WARN`
is written to `report.json` and then ignored by CI.

`--deny-warn` closes that gap:

```bash
cd /path/to/your-rust-project
/path/to/keller-verify/target/debug/keller-verify keller verify --deny-warn
```

With the flag, any `WARN` also makes the command exit `1`, so a pipeline can require an answer for
every assertion instead of only catching the ones that were refuted. The message names which half
of the contract was broken, because an undecided verdict and a counterexample need different
fixes:

```text
[Keller-Pipeline] Verification FAILED: report.json contains 1 FAIL event(s). See report.json in the working directory.
[Keller-Pipeline] Verification FAILED: 2 obligation(s) are undecided (WARN) and --deny-warn was given. See report.json in the working directory.
```

Two things produce a `WARN`:

- **The solver did not decide.** Z3 answered `unknown`, for example on nonlinear arithmetic it could
  not finish. `--solver-timeout <MS>` makes this bounded instead of open-ended, and the budget is per
  obligation and **on by default** (30000 ms), so running out is an undecided verdict rather than a
  hang. `--solver-timeout 0` removes the limit for a caller who wants that. What a *solver* budget
  cannot bound is a run that stops making progress at all, which is what the wall-clock cap
  `--run-timeout <SECONDS>` is for (off by default): a run it stops reaches no verdict and says so.
- **The encoder could not express the obligation.** Such a finding reads
  `"details": "Not modelled: …"`. This is deliberately a `WARN` rather than silence: an obligation
  the tool did not check is not an obligation that holds, and a report that omits it reads as a
  clean bill of health.

> **On this repository the flag is exercised by a fixture, not only by unit tests.** `tests/undecided`
> carries an obligation Z3 does not decide: reaching its guarded increment needs `a * b == 2^61 - 1`
> with both factors above 1, which holds for no pair because 2^61 - 1 is prime — a fact about
> factorisation rather than about that arithmetic. The fixture runner runs that crate twice, once on
> each side of the contract: exit `0` while the `WARN` is tolerated, and exit `1` under `--deny-warn`
> with the gate naming the half that failed. Every other fixture's obligations are decided, and the
> policy itself is covered by the unit tests in `src/report.rs`.

## What a failure tells you

When Z3 says SAT, the tool asks it for the model and reports the concrete input that triggers the
assertion, so a FAIL names an input rather than an unknown exploit path:

```json
{
  "crate": "overflow_fail",
  "function": "add",
  "block": "bb0",
  "status": "FAIL",
  "details": "Arithmetic Overflow Detection (Add) -> Exploit path found! [a (_1) = -2147483648, b (_2) = -1]",
  "witness": [
    { "input": "a (_1)", "symbol": "overflow_l", "value": "-2147483648" },
    { "input": "b (_2)", "symbol": "overflow_r", "value": "-1" }
  ]
}
```

- `input` is the source operand (the local's name and its MIR local, or the constant).
- `symbol` is the SMT variable the value was read from, which is what the emitted obligation uses.
- `value` is an exact decimal string, so a 128-bit value survives; it is read with Z3's own
  numeral printer, not a 64-bit conversion. Every entry is either an operand of the failure or a
  value the obligation pins, so a witness reads as evidence rather than as whatever the solver
  happened to pick; a guard's own symbols are not listed, and the guard itself is in the
  obligation's `; reachable:` comment.

The same values go into `details`, so the console line and the report agree.

A failure the constant-folding pass found — both operands are constants, so Z3 was never asked —
carries the same structured witness, under the symbol names its ground obligation declares:

```json
{
  "crate": "fold_fail",
  "function": "folded_add",
  "block": "bb0",
  "status": "FAIL",
  "kind": "overflow",
  "details": "Arithmetic Overflow: 200 Add 100 [_1 = 200, _2 = 100] -> Counterexample found by constant folding!",
  "witness": [
    { "input": "_1", "symbol": "folded_l", "value": "200" },
    { "input": "_2", "symbol": "folded_r", "value": "100" }
  ]
}
```

The `details` line ends with `-> Counterexample found by constant folding!` rather than the
`-> Exploit path found!` a solver-decided failure carries, so the report says which of the two
paths reached the verdict. A `u128` folded operand is printed in its own domain (`u128::MAX`, never the
`-1` the constant path carries it as), because a report that named an input the program cannot
contain would be worse than no report.

A FAIL also writes `obligations/<crate>/<function>_<block>.witness.smt2`: the same formula with the
reported values asserted as equalities — the failing operands, and the state values the obligation
pins. Re-discharging it must answer `sat`; if it answers `unsat`,
the reported witness does not actually fail the assertion — which is what the fixture suite checks
on every pinned witness. The main obligation carries a `; reachable:` comment naming the guards that
lead to the assertion, and those guards are part of the goal, so the file states the precondition
rather than only the failure. A constant-folded obligation is the exception: its ground instance
names only the folded operands, so it carries no `; reachable:` line. Reachability is decided before
the finding is recorded — a folded failure no input can reach is not reported at all — so the
precondition is not part of that file.

## Rolling the gate out over an existing codebase

A verifier that only offers "clean or failing" cannot be switched on for a codebase that already
has findings. The baseline is that on-switch:

```bash
# 1. Record what is already there. This is not a verdict, so it exits 0.
keller verify --write-baseline .keller-baseline.json

# 2. Gate on new findings only. The recorded ones are still reported, and still
#    counted, but they do not fail the build.
keller verify --baseline .keller-baseline.json

# 3. Shrink the file as the old findings get fixed, and drop it once it is empty.
keller verify
```

The file is small and reviewable, and is meant to be committed:

```json
{
  "keller_baseline": 1,
  "findings": [
    { "crate": "engine", "function": "encode", "block": "bb7", "status": "FAIL" }
  ]
}
```

Matching uses only those four fields. A reworded `details`, a new witness, or a renamed `kind`
still counts as the same obligation — otherwise the first unrelated edit would resurface every known
finding, which is exactly when a baseline matters most.

Two deliberate rules:

- **Findings stay in `report.json`.** The baseline says what the gate should stop complaining
  about, not what the record should forget.
- **A baseline that cannot be read fails the run** (exit `2`): a bad path, a file that is not a
  baseline, or a version this build does not know. Falling back to "no baseline" would turn a typo
  into a wall of long-known findings reported as brand new.

## GitHub code scanning

`--sarif` writes a SARIF 2.1.0 log of the findings the gate acted on, so a baselined problem is not
annotated as if this change introduced it. Each result carries a rule id (`keller/overflow`,
`keller/divzero`, …), a level (`error` for a `FAIL`, `warning` for a `WARN`), the function as a
logical location, a physical location when the compiler gave a file and line, and a
`partialFingerprints["keller/v1"]` of `crate::function::block` so a platform can recognise the same
finding across runs.

```yaml
- run: keller verify --sarif keller.sarif

- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: keller.sarif
```

The uploading job needs `permissions: security-events: write`.

## Guards are part of the question

The verifier threads a symbolic value through the control-flow graph and folds every
branch's guard into the condition required to reach the assertion, so an obligation is
only checked for states that can actually occur. Consequences worth knowing:

- Arithmetic a comparison already rules out is **proved**, not flagged. `fn f(x: u8) -> u8
  { if x < 100 { x + 1 } else { 0 } }` is UNSAT, and does not appear in `report.json`.
- A guard that is *too weak* stays a FAIL, with a witness that respects it: for
  `if x < 200 { x + 100 }`, the reported `x = 192` is inside the guard, where the older,
  path-free analysis reported `x = 255` — a value the guard makes impossible.
- A value merged from two branches is kept as "one of these" (`phi` in the emitted file),
  never narrowed to one branch's value, so a merge cannot hide a failure.
- A shift is checked on its **amount**: `a << b` panics when `b` is negative or at least the width
  of `a`. A value that a valid shift truncates out of range is not a panic (`200u8 << 1` is `144`),
  and is not reported as one. `tests/shift_amount` pins both halves.
- A loop is unrolled to its back edge, so a loop body is checked under the loop's own guards:
  the counter increment of `while i < steps { i += 1 }` is **proved**, because `i < steps` with a
  `u8` `steps` leaves `i <= 254`. Past a bounded number of trips the loop-carried values are
  widened to their type's range instead of being pinned to the trip that was modelled, so what a
  loop body is checked with is a state free within its types — the guards, not the values, are
  what carry a proof across a loop. `tests/loop_fail` pins both halves: the counter increment
  is proved, the accumulator's reachable overflow is still reported. Past the bound the body's
  *condition* is widened as well as its values, so a loop inside a loop converges too: `tests/nested_loop`
  pins both of its guarded increments proving and only the accumulator, which no guard bounds, staying a
  counterexample.
- If a body's dataflow cannot reach a fixpoint inside its round budget, the analysis falls back to the
  path-insensitive state — sound, but it costs that function every guard at once. That is **reported**
  rather than left to show up as a false positive: the function is recorded as a `WARN` with
  `"kind": "path-insensitive"`, naming the budget it hit and saying that a `FAIL` in it may be a false
  positive while a `PASS` there is still a proof. Under `--deny-warn` it fails the build like any other
  `WARN`. `KELLER_MAX_ROUNDS` overrides the budget (six rounds by default): raising it gives a shape the
  cap cut short more room, and lowering it to `1` reproduces the record on demand, which is how
  `tests/nested_loop` pins it.
- `bool` inputs are reported as `true`/`false` in the witness and pinned into the
  `.witness.smt2` file as `1`/`0`, which is what the formula itself encodes.

## Why `& ` before the path?

In **PowerShell**, when you use a quoted executable path, you must prefix it with `& `:

```powershell
& "C:\path\to\keller-verify\target\debug\keller-verify.exe" keller verify
```

In **cmd.exe** you can just write the quoted path. On Linux and macOS no prefix is needed.

## One-time setup

For the tested clean-machine build/install workflow, use [RELEASE.md](RELEASE.md) and
`scripts/install-linux.sh` on the supported Linux baseline. The manual build below is for
contributors and environments outside that support promise.


If you need to rebuild the verifier binary:

```bash
cd /path/to/Formal Verification Platform
cargo build --release
```

`build.rs` finds Z3 from `Z3_HOME`, then a project-local install under `.z3/`, then the
per-platform default, and prints a warning naming what it expected if none of them looks like a Z3
root (`<root>/bin` with `libz3.*`, and `<root>/include/z3.h`). The rule itself lives in
`src/host_env.rs`, which the test runners read too, so the Z3 the suite re-discharges with is the Z3
the verifier was linked against.